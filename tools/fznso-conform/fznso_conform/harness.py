"""Running a case against a solver, and reading back every solution."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import minizinc

from fznso_conform.case import Case

#: The repository this tool lives in.
ROOT = Path(__file__).resolve().parents[3]

#: The MiniZinc library and build this tool tests against. Both are a
#: development checkout by default, and either can name an installed MiniZinc
#: instead.
STDLIB = Path(os.environ.get("MZN_STDLIB_DIR", ROOT / "minizinc_impl/libminizinc/share/minizinc"))
BUILD = Path(os.environ.get("FZNSO_MZN_BUILD", ROOT / "minizinc_impl/build-rel/Release"))


#: What the library says when it has no way to give the solver this constraint:
#: the solver does not declare it and no decomposition is available. A case that
#: hits one of these is not applicable to that solver.
UNSUPPORTED = ("is not supported", "neither implements the constraint nor decomposes it")


class Unsupported(Exception):
    """This solver has no way to express the case at all."""


@dataclass
class Outcome:
    """What a solver answered, and whether it was right."""

    case: Case
    solver: str
    found: set[tuple[Any, ...]]
    expected: set[tuple[Any, ...]]
    declared: bool
    enumerated: str

    @property
    def ok(self) -> bool:
        return self.found == self.expected

    def report(self) -> str:
        missing = sorted(self.expected - self.found, key=repr)
        extra = sorted(self.found - self.expected, key=repr)
        lines = [
            f"{self.solver} answers {self.case.name} wrongly",
            f"  the solver {'declares' if self.declared else 'decomposes'} it,"
            f" solutions enumerated by {self.enumerated}",
            f"  {len(self.expected)} solutions expected, {len(self.found)} found",
        ]
        if missing:
            lines.append(f"  missing: {missing[:6]}")
        if extra:
            lines.append(f"  wrong:   {extra[:6]}")
        return "\n".join(lines)


class Harness:
    """Drives one solver configuration."""

    def __init__(
        self,
        solver_id: str,
        solver_path: Path | None = None,
        enumerate_with_nogoods: bool = False,
    ):
        os.environ["MZN_STDLIB_DIR"] = str(STDLIB)
        if solver_path is not None:
            os.environ["MZN_SOLVER_PATH"] = str(solver_path)
        if minizinc.default_driver is None or minizinc.default_driver.executable.parent != BUILD:
            minizinc.default_driver = minizinc.Driver.find([str(BUILD)])
        self.solver_id = solver_id
        self.solver = minizinc.Solver.lookup(solver_id)
        #: Not every solver takes `-a`; those are enumerated with nogoods
        #: instead, which is the same thing an incremental client would do.
        #:
        #: `enumerate_with_nogoods` asks for the nogood loop even from a solver
        #: that takes `-a`. Gurobi answers `-a` from its solution pool, which
        #: proves no further solution ties with the ones it holds, and for two
        #: of the cases here that proof does not finish. The same solver answers
        #: the loop in seconds, because each nogood is a row that cuts the
        #: region it excludes.
        self.all_solutions = "-a" in (self.solver.stdFlags or []) and not enumerate_with_nogoods

    @staticmethod
    def registry_dir() -> Path:
        """Where the registry lives, overridable for a checkout elsewhere."""
        return Path(os.environ.get("FZNSO_REGISTRY", ROOT / "docs/registry"))

    def library(self) -> Path:
        """The solver library this configuration names."""
        return Path(self.solver.executable)

    def declares(self, case: Case) -> bool:
        """Whether this solver posts the constraint itself.

        A declared constraint reaches the solver under its registry name, so it
        appears in the FlatZinc the compiler produces; a decomposed one does
        not. Compiling the case answers this for the constraint and the
        argument types the case actually uses, which a capability list alone
        would not.
        """
        with tempfile.TemporaryDirectory() as tmp:
            model = Path(tmp) / "case.mzn"
            model.write_text(case.model())
            out = subprocess.run(
                [str(BUILD / "minizinc"), "--solver", self.solver_id, "-c",
                 "--fzn-format", "json", "--no-output-ozn", "-o", str(Path(tmp) / "case.json"),
                 str(model)],
                capture_output=True, text=True, env=os.environ,
            )
            if out.returncode != 0:
                raise AssertionError(f"{self.solver_id} cannot compile {case.name}:\n{out.stderr}")
            flat = json.loads((Path(tmp) / "case.json").read_text())
            return any(c["id"] == f"fzn_{case.ident}" for c in flat["constraints"])

    def run(self, case: Case, time_limit_ms: int = 60_000) -> Outcome:
        try:
            found = (
                self._all_solutions(case, time_limit_ms)
                if self.all_solutions
                else self._nogood_loop(case, time_limit_ms)
            )
        except Exception as error:
            message = str(error)
            if any(marker in message for marker in UNSUPPORTED):
                raise Unsupported(message.strip().splitlines()[0]) from error
            raise
        return Outcome(
            case=case,
            solver=self.solver_id,
            found=found,
            expected=case.expected(),
            declared=self.declares(case),
            enumerated="the solver" if self.all_solutions else "nogoods",
        )

    def _instance(self, case: Case, extra: str = "") -> minizinc.Instance:
        model = minizinc.Model()
        model.add_string(case.model() + extra)
        return minizinc.Instance(self.solver, model)

    def _read(self, solution: Any, case: Case) -> tuple[Any, ...]:
        return tuple(case.key(getattr(solution, d.name)) for d in case.decisions)

    def _all_solutions(self, case: Case, time_limit_ms: int) -> set[tuple[Any, ...]]:
        import datetime

        result = self._instance(case).solve(
            all_solutions=True, time_limit=datetime.timedelta(milliseconds=time_limit_ms)
        )
        if result.status == minizinc.Status.UNSATISFIABLE:
            return set()
        # Only `ALL_SOLUTIONS` says the search finished. A solver that hits the
        # time limit reports `SATISFIED` with the solutions it reached, and
        # comparing that against the full set reads as a wrong answer.
        assert result.status == minizinc.Status.ALL_SOLUTIONS, (
            f"{self.solver_id} gave {result.status} for {case.name}:"
            f" the enumeration did not finish within {time_limit_ms} ms"
        )
        solutions = result.solution if isinstance(result.solution, list) else [result.solution]
        return {self._read(s, case) for s in solutions}

    def _nogood_loop(self, case: Case, time_limit_ms: int) -> set[tuple[Any, ...]]:
        """Enumerate by excluding each solution as it is found.

        This is the sequence an incremental client performs: solve, add a
        constraint ruling out the answer, solve again. The compiler is not yet
        incremental, so each round recompiles; the test is about the answers,
        not the cost.
        """
        import datetime

        found: set[tuple[Any, ...]] = set()
        nogoods = ""
        while True:
            result = self._instance(case, nogoods).solve(
                time_limit=datetime.timedelta(milliseconds=time_limit_ms)
            )
            if result.status == minizinc.Status.UNSATISFIABLE:
                return found
            assert result.status.has_solution(), (
                f"{self.solver_id} gave {result.status} for {case.name}"
            )
            values = self._read(result.solution, case)
            if values in found:
                raise AssertionError(f"{self.solver_id} repeated a solution for {case.name}")
            found.add(values)
            terms = " /\\ ".join(
                f"{d.name} = {self._literal(getattr(result.solution, d.name))}"
                for d in case.decisions
            )
            nogoods += f"constraint not ({terms});\n"

    @staticmethod
    def _literal(value: Any) -> str:
        if isinstance(value, bool):
            return "true" if value else "false"
        if isinstance(value, (set, frozenset)):
            return "{" + ", ".join(str(v) for v in sorted(value)) + "}"
        if isinstance(value, list):
            return "[" + ", ".join(Harness._literal(v) for v in value) + "]"
        return str(value)

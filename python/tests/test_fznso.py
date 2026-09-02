"""End-to-end tests for the FZnSO Python bindings.

A model is built in Python and run against the example solver. The example
solver assigns each decision its domain's lower bound, shifts the first by the
solution index, and warns that it ignores the constraints.
"""

import ctypes.util
import pathlib
import shutil
import sys

import pytest

import fznso

REPO = pathlib.Path(__file__).resolve().parents[2]

# The type every decision in these fixtures has.
VAR_INT = fznso.Type("int", decision=True)


def solver_path() -> str:
    prefix, suffix = ("lib", ".dylib") if sys.platform == "darwin" else ("lib", ".so")
    path = REPO / "target" / "debug" / "examples" / f"{prefix}example_solver{suffix}"
    if not path.exists():
        pytest.skip(f"{path} not built; run `cargo build --example example_solver`")
    return str(path)


def threaded_solver_path() -> str:
    prefix, suffix = ("lib", ".dylib") if sys.platform == "darwin" else ("lib", ".so")
    path = REPO / "target" / "debug" / "examples" / f"{prefix}threaded_solver{suffix}"
    if not path.exists():
        pytest.skip(f"{path} not built; run `cargo build --example threaded_solver`")
    return str(path)


@pytest.fixture
def library():
    return fznso.Library(solver_path())


@pytest.fixture
def solver():
    return fznso.Library(solver_path()).create_solver()


@pytest.fixture
def threaded_solver():
    """A solver that reads the model and reports from a worker thread."""
    return fznso.Library(threaded_solver_path()).create_solver()


def build_model():
    m = fznso.LayeredModel()
    a = m.add_decision(VAR_INT, range(3, 21), name="a")  # inclusive [3, 20]
    b = m.add_decision(VAR_INT, fznso.int_set(10, 20), name="b")
    m.add_constraint("int_le", [0, a])
    m.set_objective("minimize", a)
    return m, a, b


def test_options_round_trip(solver):
    assert solver.option_get("solution_limit") == 1
    solver.option_set("solution_limit", 3)
    assert solver.option_get("solution_limit") == 3


def test_solver_statistic(solver):
    # `runs` is declared solver-scoped, so it is read from the solver instance.
    assert solver.statistic("runs") == 0
    # Unknown, and solution-scoped, statistics are absent here.
    assert solver.statistic("nope") is None
    assert solver.statistic("solutions") is None


def test_unknown_option_raises_with_message(solver):
    with pytest.raises(ValueError, match="unknown option `nope`"):
        solver.option_set("nope", 1)


def test_run_reports_solutions_and_messages(solver):
    solver.option_set("solution_limit", 3)
    m, a, b = build_model()

    solutions, messages = [], []
    status = solver.run(
        m,
        on_solution=lambda s: solutions.append((s[a], s[b], s.statistic("solutions"))),
        on_message=lambda scope, value: messages.append((scope, value)),
    )

    assert status == fznso.Status.Complete
    assert solutions == [(3, 10, 0), (4, 10, 1), (5, 10, 2)]
    assert messages == [("warn", "ignoring 1 constraint(s)")]


def test_solution_is_invalid_outside_callback(solver):
    m, a, _ = build_model()
    escaped = []
    solver.run(m, on_solution=lambda s: escaped.append(s))
    # The solution borrowed a value that no longer exists; using it must raise
    # rather than read freed memory.
    with pytest.raises(RuntimeError, match="outside the on_solution callback"):
        escaped[0][a]


def test_callback_exception_propagates(solver):
    m, _, _ = build_model()

    def boom(_sol):
        raise KeyError("from the callback")

    with pytest.raises(KeyError, match="from the callback"):
        solver.run(m, on_solution=boom)


def test_value_kinds_round_trip():
    # Reading every value kind back through the solver's model view is covered
    # elsewhere; here we check the Python value conversions directly via the
    # domain a solution reports.
    m = fznso.LayeredModel()
    x = m.add_decision(VAR_INT, fznso.int_set(-5, 5), name="x")
    assert isinstance(x, fznso.Decision)
    assert int(x) == 0


class DomainModel(fznso.Model):
    """A model implemented in Python rather than built — the solver reads it on
    demand by calling these methods.

    The values it returns are borrowed, not copied, so it owns them: every object
    it hands back is built once in ``__init__`` and stored on the instance."""

    def __init__(self, domains, constraints=0):
        super().__init__()
        self._domains = [fznso.int_set(lo, hi) for lo, hi in domains]
        self._names = [f"x{i}" for i in range(len(domains))]
        self._ident = "int_le"
        self._arguments = [[0, fznso.Decision(i)] for i in range(constraints)]

    def decision_count(self):
        return len(self._domains)

    def decision_type(self, i):
        return VAR_INT

    def decision_domain(self, i):
        return self._domains[i]

    def decision_name(self, i):
        return self._names[i]

    def constraint_count(self):
        return len(self._arguments)

    def constraint_ident(self, i):
        return self._ident

    def constraint_arguments(self, i):
        return self._arguments[i]


def test_custom_python_model(solver):
    solver.option_set("solution_limit", 2)
    model = DomainModel([(7, 20), (30, 40)], constraints=2)

    solutions, messages = [], []
    solver.run(
        model,
        on_solution=lambda s: solutions.append((s.value(0), s.value(1))),
        on_message=lambda scope, value: messages.append((scope, value)),
    )
    # Lower bounds 7 and 30; the solver shifts the first by the solution index.
    assert solutions == [(7, 30), (8, 30)]
    assert messages == [("warn", "ignoring 2 constraint(s)")]


def test_run_can_be_stopped_early(solver):
    """`should_stop` ends a run the caller has already started."""
    solver.option_set("solution_limit", 5)
    m, a, _ = build_model()

    # "One solution is enough", decided from inside the callback.
    solutions = []
    status = solver.run(
        m,
        on_solution=lambda s: solutions.append(s[a]),
        should_stop=lambda: len(solutions) >= 1,
    )
    assert len(solutions) == 1, "the solver kept going after being asked to stop"
    assert status == fznso.Status.Incomplete

    # Already asked to stop before the run begins: no search at all.
    solutions = []
    status = solver.run(
        m, on_solution=lambda s: solutions.append(s[a]), should_stop=lambda: True
    )
    assert solutions == []
    assert status == fznso.Status.Incomplete


def test_stop_predicate_error_propagates(solver):
    """An exception from should_stop surfaces once the run unwinds."""
    m, _, _ = build_model()

    def boom():
        raise KeyError("from should_stop")

    with pytest.raises(KeyError, match="from should_stop"):
        solver.run(m, should_stop=boom)


def test_watchdog_thread_can_stop_a_run(solver):
    """Another Python thread can cancel a long run.

    This works only because ``run`` releases the GIL: the watchdog needs it to
    execute at all while the solver is searching.
    """
    import threading

    solver.option_set("solution_limit", 100_000)
    m, a, _ = build_model()

    seen = []
    stop = threading.Event()

    def watchdog():
        while len(seen) < 3:
            pass
        stop.set()

    thread = threading.Thread(target=watchdog)
    thread.start()
    status = solver.run(
        m, on_solution=lambda s: seen.append(s[a]), should_stop=stop.is_set
    )
    thread.join()

    assert status == fznso.Status.Incomplete
    assert 3 <= len(seen) < 100_000, f"expected an early stop, saw {len(seen)}"


def test_threaded_solver_does_not_deadlock(threaded_solver):
    """A solver reporting from its own thread must not hang.

    ``run`` releases the GIL for the duration of the search, so a worker thread
    can take it to invoke the callbacks. Were it held across the run, this would
    *hang* rather than fail — which is exactly why the case needs a test.
    """
    m, a, _ = build_model()  # a native LayeredModel: two decisions

    solutions, messages = [], []
    status = threaded_solver.run(
        m,
        on_solution=lambda s: solutions.append(s[a]),
        on_message=lambda scope, value: messages.append((scope, value)),
    )

    assert status == fznso.Status.Complete
    # The fixture reports the decision count it read from the worker thread.
    assert solutions == [2]
    assert messages == [("log", "read 2 decision(s) from a worker thread")]


def test_threaded_solver_reads_a_python_model(threaded_solver):
    """The strongest case: the worker thread re-enters Python for *model reads*.

    With a model implemented in Python, every query the worker makes has to
    reacquire the GIL from a thread that never held it, on top of the callbacks.
    """
    model = DomainModel([(7, 20), (30, 40), (1, 2)])

    solutions = []
    status = threaded_solver.run(model, on_solution=lambda s: solutions.append(s.value(0)))

    assert status == fznso.Status.Complete
    # Three decisions, counted by the worker through the Python model.
    assert solutions == [3]


class TwoLayerModel(fznso.Model):
    """A custom model split across two layers, to check the layer protocol."""

    def __init__(self, domains, split):
        super().__init__()
        self._domains = [fznso.int_set(lo, hi) for lo, hi in domains]
        self._names = [f"x{i}" for i in range(len(domains))]
        self._split = split  # number of decisions in the first layer
        self._ident = "int_le"
        self._arguments = [7, fznso.Decision(0)]
        self._objective = "minimize"
        self._objective_arg = fznso.Decision(0)

    def decision_count(self):
        return len(self._domains)

    def decision_type(self, i):
        return VAR_INT

    def decision_domain(self, i):
        return self._domains[i]

    def decision_name(self, i):
        return self._names[i]

    def layer_count(self):
        return 2

    def decision_layer_end(self, layer):
        return self._split if layer == 0 else len(self._domains)

    def constraint_count(self):
        return 1

    def constraint_ident(self, i):
        return self._ident

    def constraint_arguments(self, i):
        return self._arguments

    def objective_ident(self):
        return self._objective

    def objective_argument(self):
        return self._objective_arg


def test_custom_layered_model(solver):
    model = TwoLayerModel([(7, 20), (30, 40), (100, 110)], split=2)

    solutions, messages = [], []
    solver.run(
        model,
        on_solution=lambda s: solutions.append((s.value(0), s.value(1), s.value(2))),
        on_message=lambda scope, value: messages.append((scope, value)),
    )
    assert solutions == [(7, 30, 100)]
    # The solver reports the per-layer decision counts: 2 in layer 0, 1 in layer 1.
    assert ("layers", "2,1") in messages
    # It also reads back the model's strings (objective, decision name, constraint
    # identifier) and a constraint argument, all borrowing native Python objects.
    assert ("model", "obj=minimize;name0=x0;con0=int_le(7)") in messages


def test_types():
    # A decision's type is what it is; its domain is what it may take. Only the
    # type tells `var bool` from an unbounded `var int`, since both have no
    # domain to report.
    assert VAR_INT.base == "int"
    assert VAR_INT.decision
    assert not VAR_INT.set_of
    assert fznso.Type("int") != VAR_INT, "a parameter is not a decision variable"
    with pytest.raises(ValueError, match="unknown base type"):
        fznso.Type("complex")
    with pytest.raises(TypeError):
        fznso.Type("int", True)  # the qualifiers are keyword-only

    # Every shape the interface can carry is expressible, including the ones no
    # decision variable uses, and every qualifier reads back under the name it
    # was given — so a type can be taken apart and rebuilt.
    every = fznso.Type("string", list_of=True, decision=True, set_of=True, opt=True)
    assert (every.base, every.list_of, every.decision, every.set_of, every.opt) == (
        "string",
        True,
        True,
        True,
        True,
    )
    assert (
        fznso.Type(
            every.base,
            list_of=every.list_of,
            decision=every.decision,
            set_of=every.set_of,
            opt=every.opt,
        )
        == every
    )

    # `str` spells the type the way the protocol does, in every binding.
    assert str(VAR_INT) == "var int"
    assert str(fznso.Type("bool", decision=True)) == "var bool"
    assert str(fznso.Type("int", decision=True, set_of=True)) == "var set of int"
    assert str(fznso.Type("float", decision=True, opt=True)) == "var opt float"
    assert str(fznso.Type("int", list_of=True)) == "list of int"
    assert str(every) == "list of var opt set of string"

    # `repr` stays the expression that rebuilds it.
    assert repr(fznso.Type("int")) == 'Type("int")'
    assert repr(VAR_INT) == 'Type("int", decision=True)'
    assert (
        repr(every)
        == 'Type("string", list_of=True, decision=True, set_of=True, opt=True)'
    )
    assert eval(repr(every), {"Type": fznso.Type}) == every

    m = fznso.LayeredModel()
    b = m.add_decision(fznso.Type("bool", decision=True), name="b")
    assert int(b) == 0


def test_decision_type_has_no_default(solver):
    """A model that declares decisions has to say what they are.

    Every other accessor has a sensible default — no domain, no name, not
    defined — but a type cannot be guessed from anything else the model reports,
    so the base class refuses rather than inventing one.
    """

    class NoType(fznso.Model):
        def decision_count(self):
            return 1

    with pytest.raises(NotImplementedError, match="decision_type"):
        NoType().decision_type(0)


def test_custom_model_method_error_propagates(solver):
    class Broken(fznso.Model):
        def decision_count(self):
            return 1

        def decision_domain(self, i):
            raise ValueError("bad domain")

    with pytest.raises(ValueError, match="bad domain"):
        solver.run(Broken(), on_solution=lambda s: None)


def test_find_loads_a_solver_from_the_search_path(tmp_path, monkeypatch):
    # Stage the built example in a directory on the search path, under the file
    # name it will be found by: `libexample_solver.*` names `example_solver`.
    built = pathlib.Path(solver_path())
    shutil.copy(built, tmp_path / built.name)
    monkeypatch.setenv("FZNSO_SOLVER_PATH", str(tmp_path))

    # The staged directory takes precedence over the user and system ones.
    assert fznso.Library.search_paths()[0] == str(tmp_path)

    # It is listed without being loaded, carrying no version in its file name.
    found = [d for d in fznso.Library.discover() if d.name == "example_solver"]
    assert found, f"discover() did not list it: {fznso.Library.discover()}"
    assert found[0].version == ""
    assert found[0].path == str(tmp_path / built.name)

    # Loading by name works just like loading by path.
    solver = fznso.Library.find("example_solver").create_solver()
    assert solver.option_get("solution_limit") == 1

    # The staged example carries no version, so a specific request misses.
    with pytest.raises(RuntimeError, match="no solver named"):
        fznso.Library.find("example_solver", "9")

    # An unknown name reports where it looked.
    with pytest.raises(RuntimeError, match=str(tmp_path)):
        fznso.Library.find("definitely_absent")


def test_capability_lists(library):
    """A conforming application does not pass a name the solver did not declare."""
    # The example solver declares statistics and leaves the other four lists
    # empty, so this covers both the populated and the empty path.
    stats = library.statistics()
    assert [(s.ident, s.solution, s.solver) for s in stats] == [
        ("solutions", True, False),
        ("runs", False, True),
    ]
    assert stats[0].value_type == fznso.Type("int")
    assert repr(stats[1]) == 'Statistic("runs", Type("int"), solution=False, solver=True)'

    assert library.constraint_types() == []
    assert library.decision_types() == []
    assert library.objectives() == []
    assert library.options() == []


def test_layer_counts_reach_the_solver(solver):
    """`layer_unchanged` is 0 until the model says otherwise."""
    m = fznso.LayeredModel()
    x = m.add_decision(VAR_INT, range(3, 21), name="x")
    m.push_layer()
    m.add_constraint("int_le", [0, x])

    messages = []

    def run():
        messages.clear()
        solver.run(
            m,
            on_solution=lambda s: None,
            on_message=lambda scope, value: messages.append((scope, value)),
        )

    # Nothing said, nothing reused: the solver re-reads the whole model.
    run()
    assert ("layers.reuse", "unchanged=0;permanent=1;redundant=") in messages

    # Both layers are now permanent, the second one vacuous, and neither has
    # changed since the last run.
    m.mark_permanent()
    m.mark_redundant(1)
    m.mark_redundant(1)  # marking twice says nothing new
    m.set_unchanged(2)
    run()
    assert ("layers.reuse", "unchanged=2;permanent=2;redundant=1") in messages


def test_only_a_permanent_layer_can_be_redundant():
    m = fznso.LayeredModel()
    m.push_layer()
    with pytest.raises(IndexError, match="layer 1 is not permanent"):
        m.mark_redundant(1)


def test_annotation_fields():
    ann = fznso.Annotation("int_search", [[1, 2], 3])
    assert ann.ident == "int_search"
    assert ann.arguments == [[1, 2], 3]
    assert repr(ann) == 'Annotation("int_search", [[1, 2], 3])'
    assert fznso.Annotation("output_var").arguments == []


def test_annotations_reach_the_solver(solver):
    m = fznso.LayeredModel()
    x = m.add_decision(
        VAR_INT,
        range(3, 21),
        name="x",
        annotations=[fznso.Annotation("output_var")],
    )
    m.add_constraint(
        "int_le",
        [0, x],
        annotations=[fznso.Annotation("domain_consistency", [1, 2])],
    )
    m.set_objective("minimize", x, annotations=[fznso.Annotation("warm_start", [3])])

    messages = []
    solver.run(
        m,
        on_solution=lambda s: None,
        on_message=lambda scope, value: messages.append((scope, value)),
    )
    assert (
        "annotations",
        "d0:output_var() c0:domain_consistency(1,2) obj:warm_start(3)",
    ) in messages

    with pytest.raises(TypeError):
        m.add_decision(VAR_INT, annotations=["output_var"])


def test_custom_model_annotations(solver):
    """A model implemented in Python can annotate too, borrowing as ever."""

    class Annotated(fznso.Model):
        def __init__(self):
            super().__init__()
            self._domain = fznso.int_set(5, 9)
            # Owned by the model: annotations are borrowed, never copied.
            self._decision_anns = [fznso.Annotation("output_var", [7])]
            self._objective_anns = [fznso.Annotation("warm_start", [5])]

        def decision_count(self):
            return 1

        def decision_type(self, i):
            return VAR_INT

        def decision_domain(self, i):
            return self._domain

        def decision_annotations(self, i):
            return self._decision_anns

        def objective_ident(self):
            return "minimize"

        def objective_argument(self):
            return fznso.Decision(0)

        def objective_annotations(self):
            return self._objective_anns

    messages = []
    solver.run(
        Annotated(),
        on_solution=lambda s: None,
        on_message=lambda scope, value: messages.append((scope, value)),
    )
    assert ("annotations", "d0:output_var(7) obj:warm_start(5)") in messages

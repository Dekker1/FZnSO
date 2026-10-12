"""Every declared signature matches the registry's types.

The registry tests ask whether a solver answers correctly. This asks the
question one step earlier: does what it declares match what the registry
defines? The declarations are read straight through the ABI, so this needs no
compiler and no solve.
"""

from __future__ import annotations

from fznso_conform.harness import Harness
from fznso_conform.registry import Registry
from fznso_conform.signatures import check

#: Read once: the registry is the same for every solver.
REGISTRY = Registry.load(Harness.registry_dir())


def test_signatures(harness: Harness) -> None:
    library = harness.library()
    prefix = f"{library.name.removeprefix('lib').split('.', 1)[0]}_"
    reports = check(library, REGISTRY)
    failed = [r.spell(prefix) for r in reports if r.failed]
    assert not failed, f"{harness.solver_id} does not conform:\n" + "\n".join(failed)

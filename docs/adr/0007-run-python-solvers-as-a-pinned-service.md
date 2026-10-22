# 7. Run Python solvers as a pinned service

- **Status:** Proposed
- **Date:** 2026-10-22

## Context

Lean decides what is proved, but most of the work before a proof is
computation: exhaustive search of small cases, SAT and integer programs,
exact algebra, high-precision numerics, sequence fitting. The mature
solvers for that are Python libraries or have Python bindings: SymPy,
FLINT, PARI/GP, Z3, PySAT, OR-Tools CP-SAT, HiGHS, NetworkX, nauty,
mpmath. Jacobian, which the research notes recommend, is Python too.

Code the agent writes is untrusted, and a result that cannot be re-run is
not evidence. The options were:

- **Jacobian over MCP only.** Safe and curated, but it exposes operations
  rather than a language, so a bespoke search or a SAT encoding cannot be
  written.
- **Python in the agent's own container.** No isolation, and the solver
  environment would drift with the operator's machine.
- **A pinned, sandboxed service**, like the Lean service
  ([ADR 0003](0003-run-lean-as-a-pinned-service.md)).

## Decision

- Run Python as the `python` compose service, built from `docker/python/`,
  with one Python version and one solver environment locked with `uv`
  (`pyproject.toml`, `uv.lock`). Nothing is installed at run time.
- Run each request in a fresh process: no network, an unprivileged user, a
  read-only root and read-only project files, a private temporary
  directory, and limits on time, CPU, memory, files, and output.
- Run files, not source: a run names a file under the project's
  `python/work/` and a function in it, as the Lean tools name files
  ([`specs/toolkit/files/`](../specs/toolkit/files/README.md)).
- Record with every run the file and import digests, the arguments' digest,
  the seed, the Python version, and the lock's digest.
- A Python result can make a claim `checked` on a stated frame, never
  `formalised`; a witness is confirmed in Lean before it refutes anything.
- Leave SageMath out of the first lock; it needs its own image and ADR.

## Consequences

- The agent can write searches, encodings, and numerics in the language the
  solvers speak, without risk to the host.
- Results are re-runnable from their provenance, and the solver list an
  agent sees is exactly the lock.
- Another service and image to maintain; the lock moves only by a
  deliberate commit.
- Computation and proof stay separate in the ledger: Python finds and
  bounds, Lean proves.

# Shared design principles

Each principle below appears in both projects' written rules. The source line
is quoted or paraphrased so it can be checked against the pinned revisions in
[`README.md`](README.md).

## 1. Start from the question the user is asking, not from the engine's API

REA's `docs/tool-design.md` opens with "Design around the analyst question."
Tools are named for the action and the object the caller reasons about, and
fall into a small set of shapes:

| Shape | Use it for |
| --- | --- |
| `inspect` | Facts about one target, object, or resource |
| `search` / `list` | Finding candidates |
| `trace` | Relationships across evidence |
| `compare` | Explicitly paired artifacts or observations |
| `workflow` | A recurring outcome that needs composition |
| `observe` / `capture` | Runtime behavior |

Jacobian does the same with mathematics: an operation is stated as "given
input X, return Y such that P(X, Y)", where X, Y and P "make sense
independently of the algorithm, backend, benchmark, theorem, or surrounding
reasoning workflow"
(`docs/explanation/executable-mathematical-vocabulary.md`).

**Take-away:** the public contract describes the domain. Backends are
swappable implementation choices under it.

## 2. Find the right size for one operation

Jacobian calls this **semantic atomicity**: one operation establishes one
stable, reusable postcondition. Its doc gives a test with four questions:

1. Does it have a mathematical identity independent of Jacobian and of the
   motivating problem?
2. Does it establish exactly one postcondition: one map, predicate, invariant,
   construction, search result, witness or certificate?
3. Is it independent of strategy, so it does not prescribe the next call?
4. Would splitting it further only expose algorithm state or trivial
   projections?

| Candidate | Verdict |
| --- | --- |
| Smith normal form, polynomial factorization, maximum matching | Atomic |
| One DFS frontier update | Too low-level: exposes an implementation step |
| Extracting the first row of a matrix | Usually too low-level: cheap projection |
| "Solve this named conjecture" | Too high-level: encodes problem and strategy |

Algorithmic complexity is not the test. A hard algorithm can implement one
atomic operation, and a tiny helper can still be an implementation detail.

REA applies the same idea under a different name: "Prefer primitives; compose
workflows", and only add a workflow "when repeated use demonstrates a
multi-source outcome REA can join without hiding caller choices or
uncertainty."

**Take-away:** new public operations have to pass an admission test. Jacobian's
is written down in `docs/reference/public-operation-admission.md`; the first
step is to show why existing operations cannot already produce the result.

## 3. Never turn "unknown" into "no"

- REA: "Missing coverage is unknown, not absence." An empty result "proves
  absence only within the actually examined scope." Unresolved identity "must
  remain unknown rather than be treated as absent, foreign, or aligned."
- Jacobian: "Operational non-completion never establishes a mathematical
  conclusion." For a resource refusal, "returning `False` would incorrectly
  refute the claim; relabeling it as invalid mathematics would hide the
  actual limitation."

**Take-away:** results need three outcomes, not two: established, refuted,
and not established (with the reason). This matters most for agents, which
will happily read silence as a negative answer.

## 4. Return the smallest sufficient answer

REA's AGENTS.md ranks costs in a fixed order:

1. Agent tokens and context, "including schemas, extra calls and retries".
2. Analysis time.
3. CPU and memory.

It also warns that "fewer bytes alone do not establish fewer tokens or faster
analysis": a change must be measured on completed tasks.

Concrete mechanisms:

- REA advertises compact input schemas by default, with a full profile
  available behind `REA_MCP_INPUT_SCHEMA_PROFILE=full`.
- REA's `inspect_analysis_view` pages over results it already computed rather
  than recomputing or dumping them.
- Jacobian exposes 2 tools instead of 1,400 and lets `math.find` load one
  contract on demand.
- Jacobian returns plain values and omits provenance that the caller does not
  need (see the table in [`projects.md`](projects.md)).

## 5. Scale first, cap last

Jacobian's admission doc requires investigating how to run a motivating valid
workload before restricting it:

1. Identify the quantities that actually control work, intermediate growth and
   output size in every phase, including result assembly and serialization.
   "A linear kernel followed by a quadratic assembly pass is a quadratic
   operation."
2. Consider sparse, factored, modular or implicit representations before
   requiring an expanded result.
3. Try exact presolve (removing redundant constraints, fixed variables) before
   bounding the expanded form.
4. Look for a maintained specialist backend (FLINT, GMP, Arb) before writing a
   custom kernel.
5. Keep a restrictive ceiling only after all of that, and document what was
   tried. Calling a limit "conservative" does not count as an investigation.

REA has the matching rule: "Add limits only for real format, protocol,
authority, or measured resource constraints. An assumed agent budget is
insufficient justification for a limit."

## 6. Every resource has exactly one owner

REA spends about 15k lines on process supervision (`src/process/`) and a long
section of `docs/tool-design.md` on lifecycle rules. The key ones:

- Keep admission, readiness, active work, process exit and resource release
  as separate facts. A process exiting does not prove its output was fully
  read.
- Carry cancellation through result consumption and cleanup; check the cancel
  signal again after the last cleanup step before returning success.
- Release only resources you own. Attaching to a process does not mean you own
  it.
- When cleanup fails, keep the owner alive and retry; block new work until the
  cleanup resolves.

Jacobian's version is shorter because its kernel is stateless: "All mandatory
phases share the request's deadline and work accounting", and a subprocess
"needs a concrete isolation, killability, or fixed-toolchain reason."

## 7. Parse once, at the owning boundary

- REA: "Normalize once at the owning boundary using format-aware parsers."
  Consumers "must use that owner's parsed facts rather than reinterpret raw
  source."
- Jacobian: mathematical syntax has "a named, non-evaluating grammar". Caller
  strings "must never reach `sympify`, `parse_expr`, `eval`, `exec`, or
  generated evaluators." Exact scalars stay native inside the kernel and are
  encoded only at wire boundaries.

## 8. Fix the producer, not the symptom

REA's AGENTS.md: "Fix the producing representation or ownership rule before
adding downstream repairs." When fixing a bug, inspect the producing boundary
and its consumers for duplicated interpretation, do the related refactor in
the same change, and remove superseded code. "Passing regression tests alone
does not establish that the brittleness is resolved."

Jacobian applies it to its own contracts: it is pre-stable, so it repairs an
unsound contract "rather than preserving it through compatibility machinery".
Its changelog is full of breaking changes made deliberately, such as removing
`lean.check` and replacing three graph operations with one typed-map contract.

## 9. Bring your own heavy dependencies; never touch the user's machine silently

- REA's setup "must show its planned changes and require approval before
  writing files". It never installs or upgrades Node.js, npm, Homebrew, Java,
  Ghidra or IDA. Configuration changes are additive, idempotent and backed up.
- Jacobian's setup previews an explicit plan (`--dry-run`) and refuses `--yes`
  without explicit agent flags. It does not install Node.js, Python, `uv` or
  any agent.

## 10. Draw the scope boundary in writing

REA's CONTRIBUTING.md lists what it will never integrate: workflows to defeat
commercial packers or DRM, anti-debugging evasion, exploit automation. It also
has a **fail-closed integration** rule: if scope, authority, side effects or
the required verification are unresolved, the change does not merge.
Jacobian's equivalent is the catalog admission contract: a capability that
fails the admission test stays a private helper, however useful it looked in
one trace.

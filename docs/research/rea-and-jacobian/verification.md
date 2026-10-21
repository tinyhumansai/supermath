# Verification, evaluation and benchmarks

Both projects produce answers whose correctness is hard to see from the
outside: decompiler output and mathematical results. Most of their engineering
effort goes into proving those answers right, and into measuring whether
agents can actually use the tools.

## Test at the highest useful boundary

Both testing guides use the same order of preference:

1. End-to-end behavior through a real caller-visible path (CLI or MCP), with
   real components and no mocked boundaries.
2. Integration across schemas, APIs and producer-consumer seams.
3. Golden cases with realistic inputs and exact expected results.

A lower-level unit test stays only when it proves a distinct invariant,
admission rule or failure mode that a higher-level test cannot reach.
Jacobian: "Classify evidence by the boundaries it actually crosses; an
owner-local operation call is not an end-to-end test." REA: "simulated
providers do not establish real-provider behavior."

Both projects also **prune** tests. Recent Jacobian commits include "test:
prune redundant coverage across domains" and "retire the stale moved_from_math
duplicate test tree". Test count is not treated as a quality measure.

## Use an oracle that does not share the implementation

- REA's optimization skill: "Establish an oracle independent of the changed
  implementation." Source-built fixtures, independently mapped bytes, or a
  native producer. "Comparing two wrappers around the same faulty decoder does
  not establish correctness."
- Jacobian's admission rule: state the operation's **defining invariant** (an
  identity, a reconstruction, an exhaustive relation or an independent
  oracle) and test against it. When the postcondition covers a family (every
  factor, every root), the test has to cover the whole family, not just the
  cases the implementation happens to handle.

Example: a polynomial factorization is checked by multiplying the factors back
together and checking each factor's irreducibility, not by comparing against
the same backend's output.

## Adversarial boundary tests

Jacobian's testing strategy has an "Adversarial closure" section:

- Pair an expensive-case rejection with a **larger but structurally simple**
  accepted case, so "larger input" never silently means "more expensive".
- When widening an execution limit, test three cases: the smallest formerly
  rejected valid request, a simple request at the full boundary, and a request
  near the output or storage bound.
- Round-trip **the same newly accepted result** through serialization and a
  real consumer. A correct native result can still violate the published
  result schema.
- A resource refusal must stay a resource error at every layer. Returning
  `false` would wrongly refute the claim.
- Assert stable error codes, not message wording. One PR converted 1,640
  message-text assertions into error-code assertions.
- A review finding is closed only when the behavior is fixed at the public
  boundary and a regression test proves it cannot recur.

REA has a parallel "Lifecycle regressions" section: late handshakes,
cancellation during final cleanup, mixed child outcomes, and cleanup retry.
"Active-execution cancellation alone does not cover finalization."

## Real-engine CI lanes

REA runs a separate CI workflow per real toolchain: `real-hopper.yml`,
`real-ghidra-windows.yml`, `real-browser.yml`, `real-cutter.yml`,
`real-go-binary.yml`, `real-packet-capture.yml`, and many more. Changelog
entries state exactly what was verified, for example: "Recorded Ghidra 12.1.4
verification uses `WindowsResourceReference` disabled ... ARM64EC/ARM64X and
Windows-on-Arm hosts remain unverified or unsupported."

The rule: a claim about a real engine requires that engine's lane. Unverified
scope is written down, not implied.

## Mutation testing

REA runs Stryker mutation testing (`stryker.config.mjs`,
`.github/workflows/mutation.yml`) on its core reverse-engineering tools. A
surviving mutant means a test suite that passes even when the code is wrong.

## Benchmarks with clean-room verifiers (Jacobian)

Jacobian's `benchmarks/` holds Harbor tasks in five datasets:

| Dataset | Purpose |
| --- | --- |
| `mathematical-benchmarks-v1` | Fixed tasks with hidden runtime details |
| `symbolic-coordination-v1` | Exact polynomial-map coordination pilot |
| `public-reproductions-v1` | Replay known public mathematical results |
| `conjecture-probes-v1` | Bounded, independently checked progress on conjectures |
| `examples-v1` | Tutorial and smoke workflows |

Design rules worth copying:

- The agent sees only `instruction.md` and `environment/`. Reference solutions
  and verifier code live in separate directories it cannot read.
- Verifiers **accept any valid answer**, not just the reference one. The
  constant-weight-code task "does not trust the public file or compare against
  it. It accepts any 2992 represented words satisfying the weight and distance
  predicates."
- Tasks test **calibration**, not just correctness. The BSD task asks for a
  checkable infinite-order rational-point certificate "while refusing to infer
  analytic rank or Birch–Swinnerton-Dyer". Some tasks are audits: does a
  proposed Lean formalization actually mean the same thing as the informal
  statement?
- An unsolved conjecture is not a usable oracle. Probes extract "frozen,
  independently checkable finite obligations" from it.
- Each task records a trust level, such as `COMPUTED`, and states what was not
  assessed.
- Evaluation runs are locked by content address: dataset header, task
  digests, images, source tree and configuration.

## Measuring whether agents can use the tools

Jacobian splits tool usability into separate questions, each with its own
skill:

| Question | Skill | What a pass shows |
| --- | --- | --- |
| Availability: does the client receive the tools and schemas? | `evaluate-mcp-tool-adoption` | The connection and catalog work |
| Discovery: can a natural phrase find the right operation? | `evaluate-mcp-tool-adoption` | Reachable without knowing IDs |
| Execution: does a known valid payload work? | `evaluate-mcp-tool-adoption` | Schema and dispatch work |
| Selection: does an uncued agent choose the tool? | `evaluate-mcp-tool-adoption` | Routing and salience |
| Friction: once chosen, can the agent form a valid request and recover from errors? | `audit-mcp-tool-friction` | The public contract is usable |

Rules from these skills:

- Never score "the agent called the tool" as success. Final correctness,
  witness validity and truthful claims are the outcomes.
- Freeze the model, client, server revision, image, task and budget across
  compared arms.
- One malformed call is an agent slip, not a product defect, unless the
  contract hid the rule. A successful fallback to hand-written Python is not
  evidence that the tool is easy to use.

REA has the equivalent in `src/evaluation/` (`CodexAgentEval.ts`,
`KnownAnswerEvaluation.ts`) and a rule in `docs/tool-design.md`: "Schema
validity and keyword heuristics do not establish successful tool selection."

## Learning from agent runs

Jacobian has two skills that turn agent work into product improvements:

- `learn-from-math-agent-trajectories` reviews one completed or paused
  investigation and attributes each problem to a category: discovery,
  execution friction, representation, contract or scale defect, missing
  operation, or the caller's own reasoning. It explicitly does not resume
  solving the problem.
- `decompose-mathematical-solution-corpora` reads a body of solutions (papers,
  Lean developments, solver encodings, certificate bundles), clusters them by
  mathematical technique, and extracts candidate operations. Each candidate
  still has to pass the admission contract.

A recurring warning in both: hand-written code in a trace is a lead, not proof
that an operation is missing.

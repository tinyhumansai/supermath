# Agent-driven development at high volume

REA reached roughly 6,100 PRs in six months and Jacobian roughly 4,500 in
three. Recent PRs are co-authored by Cursor Agent alongside the maintainer.
One person cannot hand-write that much code; what they can do is write the
rules agents follow and review against those rules. These are the mechanisms
that make that work.

## AGENTS.md is the control system

Both AGENTS.md files are short relative to the codebase (REA ~120 lines,
Jacobian ~110) but dense. They do not describe the code. They state
invariants and point to the document that owns each topic. Jacobian's has a
routing table:

| Change | Authority |
| --- | --- |
| New public operation | Public operation admission |
| Backend adapter or child worker | Backend contract |
| MCP projection or transport | Tool reference |
| Validation and error assertions | Testing strategy |

Patterns in how the rules are written:

- **Invariants, not procedures.** "Keep the kernel stateless." "Missing
  coverage is unknown, not absence." An agent can check its diff against an
  invariant; it cannot reliably follow a long procedure.
- **Name the failure it prevents.** "A PID and executable path alone do not
  establish ownership after exit or reuse." Each rule carries the bug that
  motivated it.
- **Forbid the tempting shortcut explicitly.** "Caller strings must never
  reach `sympify`, `parse_expr`, `eval`, `exec`." "Calling a limit
  'conservative' does not discharge that investigation."
- **Say what does not count as evidence.** "Passing regression tests alone does
  not establish that the brittleness is resolved." "Simulated providers do not
  establish real-provider behavior." This is aimed directly at agents, which
  tend to report success once tests pass.

## Project skills encode recurring workflows

Both repositories ship skills under `.agents/skills/`:

- REA: `rea-tool-design`, `rea-analysis-optimization`, `rea-ci-tests-audit`,
  `rea-changelog-update`, plus the user-facing `reverse-engineer-anything`
  skill that teaches an agent how to use REA itself.
- Jacobian: `audit-public-operation-contracts`,
  `audit-mathematical-vocabulary`, `audit-mcp-tool-friction`,
  `evaluate-mcp-tool-adoption`, `learn-from-math-agent-trajectories`,
  `decompose-mathematical-solution-corpora`, `harbor-benchmarks`,
  `verifier-evaluations`, `recent-conjecture-evaluations`, and the user-facing
  `jacobian-math`.

Each skill is scoped narrowly and routes to its neighbors ("use X for Y; this
skill owns only Z"). Several state what they must not do, for example
"do not resume solving the problem" or "never enable it in an agent-under-test
configuration".

The user-facing skill is part of the product. REA's skill tells an agent which
tool to start with for each kind of target, when not to use REA at all
("ordinary analysis of a complete source repository"), and how to diagnose a
broken setup without repeating it blindly.

## PR template as a checklist for reasoning

REA's PR template asks for: the concrete problem and expected behavior, the
change and its scope, what is out of scope, the semantic owner and earliest
changed stage, the effect on the CLI and MCP contract, the effect on Evidence,
and checks that actually ran. Example from PR #6024: "Out of scope: MCP
response and startup ceilings, JEB/IDA/`command_timeout_ms`, JavaScript heap
and semantic ceilings ..."

Rules around it:

- A human must review and own the final diff and declare it ready. Agents must
  leave the readiness boxes unchecked and never attest to human review.
- "Report only checks that actually ran." Distinguish source-derived
  conclusions, proposed checks, and unavailable coverage.
- Address every automated review finding or explain the disagreement.

## Fast, scoped validation

Both projects make the cheap check the default and the full check optional:

- REA: `npm run test:focused -- PATH`, `npm run check:fast`; the full
  `check:pr` is "optional for broad changes or CI diagnosis".
- Jacobian: `make test-focused LANE=... TESTS=...` during edits, then
  `make affected AFFECTED_BASE=origin/main`, which plans and prints exactly
  which lanes a diff affects before running them. Agents are told not to
  "reconstruct owner-wide or broad `pytest -n N` commands".

Jacobian also has explicit rules for parallel agents: isolated worktrees,
distinct branches, one writer per branch, check open PRs for the same
operation ID before pushing, and only the coordinator runs exhaustive
validation in a shared checkout.

## Changelog and release discipline

- Conventional Commit titles; breaking changes marked with `!`.
- release-please generates releases; REA's changelog thanks contributors per
  item and links every PR and issue.
- Each changelog entry states the verified scope, for example which engine
  version and host a feature was tested on, and what stays unverified.

## Distribution is treated as product work

- One command sets up the tool for many agent clients, with a dry-run plan and
  backups (REA supports about 15 clients).
- `doctor` commands diagnose registration problems separately from missing
  engines.
- REA's README is translated into 20 languages and links showcase case
  studies whose results can be checked. The DX-Ball showcase reconstructs a C
  function that passes 3,205 tests against the original x86 and reproduces
  all 63 compiled bytes.

## What this costs

The approach is not free, and we should not pretend otherwise:

- The rules are long and precise. Some passages in REA's `tool-design.md`
  are dense enough that a human reader struggles; they are written for agents
  that will apply them literally.
- The test suites are larger than the source and need active pruning.
- Breaking changes are frequent while the projects are pre-stable, which
  pushes migration cost onto users.
- A large share of recent commits are fixes to edge cases in lifecycle and
  error reporting. That is the price of the honesty rules: every new boundary
  needs its own unknown and failure handling.

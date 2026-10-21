# Implications for supermath

These are recommendations drawn from the other files in this folder, not
decisions. Anything we adopt should go through a spec in `docs/specs/` or an
ADR in `docs/adr/`, with a link back here.

supermath starts from the TinyBus Rust template: a wire-contract crate with
no behavior, and a module crate that implements it. That split already matches
the strongest pattern in both projects (an owned contract, separate from the
engines behind it), so most of what follows extends structure we already have.

## Adopt

### Write the operation admission test before the first operation

Jacobian's most transferable artifact is its admission contract
(see [`principles.md`](principles.md), section 2). Before supermath ships its
first real operation, write our version as a spec:

- State each operation as "for valid input X, return Y such that P(X, Y)".
- State the defining invariant that tests it (an identity, a reconstruction,
  an exhaustive relation or an independent oracle).
- Require evidence that existing operations cannot already produce the result.
- Reject operations that expose one algorithm step, and operations that bundle
  a whole strategy.

This maps cleanly onto the template: X and Y are payload types in the
contract crate; P and its invariant are the test in the module crate.

### Three-valued results from day one

Model "not established" as its own outcome, with a reason (deadline, work
limit, unsupported representation), and make it impossible to confuse with a
negative answer. In Rust this is natural: an enum such as
`Established(T) | Refuted(Witness) | NotEstablished(Reason)` in the contract
crate, rather than `Result<bool>`. Retrofitting this later means a breaking
change to every operation.

### Bound work before running it

Adopt Jacobian's "scale first, cap last" order: identify the quantities that
control work and output size, try better representations and maintained
backends, and only then add a limit, with the reason documented next to it.
Every operation's request should be admitted against a work budget before any
expensive call starts, and the refusal should be a typed error.

### Never evaluate caller strings

If supermath accepts mathematical expressions, parse them with a named,
non-evaluating grammar at the contract boundary. No backend should ever
receive raw caller text to interpret.

### Test against independent oracles

Our AGENTS.md already requires a failing test before every bug fix and 90%
line coverage. Add the two rules both projects rely on most:

- Every operation has a test against its defining invariant, not against the
  same backend's output.
- Boundary changes get the three-case adversarial set: the smallest formerly
  rejected input, a simple input at the full boundary, and an input near the
  output bound. Assert error variants, not message text.

### Plan for a small agent-facing surface

If supermath exposes many operations to agents, follow Jacobian rather than
REA: a discovery call and an execution call over a catalog, instead of one
tool per operation. Keep each operation's schema loadable on demand. Count the
cost of schemas in tokens, not just bytes.

### Measure agent use separately from correctness

When we have agents calling supermath, split evaluation the way Jacobian does:
availability, discovery, execution, selection, and friction after selection.
Never score "the agent called the tool" as success.

### Grow AGENTS.md by invariants

As supermath's rules grow, write them the way both projects do: invariants
rather than procedures, each with the failure it prevents, and an explicit
list of what does not count as evidence. Keep a routing table from kinds of
change to the document that owns them.

## Adapt

- **Provenance.** REA attaches full Evidence to every result because claims
  about binaries depend on their source. Jacobian omits it because results can
  be checked directly. supermath should decide per result type: exact results
  that a caller can verify cheaply need no provenance; numerical
  approximations, solver outputs with timeouts, and anything that depends on a
  backend version should carry at least the backend identity and the precision
  or completeness achieved.
- **Witnesses.** Return a witness when it is the operation's purpose, or when
  the result would otherwise be unverifiable (for example, a claimed
  satisfying assignment). Do not wrap every value in a certificate.
- **Lean.** Jacobian removed its Lean checker. If supermath wants formal
  verification, the lower-risk design is a separate module or MCP server for
  Lean, composed by the caller, rather than a Lean runtime inside the math
  kernel. This needs its own ADR.

## Avoid

- **Copying the scale.** Both projects reached hundreds of thousands of lines
  in months. That volume is a consequence of agent-driven development, not a
  goal. Our template's lint and coverage gates are the right starting point;
  add rules when a real failure shows they are needed.
- **Silent fallbacks.** Both projects refuse to fall back to a different
  engine without saying so. Do the same: if a preferred backend is missing,
  report it rather than switching quietly.
- **Tests as a count.** Both projects actively prune duplicate tests. Coverage
  thresholds are a floor, not evidence that the important invariant is
  tested.

## Open questions

1. Will supermath be called mainly by agents over MCP, by other TinyBus
   modules, or both? That decides how much weight to put on discovery and
   schema size.
2. Which backends will it wrap (for example FLINT, a SAT or SMT solver, a CAS),
   and which operations will it implement natively in Rust?
3. Does supermath need formal proof checking, or only computation with
   checkable witnesses?

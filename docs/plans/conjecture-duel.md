# Conjecture duel: implementation plan

- **Spec:** [`agent_settle`](../specs/toolkit/subagents/tools/agent_settle.md)
- **Decision:** [ADR 0009](../adr/0009-settle-conjectures-by-a-prover-refuter-duel.md)
- **Status:** Implemented, except the live test (task 9)

## Goal

Add `agent_settle` to `supermath-subagents`: a prover and a refuter run at
the same time on one pinned conjecture, in one workspace, with equal
budgets. They exchange verified posts, a monitor keeps them on task, and the
first verified win cancels the other side.

## Non-goals

- Formalising or aligning the statement. That is still the job of
  `agent_formalize` and `agent_review_alignment`.
- Moving budget between the sides, or deciding a stopping point from debts
  (`strategies/03-failure.md` §5).
- A CLI entry point. The top-level agent calls the tool.

## Tasks

Write each task's test before the code, and give each task its own commit.

1. **Turn control.** Add `child/control.rs` with `TurnControl`, a steer queue
   and a cancel flag that the host owns. Add `ChildRequest.control` and
   `ChildTurn.unapplied`. `OpenHumanChild::run` passes steers to
   `TurnSteering::steer` and cancellation to `TurnCancellation::cancel`, and
   maps `CoreError::TurnCancelled` to `OutOfBudget("cancelled")`.
   Tests: queueing, draining, cancel wake-up. Focus is enforced by the host
   from the belt journal, through steering and cancellation. It does not use
   a separate stop hook, so scripted runners exercise it too.
2. **Belt watcher.** Add `Belt::with_watcher`, so the host sees each journaled
   call as it happens. Test: the watcher sees each call once, in order.
3. **Duel children.** Add `ChildKind::DuelProver` and `DuelRefuter` with
   their primitives, prompts (`child/prompts.rs`), and agent ids. Add
   `Models::duel` (`SUPERMATH_DUEL_MODEL`) and `DuelSettings`
   (`SUPERMATH_DUEL_DRIFT_CALLS`, `_MAX_ROUNDS`, `_DIGEST_EVERY`). Tests:
   defaults, overrides, refusals.
4. **Write guard** (`duel/guard.rs`). Tests: in-area writes pass; the other
   side's area, `Statement.lean`, verdict files, and `..` are `FORBIDDEN`.
5. **Target and verdict** (`duel/target.rs`). Cover the names derived from
   a claim id, the `Statement.lean` and `Proved.lean`/`Refuted.lean` text,
   preconditions, and the two-step win check. Tests: each precondition is
   refused; a `sorry` proof is no win; a verdict file that fails is no win.
6. **Board, posts, and relay** (`duel/board.rs`, `duel/post.rs`). Cover
   `duel_post` checks per kind, `duel_board`, the digest's rate and cap,
   undelivered posts, and progress and drift tracking. Tests: every kind
   accepted and refused; wrong-side kinds; the post cap; digest framing;
   nudge, then cancel after three drifts.
7. **The duel** (`duel/mod.rs`). Cover two side drivers and a controller,
   with rounds, equal limits, the first win cancelling the other side,
   `inconsistent`, and the closing bound records. Tests use scripted sides:
   the refuter wins; the prover wins; both run out; both verify; a second
   round gets the host summary and undelivered posts.
8. **`agent_settle`** (`tools/settle.rs`). Cover arguments, admission at two
   sides' demand, the envelope, and wiring through `Subagents`, `Runners`,
   and the runner's pinned tool list. Tests: the contract checker, budget
   refusal, and the tool listing in `supermath-agent`.
9. **Live.** `live_duel_refutes_euler_polynomial`, gated by
   `SUPERMATH_LIVE_SUBAGENTS=1`.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
scripts/check-crate-deps.sh
.github/scripts/check-file-coverage.sh 90 coverage.json
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace
```

## Checklist

- [x] 1 turn control
- [x] 2 belt watcher
- [x] 3 duel children and settings
- [x] 4 write guard
- [x] 5 target and verdict
- [x] 6 board, posts, relay
- [x] 7 the duel
- [x] 8 `agent_settle` and wiring
- [ ] 9 live test

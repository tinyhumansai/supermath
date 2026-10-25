# rsi-ralph, as supermath runs it

Notes from reading and running [`tinyhumansai/rsi-ralph`](https://github.com/tinyhumansai/rsi-ralph)
at `8e44890` (v0.2.1, October 2026), for [`docs/specs/loops.md`](../specs/loops.md).
They record what the design relies on, so a later version of rsi-ralph can be
checked against them.

## What it is

A Rust workspace whose `rsi-ralph` binary (crate `rsi-ralph-cli`) runs a
*loop folder*: `rsi.json`, a `brief.md` template, a tinyhivemind package in
`hive/`, and the scorer and gate commands the manifest names
(`docs/specs/rsi-cli.md` there). It embeds OpenHuman as a library, so the
agents, sessions, providers and tool runtime live in that binary, not in
supermath.

One run:

1. **Baseline.** It validates the folder (`rsi-ralph check`), freezes a copy
   into the run store (`$TINYRSI_HOME`, default `~/.tinyrsi`), starts a spend
   gateway, and scores the incumbent.
2. **Epochs.** Each epoch renders the brief, gives every editing seat its own
   candidate checkout, and runs the hive until it is idle or `wall_secs`
   passes.
3. **Judging.** Each changed candidate goes through the surface check
   (`editable`/`frozen` prefixes), then the gates, then the scorers, then the
   decision. The best accepted candidate is promoted.
4. **Stop.** The run ends on `goal_reached`, `stalled` (no promotion for
   `patience` epochs), `budget`, `max_epochs`, `provider_refused` or `error`.

## Facts the loop design depends on

- **Seats work in their checkout.**
  - `rsi-ralph-swarm` builds one OpenHuman agent per seat with
    `action_dir` set to its candidate checkout (editing seats) or the
    incumbent (the rest; `src/swarm/seat.rs`).
  - Editing seats get `Access::full()`, the rest `Access::readonly()`.
  - The kickoff message names each candidate's path.
- **Executable tools run in the seat's directory with a cleared environment.**
  - tinyhivemind's `ProgramTool` runs `command ++ argv` in the seat's
    workspace (`cwd: workspace`, the default).
  - Its environment holds only `PATH`, `HOME`, `LANG`, the declared `env`
    references, and `THM_*` (`tinyhivemind-exec/src/run/command.rs`).
  - An `env` reference that does not resolve fails the call. So
    `supermath-mcp call` reads backend settings from `--env-file` rather
    than declaring them.
  - **A non-zero exit drops stdout.** The model then sees only `<tool> exited
    with status N: <stderr>` (`tinyhivemind-exec/src/run/mod.rs`). So
    `supermath-mcp call` exits 0 for every envelope.
- **Executable tools are not attached to seats by rsi-ralph `8e44890`.**
  - The hive language lowers each seat's `programs` (from `tools` and
    `toolpacks`), but `rsi-ralph-swarm` builds its own `AgentSpec` and never
    adds them.
  - tinyhumansai/rsi-ralph#75 attaches them through
    `OpenHumanHost::with_tools`.
  - Until it lands, a seat can reach the same commands only through the
    `shell` tool.
- **Package files are not runnable.** The swarm receives the package as a
  path → text map, and `load_package` keeps only root `*.json` and nested
  `*.md`. A program must be on `PATH` or absolute; `supermath-mcp` is either.
- **Folder subjects are copied, hashed and compared with build folders
  skipped.**
  - `.lake` is not among them at `8e44890`, so a Lean check's build output
    in a candidate counts as an edit outside the surface.
  - tinyhumansai/rsi-ralph#76 adds it.
  - The incumbent is copied into the run store and the loop folder is never
    written.
- **Scorers.**
  - A scorer command runs in the loop folder with `RSI_CANDIDATE`, `RSI_OUT`,
    `RSI_SEED` and the rest of the step contract.
  - It writes `$RSI_OUT/result.json` (`score`, `failures`, `metrics`,
    `private`) or prints `METRIC <name>=<float>`.
  - `failures` and `metrics` reach the next brief.
  - A `seeds` decision needs at least two distinct seeds. A deterministic
    score has zero variance, so any real improvement is accepted.
- **Models and keys.**
  - Every model call goes through the run's gateway, which forwards to
    OpenRouter, or to `TINYRSI_UPSTREAM` when set.
  - A model needs a price ceiling in `budget.prices`; `provider: "*"` sends
    no OpenRouter routing.
  - `keys.openrouter` names the variable that holds the upstream key, so
    `"SURPLUS_API_KEY"` with `TINYRSI_UPSTREAM` set to Surplus's `/v1` base
    sends every call to Surplus Intelligence.
- **No agent memory.** Seat recall is switched off (`memory.recall.enabled =
  false`). What crosses epochs is the brief (history, failures), the subject,
  and whatever the seats write themselves: here the ledger in CortexDB and
  the subject's `notes/`.
- **One swarm per process.** OpenHuman embeds one runtime per process.
  `rsi-ralph-swarm`'s own integration tests sometimes fail with
  `Runtime(AlreadyRunning)` when one test's runtime is still shutting down as
  the next starts. That is seen on unmodified `8e44890` too.
- **License.** GPL-3.0-only, the same as supermath.

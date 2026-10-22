//! Request and answer types of the Python service's HTTP API.
//!
//! The wire form is specified in `docs/specs/toolkit/python/README.md`
//! ("HTTP API"); these types are its Rust side, and their serde forms are
//! pinned in the module tests.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};

/// A `POST /run` request: call one function of a project file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunRequest {
    /// The project slug.
    pub project: String,
    /// The file, e.g. `python/work/sunflower_search.py`.
    pub path: String,
    /// The function to call.
    pub entry: String,
    /// Keyword arguments: a JSON object.
    pub args: Value,
    /// The seed for `random`, `numpy`, and a `seed` parameter.
    pub seed: u64,
    /// The wall-clock limit in seconds.
    pub timeout_s: u64,
}

/// A `POST /search` request: look for an input where a predicate is false.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchRequest {
    /// The project slug.
    pub project: String,
    /// The file holding the predicate.
    pub path: String,
    /// The predicate: a function of one argument returning `bool`.
    pub predicate: String,
    /// The frame, in the form [`Frame::to_wire`](crate::Frame::to_wire) gives.
    pub frame: Value,
    /// The seed for `random` and `numpy`.
    pub seed: u64,
    /// The wall-clock limit in seconds.
    pub timeout_s: u64,
    /// The most values the service will check before stopping.
    pub max_frame: u64,
}

/// A limit that ended a run, which is `unknown` and never a mathematical
/// answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Limit {
    /// The wall-clock timeout; the run was killed.
    Time,
    /// The CPU-time limit.
    Cpu,
    /// The address-space limit, or the kernel's out-of-memory kill.
    Memory,
    /// More output (or a larger return value) than the service allows.
    Output,
    /// The interpreter died without an answer, for example in native code.
    Crashed,
}

impl Limit {
    /// The wire name, also used in `unknowns` text.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Time => "time",
            Self::Cpu => "cpu",
            Self::Memory => "memory",
            Self::Output => "output",
            Self::Crashed => "crashed",
        }
    }

    /// A sentence for the agent: which limit, and that it settles nothing.
    #[must_use]
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Time => "the run hit its wall-clock timeout and was killed",
            Self::Cpu => "the run used up its CPU-time limit",
            Self::Memory => "the run exceeded its memory limit",
            Self::Output => "the run produced more output than the service allows",
            Self::Crashed => "the interpreter died without an answer",
        }
    }
}

/// An exception raised by the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exception {
    /// The exception class name, e.g. `ValueError`.
    #[serde(rename = "type")]
    pub type_name: String,
    /// `str(exception)`, cut to 2,000 characters.
    pub message: String,
    /// The last ten frames, innermost last, as `file:line in function: code`.
    #[serde(default)]
    pub traceback: Vec<String>,
}

/// What the run printed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Output {
    /// Standard output, cut to 8,000 characters.
    pub stdout: String,
    /// Whether `stdout` was cut.
    pub stdout_truncated: bool,
    /// Standard error, cut to 8,000 characters.
    pub stderr: String,
    /// Whether `stderr` was cut.
    pub stderr_truncated: bool,
}

/// What the service reports about how a run went, whatever its result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Execution {
    /// The Python version.
    pub python: String,
    /// The digest of the service's `uv.lock`.
    pub lock_sha256: Option<String>,
    /// The digest of the file as the service read it.
    pub file_sha256: String,
    /// Wall time in milliseconds.
    pub elapsed_ms: u64,
    /// Peak resident memory in MiB, when the run lived to report it.
    pub peak_rss_mb: Option<u64>,
    /// What was printed.
    pub output: Output,
}

/// How a `/run` ended.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum RunOutcome {
    /// The function returned this JSON value (integers past 2^53 are strings).
    Returned {
        /// The value.
        value: Value,
    },
    /// The function raised.
    Raised {
        /// The exception.
        exception: Exception,
    },
    /// A limit ended the run.
    Limit {
        /// Which one.
        limit: Limit,
    },
}

/// The answer to a `/run`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunReport {
    /// How it ended.
    pub outcome: RunOutcome,
    /// How it went.
    pub execution: Execution,
}

/// How a `/search` ended.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum SearchOutcome {
    /// The predicate was false at `witness`.
    Found {
        /// The witness as JSON (integers past 2^53 are strings).
        witness: Value,
        /// The witness as lossless JSON text, big integers intact.
        witness_json: String,
        /// Where the witness held tuples, which JSON turned into lists: each
        /// a path of indices and keys from the root (`[]` is the witness
        /// itself). Empty from a service that does not report them.
        witness_tuples: Vec<Value>,
        /// How many values passed before it.
        checked: u64,
    },
    /// The whole frame was checked and the predicate held throughout.
    Exhausted {
        /// How many values were checked.
        checked: u64,
    },
    /// The service stopped at its admission cap before the frame ended.
    Capped {
        /// How many values were checked.
        checked: u64,
    },
    /// The predicate raised (or returned a non-bool) on `input`.
    PredicateRaised {
        /// The input it failed on.
        input: Value,
        /// What it raised.
        exception: Exception,
        /// How many values passed before it.
        checked: u64,
    },
    /// The file failed to import or a generator raised.
    Raised {
        /// The exception.
        exception: Exception,
    },
    /// A limit ended the search.
    Limit {
        /// Which one.
        limit: Limit,
        /// How many values were checked, when progress was reported.
        checked: Option<u64>,
    },
}

/// The answer to a `/search`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchReport {
    /// How it ended.
    pub outcome: SearchOutcome,
    /// How it went.
    pub execution: Execution,
}

/// One solver in the pinned environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Solver {
    /// The distribution name in `uv.lock`, e.g. `python-flint`.
    pub package: String,
    /// The locked version.
    pub version: String,
    /// The name to `import`, e.g. `flint`.
    pub import_name: String,
    /// `symbolic`, `numeric`, `sat`, `smt`, `optimization`, or `graphs`.
    pub area: String,
    /// One line on what it is for.
    pub summary: String,
}

/// The answer to `GET /solvers`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SolverList {
    /// The Python version.
    pub python: String,
    /// The digest of the lock the list was read from.
    #[serde(default)]
    pub lock_sha256: Option<String>,
    /// The solvers.
    pub solvers: Vec<Solver>,
}

/// The answer to `GET /health`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Health {
    /// Always `ok` when the service answers.
    pub status: String,
    /// The Python version.
    pub python: String,
    /// The digest of the lock.
    #[serde(default)]
    pub lock_sha256: Option<String>,
}

/// The common wire answer to `/run` and `/search`, before it is typed.
#[derive(Debug, Deserialize)]
pub(crate) struct Answer {
    result: String,
    #[serde(default)]
    limit: Option<Limit>,
    #[serde(default)]
    value: Value,
    #[serde(default)]
    exception: Option<Exception>,
    #[serde(default)]
    search: Option<Detail>,
    #[serde(default)]
    stdout: String,
    #[serde(default)]
    stdout_truncated: bool,
    #[serde(default)]
    stderr: String,
    #[serde(default)]
    stderr_truncated: bool,
    python: String,
    #[serde(default)]
    lock_sha256: Option<String>,
    #[serde(default)]
    file_sha256: Option<String>,
    elapsed_ms: u64,
    #[serde(default)]
    peak_rss_mb: Option<u64>,
}

/// The `search` object of a `/search` answer.
#[derive(Debug, Default, Deserialize)]
struct Detail {
    #[serde(default)]
    checked: Option<u64>,
    #[serde(default)]
    witness: Value,
    #[serde(default)]
    witness_json: Option<String>,
    #[serde(default)]
    witness_tuples: Vec<Value>,
    #[serde(default)]
    input: Value,
    #[serde(default)]
    exception: Option<Exception>,
}

fn decode(message: impl Into<String>) -> Error {
    Error::Decode(message.into())
}

impl Answer {
    fn execution(&mut self) -> Result<Execution> {
        Ok(Execution {
            python: std::mem::take(&mut self.python),
            lock_sha256: self.lock_sha256.take(),
            file_sha256: self
                .file_sha256
                .take()
                .ok_or_else(|| decode("the answer has no file_sha256"))?,
            elapsed_ms: self.elapsed_ms,
            peak_rss_mb: self.peak_rss_mb,
            output: Output {
                stdout: std::mem::take(&mut self.stdout),
                stdout_truncated: self.stdout_truncated,
                stderr: std::mem::take(&mut self.stderr),
                stderr_truncated: self.stderr_truncated,
            },
        })
    }

    fn require_limit(&self) -> Result<Limit> {
        self.limit
            .ok_or_else(|| decode("a limit answer names no limit"))
    }

    fn require_exception(&mut self) -> Result<Exception> {
        self.exception
            .take()
            .ok_or_else(|| decode("a raised answer carries no exception"))
    }

    /// Types a `/run` answer.
    pub(crate) fn into_run(mut self) -> Result<RunReport> {
        let outcome = match self.result.as_str() {
            "returned" => RunOutcome::Returned {
                value: std::mem::take(&mut self.value),
            },
            "raised" => RunOutcome::Raised {
                exception: self.require_exception()?,
            },
            "limit" => RunOutcome::Limit {
                limit: self.require_limit()?,
            },
            other => return Err(decode(format!("unknown run result {other:?}"))),
        };
        Ok(RunReport {
            outcome,
            execution: self.execution()?,
        })
    }

    /// Types a `/search` answer.
    pub(crate) fn into_search(mut self) -> Result<SearchReport> {
        let detail = self.search.take().unwrap_or_default();
        let checked = detail.checked;
        let need_checked = || checked.ok_or_else(|| decode("a search answer carries no count"));
        let outcome = match self.result.as_str() {
            "found" => SearchOutcome::Found {
                witness: detail.witness,
                witness_json: detail
                    .witness_json
                    .ok_or_else(|| decode("a found answer carries no witness_json"))?,
                witness_tuples: detail.witness_tuples,
                checked: need_checked()?,
            },
            "exhausted" => SearchOutcome::Exhausted {
                checked: need_checked()?,
            },
            "capped" => SearchOutcome::Capped {
                checked: need_checked()?,
            },
            "predicate_raised" => SearchOutcome::PredicateRaised {
                input: detail.input,
                exception: detail
                    .exception
                    .ok_or_else(|| decode("a predicate_raised answer carries no exception"))?,
                checked: need_checked()?,
            },
            "raised" => SearchOutcome::Raised {
                exception: self.require_exception()?,
            },
            "limit" => SearchOutcome::Limit {
                limit: self.require_limit()?,
                checked,
            },
            other => return Err(decode(format!("unknown search result {other:?}"))),
        };
        Ok(SearchReport {
            outcome,
            execution: self.execution()?,
        })
    }
}

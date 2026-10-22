//! Tests against the real Python service. They run only when `PYTHON_URL` is
//! set, and need `SUPERMATH_PROJECTS` to name the directory the service
//! mounts (read-only) at `/projects`:
//!
//! ```sh
//! docker compose up -d python
//! PYTHON_URL=http://localhost:3160 SUPERMATH_PROJECTS=./.supermath/projects \
//!     cargo test -p supermath-python --test live
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use supermath_files::{ProjectPath, Workspace, WriteMeta};
use supermath_python::{Python, PythonConfig};
use supermath_tool::{CallContext, Outcome, Project};

struct Live {
    python: Python,
    workspace: Arc<Workspace>,
    root: PathBuf,
    slug: String,
}

impl Live {
    fn start(name: &str) -> Option<Self> {
        let config = PythonConfig::from_env();
        let (Some(_), Ok(root)) = (&config.url, std::env::var("SUPERMATH_PROJECTS")) else {
            eprintln!("skipping {name}: PYTHON_URL and SUPERMATH_PROJECTS are not both set");
            return None;
        };
        let root = PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        let workspace = Arc::new(Workspace::new(&root));
        let python = Python::new(&config, workspace.clone()).unwrap();
        Some(Self {
            python,
            workspace,
            root,
            slug: format!("live-{}-{}", name.replace('_', "-"), std::process::id()),
        })
    }

    fn write(&self, path: &str, source: &str) {
        let path = ProjectPath::parse(path).unwrap();
        let meta = WriteMeta::new("file_write", "live");
        self.workspace
            .write(&self.slug, &path, source, None, &meta)
            .unwrap();
    }

    async fn call(&self, tool: &str, args: Value) -> supermath_tool::Envelope {
        let tool = self
            .python
            .tools()
            .into_iter()
            .find(|t| t.name() == tool)
            .unwrap();
        let ctx = CallContext::new(
            Project::new(&self.slug, "ws:live/project:live").unwrap(),
            "live-call",
        );
        tool.call(&ctx, args).await
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.root.join(&self.slug));
    }
}

#[tokio::test]
async fn live_python_run() {
    let Some(live) = Live::start("live_python_run") else {
        return;
    };
    let health = live.python.client().health().await.unwrap();
    assert!(health.python.starts_with("3.12"), "{health:?}");

    live.write(
        "python/work/factor.py",
        "import sympy\n\n\ndef factor(n):\n    return {str(p): e for p, e in sympy.factorint(n).items()}\n",
    );
    let answer = live
        .call(
            "py_run",
            json!({"path": "python/work/factor.py", "entry": "factor", "args": {"n": 2_u64.pow(32) + 1}}),
        )
        .await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    // 2^32 + 1 = 641 * 6700417 (Euler); 2^64 + 1 needs a 64-bit argument.
    assert_eq!(answer.data["value"], json!({"641": 1, "6700417": 1}));
    assert_eq!(answer.data["provenance"]["entry"], "factor");

    live.write(
        "python/work/fermat.py",
        "import sympy\n\n\ndef factor():\n    return {str(p): e for p, e in sympy.factorint(2**64 + 1).items()}\n",
    );
    let big = live
        .call(
            "py_run",
            json!({"path": "python/work/fermat.py", "entry": "factor"}),
        )
        .await;
    assert_eq!(
        big.data["value"],
        json!({"274177": 1, "67280421310721": 1}),
        "{big:?}"
    );

    let solvers = live.call("py_solvers", json!({"area": "smt"})).await;
    assert_eq!(solvers.data["solvers"][0]["package"], "z3-solver");
}

#[tokio::test]
async fn live_python_search_counterexample() {
    let Some(live) = Live::start("live_python_search") else {
        return;
    };
    live.write(
        "python/work/euler.py",
        "def prime_value(n):\n    m = n * n + n + 41\n    return all(m % d for d in range(2, int(m ** 0.5) + 1))\n",
    );
    let answer = live
        .call(
            "py_search_counterexample",
            json!({"path": "python/work/euler.py", "predicate": "prime_value",
                   "frame": {"kind": "range", "from": 0, "to": 100}}),
        )
        .await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    assert_eq!(answer.data["witness"], 40);
    // The generated witness file re-runs.
    let witness = answer.data["witness_file"]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let rerun = live
        .call("py_run", json!({"path": witness, "entry": "check"}))
        .await;
    assert_eq!(
        rerun.data["value"],
        json!({"witness": 40, "predicate_holds": false}),
        "{rerun:?}"
    );
}

#[tokio::test]
async fn live_python_isolation() {
    let Some(live) = Live::start("live_python_isolation") else {
        return;
    };
    live.write(
        "python/work/escape.py",
        r#"import os
import socket
import time


def attempts():
    found = {}
    try:
        s = socket.create_connection(("1.1.1.1", 53), timeout=3)
        found["socket"] = "connected"
        s.close()
    except OSError as error:
        found["socket"] = "refused: " + type(error).__name__
    try:
        with open(os.path.join(os.path.dirname(__file__), "pwned.txt"), "w") as handle:
            handle.write("x")
        found["write_project"] = "written"
    except OSError as error:
        found["write_project"] = "refused: " + type(error).__name__
    try:
        with open("/escape.txt", "w") as handle:
            handle.write("x")
        found["write_root"] = "written"
    except OSError as error:
        found["write_root"] = "refused: " + type(error).__name__
    return found


def forever():
    print("started", flush=True)
    while True:
        time.sleep(1)
"#,
    );
    let answer = live
        .call(
            "py_run",
            json!({"path": "python/work/escape.py", "entry": "attempts", "timeout_s": 20}),
        )
        .await;
    assert_eq!(answer.outcome, Outcome::Ok, "{answer:?}");
    let found = &answer.data["value"];
    for key in ["socket", "write_project", "write_root"] {
        assert!(
            found[key].as_str().unwrap().starts_with("refused"),
            "{key} was not refused: {found}"
        );
    }
    assert!(
        !live
            .root
            .join(&live.slug)
            .join("python/work/pwned.txt")
            .exists()
    );

    let started = std::time::Instant::now();
    let killed = live
        .call(
            "py_run",
            json!({"path": "python/work/escape.py", "entry": "forever", "timeout_s": 2}),
        )
        .await;
    assert_eq!(killed.outcome, Outcome::Unknown, "{killed:?}");
    assert_eq!(killed.data["limit"], "time");
    assert_eq!(killed.data["stdout"], "started\n");
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
}

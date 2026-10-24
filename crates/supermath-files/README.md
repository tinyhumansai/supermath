# supermath-files

The project workspace and the four `file_*` tools. Source reaches a project
only through them; every tool that runs code runs a file here. The spec is
[`docs/specs/toolkit/files/`](../../docs/specs/toolkit/files/README.md).

## Layout

```text
<SUPERMATH_PROJECTS>/<slug>/
├── lean/Supermath/…        Lean modules   (^lean/Supermath/[A-Z]…\.lean$)
├── python/work/…           Python modules (^python/work/[a-z]…\.py$)
├── .log/writes.jsonl       the write log
└── .log/calls.jsonl        the call log, for hosts that run each call as a process
```

With `SUPERMATH_PROJECT_DIR` set (or `Workspace::single(dir)`), the project
is that one directory instead of `<SUPERMATH_PROJECTS>/<slug>/`.

## Public surface

| Module | Exports |
| --- | --- |
| `workspace` | `Workspace` (`new`, `single`, `resolve`, `read`, `sha256`, `exists`, `list`, `write`, `imports_digest`, `write_log`, `call_log`), `SourceFile`, `WriteMeta`, `WriteOutcome`, `ImportsDigest`, `ImportedModule`, `sha256_hex`, `MAX_FILE_BYTES` |
| `path` | `ProjectPath`, `Language` |
| `imports` | `check`, `parse`, `is_project_module`, `Import` |
| `outline` | `outline`, `OutlineItem` |
| `write_log` | `WriteLog`, `WriteRecord` |
| `call_log` | `FileCallLog`, the `CallLog` port on disk |
| `observer` | `FileObserver` (port), `LastResult` |
| `config` | `FilesConfig` (`SUPERMATH_PROJECTS`, `SUPERMATH_PROJECT_DIR`) |
| `tools` | `FileWrite`, `FileEdit`, `FileRead`, `FileList`, `file_tools` |

## Design constraints

- Rules live in `Workspace`, not the tools: the Lean and Python crates reuse
  the same path, symlink, import, and digest logic.
- A write is checked (size, generated directory, imports, `if_sha256`) before
  it touches the disk, lands through a same-directory temporary file and a
  rename, and is logged only after it succeeds. One lock inside `Workspace`
  serialises writes, so share a single `Arc<Workspace>`.
- The project slug comes from `CallContext`, never from tool arguments.
- Disk calls are blocking `std::fs` on files of at most 1 MB.
- The import check is an early refusal, not the sandbox: the Lean and Python
  services are the real boundary.

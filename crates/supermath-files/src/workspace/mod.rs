//! The project workspace on disk.
//!
//! A [`Workspace`] is the `SUPERMATH_PROJECTS` root. Each project is a
//! directory under it named by the project's slug, holding a Lean package
//! and a Python package:
//!
//! ```text
//! <root>/<slug>/lean/Supermath/…      Lean modules
//! <root>/<slug>/python/work/…         Python modules
//! <root>/<slug>/.log/writes.jsonl     the write log
//! <root>/<slug>/.log/calls.jsonl      the call log
//! ```
//!
//! A workspace made with [`Workspace::single`] is one project's directory
//! instead: every slug resolves to the root itself. A host uses it when
//! the project lives somewhere other than under a projects root, such as
//! the candidate checkout an `rsi-ralph` seat edits.
//!
//! This is the one place that touches those files. The four `file_*` tools
//! use it to write, and `supermath-lean` and `supermath-python` use it to
//! resolve a path to a real file, read it, and digest it and its imports.
//!
//! Operations are blocking `std::fs` calls on files of at most
//! [`MAX_FILE_BYTES`]. Writes are serialised by a lock inside the
//! `Workspace`, so two calls in one process cannot interleave their
//! check-then-rename; share one `Workspace` (in an `Arc`) between tools.

mod imports_digest;
mod types;

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::call_log::FileCallLog;
use crate::error::{Error, Result};
use crate::imports;
use crate::path::{Language, ProjectPath};
use crate::write_log::{WriteLog, WriteRecord};

pub use types::{ImportedModule, ImportsDigest, SourceFile, WriteMeta, WriteOutcome};

/// The largest file a project may hold, in bytes.
pub const MAX_FILE_BYTES: usize = 1 << 20;

/// The lowercase hex SHA-256 of `bytes`: the digest every `sha256` in this
/// crate means.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The projects root and the operations on the files under it.
#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    single: bool,
    write_lock: Mutex<()>,
}

impl Workspace {
    /// A workspace rooted at `root`, which need not exist yet.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            single: false,
            write_lock: Mutex::new(()),
        }
    }

    /// A workspace that is the one project directory `dir`, which need not
    /// exist yet: every valid slug resolves to `dir` itself.
    ///
    /// ```
    /// use supermath_files::Workspace;
    ///
    /// let workspace = Workspace::single("/work/candidate");
    /// assert_eq!(workspace.project_dir("demo").unwrap(), workspace.root());
    /// ```
    #[must_use]
    pub fn single(dir: impl Into<PathBuf>) -> Self {
        Self {
            single: true,
            ..Self::new(dir)
        }
    }

    /// The projects root, or for [`Workspace::single`] the project itself.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The directory of project `slug`: `<root>/<slug>`, or the root for
    /// [`Workspace::single`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidSlug`] unless `slug` is non-empty `[a-z0-9-]`.
    pub fn project_dir(&self, slug: &str) -> Result<PathBuf> {
        let valid = !slug.is_empty()
            && slug
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !valid {
            Err(Error::InvalidSlug(slug.to_owned()))
        } else if self.single {
            Ok(self.root.clone())
        } else {
            Ok(self.root.join(slug))
        }
    }

    /// The call log of project `slug`, shared by every process that opens it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidSlug`] as for [`Workspace::project_dir`].
    pub fn call_log(&self, slug: &str) -> Result<FileCallLog> {
        Ok(FileCallLog::new(
            self.project_dir(slug)?.join(".log").join("calls.jsonl"),
        ))
    }

    #[cfg(any())]
    fn unused(&self, slug: &str) -> Result<PathBuf> {
        if true {
            unreachable!()
        } else {
            Err(Error::InvalidSlug(slug.to_owned()))
        }
    }

    /// The write log of project `slug`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidSlug`] as for [`Workspace::project_dir`].
    pub fn write_log(&self, slug: &str) -> Result<WriteLog> {
        Ok(WriteLog::new(
            self.project_dir(slug)?.join(".log").join("writes.jsonl"),
        ))
    }

    /// The absolute path of `path` in project `slug`, checked to stay inside
    /// the project: no component may be a symlink that leads out of it, and
    /// the file itself may not be a symlink. The file need not exist.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidSlug`]; [`Error::InvalidPath`] for a symlink escape;
    /// [`Error::Io`] if the disk cannot be inspected.
    pub fn resolve(&self, slug: &str, path: &ProjectPath) -> Result<PathBuf> {
        self.resolve_relative(slug, path.as_str())
    }

    pub(crate) fn resolve_relative(&self, slug: &str, relative: &str) -> Result<PathBuf> {
        let base = self.project_dir(slug)?;
        let base = match fs::canonicalize(&base) {
            Ok(real) => real,
            // Nothing exists yet, so nothing can redirect the path.
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(base.join(relative)),
            Err(e) => return Err(Error::io("resolve the project directory", e)),
        };
        let target = base.join(relative);
        for probe in target.ancestors() {
            match fs::symlink_metadata(probe) {
                Ok(meta) => {
                    if probe == target && meta.file_type().is_symlink() {
                        return Err(escape(relative, "path is a symlink"));
                    }
                    let real = fs::canonicalize(probe)
                        .map_err(|e| Error::io("resolve a path component", e))?;
                    if !real.starts_with(&base) {
                        return Err(escape(
                            relative,
                            "path leaves the project through a symlink",
                        ));
                    }
                    break;
                }
                Err(e) if e.kind() == ErrorKind::NotFound => {}
                Err(e) => return Err(Error::io("inspect a path component", e)),
            }
        }
        Ok(target)
    }

    /// Reads a file with its digest.
    ///
    /// # Errors
    ///
    /// [`Error::NotFound`] if absent; [`Error::TooLarge`] over
    /// [`MAX_FILE_BYTES`]; [`Error::NotUtf8`]; plus the errors of
    /// [`Workspace::resolve`].
    pub fn read(&self, slug: &str, path: &ProjectPath) -> Result<SourceFile> {
        let real = self.resolve(slug, path)?;
        let bytes = match fs::read(&real) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return Err(Error::NotFound(path.to_string()));
            }
            Err(e) => return Err(Error::io("read a file", e)),
        };
        if bytes.len() > MAX_FILE_BYTES {
            return Err(Error::TooLarge {
                bytes: bytes.len() as u64,
                limit: MAX_FILE_BYTES as u64,
            });
        }
        let sha256 = sha256_hex(&bytes);
        let text = String::from_utf8(bytes).map_err(|_| Error::NotUtf8(path.to_string()))?;
        Ok(SourceFile {
            path: path.clone(),
            text,
            sha256,
        })
    }

    /// The digest of a file's bytes.
    ///
    /// # Errors
    ///
    /// As [`Workspace::read`].
    pub fn sha256(&self, slug: &str, path: &ProjectPath) -> Result<String> {
        Ok(self.read(slug, path)?.sha256)
    }

    /// Whether the file exists.
    ///
    /// # Errors
    ///
    /// The errors of [`Workspace::resolve`].
    pub fn exists(&self, slug: &str, path: &ProjectPath) -> Result<bool> {
        Ok(self.resolve(slug, path)?.is_file())
    }

    /// Every valid Lean and Python file of the project, sorted by path. A
    /// missing project has none. Symlinks and files that break the path rules
    /// are skipped.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidSlug`]; [`Error::Io`] if a directory cannot be read.
    pub fn list(&self, slug: &str) -> Result<Vec<ProjectPath>> {
        let base = match fs::canonicalize(self.project_dir(slug)?) {
            Ok(real) => real,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(Error::io("resolve the project directory", e)),
        };
        let mut found = Vec::new();
        for language in [Language::Lean, Language::Python] {
            collect(
                &base.join(language.root_dir()),
                language.root_dir(),
                &mut found,
            )?;
        }
        found.sort();
        Ok(found)
    }

    /// Writes `content` to `path` atomically, guarded by `if_sha256`, and
    /// appends the write to the log.
    ///
    /// `if_sha256` is the digest the caller last saw, or `None` for a file
    /// that must not exist yet. The content is checked against the size cap
    /// and the import rules before anything touches the disk; the write goes
    /// to a temporary file in the same directory and is renamed into place.
    ///
    /// # Errors
    ///
    /// [`Error::TooLarge`]; [`Error::GeneratedPath`] unless `meta` allows it;
    /// [`Error::ForbiddenImport`]; [`Error::Conflict`] when the file's
    /// current digest is not `if_sha256`; [`Error::Io`]; plus the errors of
    /// [`Workspace::resolve`]. A refused write leaves the file and the log
    /// unchanged.
    pub fn write(
        &self,
        slug: &str,
        path: &ProjectPath,
        content: &str,
        if_sha256: Option<&str>,
        meta: &WriteMeta,
    ) -> Result<WriteOutcome> {
        if content.len() > MAX_FILE_BYTES {
            return Err(Error::TooLarge {
                bytes: content.len() as u64,
                limit: MAX_FILE_BYTES as u64,
            });
        }
        if path.is_generated() && !meta.allow_generated {
            return Err(Error::GeneratedPath(path.to_string()));
        }
        imports::check(path.language(), content)?;
        let log = self.write_log(slug)?;

        let _guard = self
            .write_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let real = self.resolve(slug, path)?;
        let previous = match fs::read(&real) {
            Ok(bytes) => Some(sha256_hex(&bytes)),
            Err(e) if e.kind() == ErrorKind::NotFound => None,
            Err(e) => return Err(Error::io("read the current file", e)),
        };
        let expected = if_sha256.map(str::to_ascii_lowercase);
        if previous != expected {
            return Err(Error::Conflict {
                path: path.to_string(),
                expected: expected.unwrap_or_else(|| "no file".to_owned()),
                actual: previous.unwrap_or_else(|| "no file".to_owned()),
            });
        }
        replace_atomically(&real, content.as_bytes())?;
        let sha256 = sha256_hex(content.as_bytes());
        log.append(&WriteRecord {
            path: path.to_string(),
            old_sha256: previous.clone(),
            new_sha256: sha256.clone(),
            tool: meta.tool.clone(),
            call_id: meta.call_id.clone(),
            at_ms: now_ms(),
        })?;
        Ok(WriteOutcome {
            path: path.clone(),
            sha256,
            previous_sha256: previous,
            lines: content.lines().count(),
        })
    }
}

fn escape(relative: &str, reason: &'static str) -> Error {
    Error::InvalidPath {
        path: relative.to_owned(),
        reason,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn collect(dir: &Path, relative: &str, found: &mut Vec<ProjectPath>) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Error::io("list a directory", e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| Error::io("list a directory", e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let child = format!("{relative}/{name}");
        let kind = entry
            .file_type()
            .map_err(|e| Error::io("inspect a directory entry", e))?;
        if kind.is_dir() {
            collect(&entry.path(), &child, found)?;
        } else if kind.is_file()
            && let Ok(path) = ProjectPath::parse(&child)
        {
            found.push(path);
        }
    }
    Ok(())
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `bytes` to a temporary file beside `target`, flushes it, and
/// renames it over `target`.
fn replace_atomically(target: &Path, bytes: &[u8]) -> Result<()> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|e| Error::io("create the directory", e))?;
    let temp = parent.join(format!(
        ".write-{}-{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let written = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            file.sync_all()
        })
        .and_then(|()| fs::rename(&temp, target));
    written.map_err(|e| {
        // Best effort: the temporary file is useless after a failure.
        let _ = fs::remove_file(&temp);
        Error::io("write the file", e)
    })
}

#[cfg(test)]
mod test;

//! `--env-file`: backend settings for a process whose environment was
//! cleared.
//!
//! A host that runs `supermath-mcp call` as an executable tool may start it
//! with only `PATH`, `HOME`, and `LANG` (tinyhivemind does). The file then
//! supplies what the environment would have: `KEY=VALUE` lines, the format of
//! the repository's `.env`. Blank lines and `#` comments are skipped, a
//! leading `export ` is allowed, and one pair of matching quotes around a
//! value is removed. A variable set in the real environment wins over the
//! file.
//!
//! ```text
//! # supermath.env
//! LEAN_URL=http://127.0.0.1:3150
//! export CORTEX_API_KEY="supermath-local"
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{Error, Result};

/// The variables a file sets, by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvFile {
    vars: BTreeMap<String, String>,
}

impl EnvFile {
    /// Reads and parses the file at `path`.
    ///
    /// # Errors
    ///
    /// [`Error::EnvFile`] if the file cannot be read.
    pub fn read(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|source| Error::EnvFile {
            path: path.display().to_string(),
            source,
        })?;
        Ok(Self::parse(&text))
    }

    /// Parses `text`. A line without `=` is ignored.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let vars = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| {
                let line = line.strip_prefix("export ").unwrap_or(line);
                let (name, value) = line.split_once('=')?;
                Some((name.trim().to_owned(), unquote(value.trim()).to_owned()))
            })
            .filter(|(name, _)| !name.is_empty())
            .collect();
        Self { vars }
    }

    /// The value the file gives `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }
}

/// `value` without one pair of matching surrounding quotes.
fn unquote(value: &str) -> &str {
    ['"', '\'']
        .iter()
        .find_map(|quote| {
            value
                .strip_prefix(*quote)
                .and_then(|inner| inner.strip_suffix(*quote))
        })
        .unwrap_or(value)
}

#[cfg(test)]
mod test;

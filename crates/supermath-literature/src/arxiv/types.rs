//! arXiv identifiers, papers, and search requests.

use std::fmt;

use serde::Serialize;

use crate::error::{Error, Result};

/// A validated arXiv identifier, with an optional version.
///
/// Accepts `2310.01234`, `2310.01234v2`, the old `math/0301234v1` form, an
/// `arXiv:` prefix, and `arxiv.org/abs/` or `/pdf/` URLs.
///
/// ```
/// use supermath_literature::ArxivId;
///
/// let id = ArxivId::parse("arXiv:2310.01234v2").unwrap();
/// assert_eq!(id.base(), "2310.01234");
/// assert_eq!(id.version(), Some(2));
/// assert_eq!(id.to_string(), "2310.01234v2");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArxivId {
    base: String,
    version: Option<u32>,
}

impl ArxivId {
    /// Parses and validates an identifier.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`] when `raw` is not an arXiv identifier.
    pub fn parse(raw: &str) -> Result<Self> {
        let bad = || Error::InvalidInput(format!("{raw:?} is not an arxiv id"));
        let mut s = raw.trim();
        for prefix in [
            "https://arxiv.org/abs/",
            "http://arxiv.org/abs/",
            "https://arxiv.org/pdf/",
            "http://arxiv.org/pdf/",
            "arxiv.org/abs/",
            "arxiv.org/pdf/",
        ] {
            if let Some(rest) = s.strip_prefix(prefix) {
                s = rest.strip_suffix(".pdf").unwrap_or(rest);
                break;
            }
        }
        if s.len() >= 6 && s[..6].eq_ignore_ascii_case("arxiv:") {
            s = &s[6..];
        }
        let (base, version) = match s.rfind('v') {
            Some(i)
                if i > 0
                    && !s[i + 1..].is_empty()
                    && s[i + 1..].bytes().all(|b| b.is_ascii_digit()) =>
            {
                let v: u32 = s[i + 1..].parse().map_err(|_| bad())?;
                if v == 0 {
                    return Err(bad());
                }
                (&s[..i], Some(v))
            }
            _ => (s, None),
        };
        if new_style(base) || old_style(base) {
            Ok(Self {
                base: base.to_owned(),
                version,
            })
        } else {
            Err(bad())
        }
    }

    /// The identifier without its version.
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    /// The version, if the identifier named one.
    #[must_use]
    pub fn version(&self) -> Option<u32> {
        self.version
    }

    /// The same identifier pinned to `version`.
    #[must_use]
    pub fn with_version(&self, version: u32) -> Self {
        Self {
            base: self.base.clone(),
            version: Some(version),
        }
    }
}

impl fmt::Display for ArxivId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.version {
            Some(v) => write!(f, "{}v{v}", self.base),
            None => f.write_str(&self.base),
        }
    }
}

/// `YYMM.NNNN` or `YYMM.NNNNN`.
fn new_style(s: &str) -> bool {
    let Some((head, tail)) = s.split_once('.') else {
        return false;
    };
    head.len() == 4
        && head.bytes().all(|b| b.is_ascii_digit())
        && (4..=5).contains(&tail.len())
        && tail.bytes().all(|b| b.is_ascii_digit())
}

/// `archive/YYMMNNN`, with an optional `.XX` subject class on the archive.
fn old_style(s: &str) -> bool {
    let Some((archive, number)) = s.split_once('/') else {
        return false;
    };
    let (name, class) = archive
        .split_once('.')
        .map_or((archive, None), |(n, c)| (n, Some(c)));
    !name.is_empty()
        && name.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
        && class.is_none_or(|c| c.len() == 2 && c.bytes().all(|b| b.is_ascii_uppercase()))
        && number.len() == 7
        && number.bytes().all(|b| b.is_ascii_digit())
}

/// One paper's metadata from the arXiv Atom feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Paper {
    /// The identifier without a version.
    pub arxiv_id: String,
    /// The version this record describes.
    pub version: u32,
    /// The title, whitespace collapsed.
    pub title: String,
    /// Every author, in order.
    pub authors: Vec<String>,
    /// The primary category, such as `math.NT`.
    pub primary_category: String,
    /// Every category, primary first.
    pub categories: Vec<String>,
    /// When version 1 was submitted, as arXiv reports it.
    pub published: String,
    /// When this version was submitted.
    pub updated: String,
    /// The abstract, whitespace collapsed.
    pub summary: String,
}

/// A page of search results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPage {
    /// The papers on this page.
    pub papers: Vec<Paper>,
    /// How many papers matched in all.
    pub total: u64,
    /// The offset of the first paper on this page.
    pub start: u64,
    /// How old the cached answer was, in seconds, if it was cached.
    pub cached_age_secs: Option<u64>,
}

/// How a search is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// arXiv's relevance ranking.
    #[default]
    Relevance,
    /// Newest submissions first.
    Recent,
}

/// A search request. At least one of `text`, `title` and `author` is needed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchQuery {
    /// Free text, matched against every field.
    pub text: Option<String>,
    /// Words that must appear in the title.
    pub title: Option<String>,
    /// An author name.
    pub author: Option<String>,
    /// An arXiv category such as `math.NT`.
    pub category: Option<String>,
    /// Only papers submitted on or after this `YYYY-MM-DD` date.
    pub since: Option<String>,
    /// The ordering.
    pub sort: SortOrder,
    /// The offset of the first result.
    pub start: u64,
    /// How many results to return.
    pub limit: usize,
}

/// What `read` returns: metadata and the unpacked source.
#[derive(Debug, Clone)]
pub struct Read {
    /// The paper's metadata.
    pub paper: Paper,
    /// The e-print, unpacked.
    pub source: crate::Source,
    /// The oldest cached answer used, in seconds.
    pub cached_age_secs: Option<u64>,
}

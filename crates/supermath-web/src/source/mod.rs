//! Sources: where a finding came from, and what kind of place that is.
//!
//! Every resolved source is classified by its domain so the agent can weigh
//! it without opening it. The table is data, [`CLASS_TABLE`], with a test per
//! row.

use serde::{Deserialize, Serialize};
use url::Url;

/// What kind of place a source is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceClass {
    /// A preprint server: arXiv, HAL, `IACR` `ePrint`.
    Preprint,
    /// A journal or publisher page, including DOI landing pages.
    Journal,
    /// Question-and-answer sites: `MathOverflow`, Math Stack Exchange.
    Qa,
    /// Reference works: `OEIS`, `zbMATH`, `MathWorld`, Wikipedia.
    Reference,
    /// University pages, course notes, slides.
    Notes,
    /// Blogs.
    Blog,
    /// Everything else.
    Other,
}

impl SourceClass {
    /// The wire spelling, e.g. `"preprint"`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preprint => "preprint",
            Self::Journal => "journal",
            Self::Qa => "qa",
            Self::Reference => "reference",
            Self::Notes => "notes",
            Self::Blog => "blog",
            Self::Other => "other",
        }
    }

    /// Parses the wire spelling.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [
            Self::Preprint,
            Self::Journal,
            Self::Qa,
            Self::Reference,
            Self::Notes,
            Self::Blog,
            Self::Other,
        ]
        .into_iter()
        .find(|class| class.as_str() == text)
    }
}

impl SourceClass {
    /// How a prompt names the class to Gemini.
    pub(crate) fn describe(self) -> &'static str {
        match self {
            Self::Preprint => "preprints (arXiv and similar servers)",
            Self::Journal => "journal articles",
            Self::Qa => "MathOverflow and Math Stack Exchange answers",
            Self::Reference => "reference works (OEIS, zbMATH, MathWorld, Wikipedia)",
            Self::Notes => "university pages, course notes, and slides",
            Self::Blog => "mathematicians' blogs",
            Self::Other => "other sources",
        }
    }
}

/// Parses a `prefer` argument into classes.
///
/// # Errors
///
/// The first name that is not a source class.
pub(crate) fn parse_classes(names: &[String]) -> std::result::Result<Vec<SourceClass>, String> {
    names
        .iter()
        .map(|name| {
            SourceClass::parse(name).ok_or_else(|| format!("unknown source class {name:?}"))
        })
        .collect()
}

/// Domains per class. A host matches a domain when it equals it or is a
/// subdomain of it. Checked in order, so a more specific class goes first.
pub const CLASS_TABLE: &[(SourceClass, &[&str])] = &[
    (
        SourceClass::Preprint,
        &[
            "arxiv.org",
            "hal.science",
            "hal.archives-ouvertes.fr",
            "eprint.iacr.org",
            "biorxiv.org",
            "osf.io",
        ],
    ),
    (
        SourceClass::Journal,
        &[
            "doi.org",
            "ams.org",
            "springer.com",
            "springerlink.com",
            "sciencedirect.com",
            "projecteuclid.org",
            "wiley.com",
            "tandfonline.com",
            "jstor.org",
            "annalsofmath.org",
            "cambridge.org",
            "oup.com",
            "academic.oup.com",
        ],
    ),
    (
        SourceClass::Qa,
        &[
            "mathoverflow.net",
            "math.stackexchange.com",
            "stackexchange.com",
            "stackoverflow.com",
        ],
    ),
    (
        SourceClass::Reference,
        &[
            "oeis.org",
            "zbmath.org",
            "mathworld.wolfram.com",
            "wikipedia.org",
            "leanprover-community.github.io",
            "ncatlab.org",
            "planetmath.org",
        ],
    ),
    (
        SourceClass::Blog,
        &[
            "wordpress.com",
            "blogspot.com",
            "substack.com",
            "medium.com",
            "tumblr.com",
            "lesswrong.com",
        ],
    ),
];

fn host_matches(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|rest| rest.ends_with('.'))
}

fn is_academic(host: &str) -> bool {
    host.rsplit('.').next() == Some("edu")
        || host.contains(".edu.")
        || host.split('.').skip(1).any(|label| label == "ac")
}

/// Classifies a source by its domain.
///
/// A host in [`CLASS_TABLE`] takes its class; otherwise a `.edu` or `.ac.*`
/// host is `notes`, a host with a `blog` label is `blog`, and anything else
/// is `other`.
#[must_use]
pub fn classify(url: &Url) -> SourceClass {
    let Some(host) = url.host_str() else {
        return SourceClass::Other;
    };
    let host = host.to_ascii_lowercase();
    for (class, domains) in CLASS_TABLE {
        if domains.iter().any(|d| host_matches(&host, d)) {
            return *class;
        }
    }
    if is_academic(&host) {
        SourceClass::Notes
    } else if host.split('.').any(|label| label.starts_with("blog")) {
        SourceClass::Blog
    } else {
        SourceClass::Other
    }
}

/// The arXiv identifier of an `arxiv.org` abstract or PDF URL, without its
/// version suffix.
#[must_use]
pub fn arxiv_id(url: &Url) -> Option<String> {
    let host = url.host_str()?.to_ascii_lowercase();
    if !host_matches(&host, "arxiv.org") {
        return None;
    }
    let path = url.path();
    let rest = ["/abs/", "/pdf/", "/html/"]
        .iter()
        .find_map(|prefix| path.strip_prefix(prefix))?;
    let rest = rest.strip_suffix(".pdf").unwrap_or(rest);
    let id = strip_version(rest.trim_end_matches('/'));
    is_arxiv_id(id).then(|| id.to_owned())
}

fn strip_version(id: &str) -> &str {
    match id.rfind('v') {
        Some(at)
            if at > 0 && id.len() > at + 1 && id[at + 1..].bytes().all(|b| b.is_ascii_digit()) =>
        {
            &id[..at]
        }
        _ => id,
    }
}

fn is_arxiv_id(id: &str) -> bool {
    let digits = |s: &str, n: std::ops::RangeInclusive<usize>| {
        n.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
    };
    // Old style: `math/0309136`, `math.GT/0309136`.
    if let Some((archive, number)) = id.split_once('/') {
        return !archive.is_empty()
            && archive
                .bytes()
                .all(|b| b.is_ascii_alphabetic() || b == b'-' || b == b'.')
            && digits(number, 7..=7);
    }
    id.split_once('.')
        .is_some_and(|(yymm, number)| digits(yymm, 4..=4) && digits(number, 4..=5))
}

/// One resolved source: a real URL, never a grounding redirect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// The resolved URL.
    pub url: String,
    /// The title Google gave it; often just the domain.
    pub title: String,
    /// What kind of place it is.
    pub class: SourceClass,
    /// The arXiv id, for an arXiv source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arxiv_id: Option<String>,
}

impl Source {
    /// Builds a source from a resolved URL and a title, classifying it.
    #[must_use]
    pub fn new(url: &Url, title: impl Into<String>) -> Self {
        Self {
            url: url.as_str().to_owned(),
            title: title.into(),
            class: classify(url),
            arxiv_id: arxiv_id(url),
        }
    }
}

#[cfg(test)]
mod test;

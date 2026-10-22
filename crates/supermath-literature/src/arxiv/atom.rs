//! Parses arXiv's Atom feed into [`Paper`]s.

use quick_xml::Reader;
use quick_xml::events::Event;

use super::types::{ArxivId, Paper};
use crate::error::{Error, Result};

/// A parsed feed.
#[derive(Debug, Default)]
pub(super) struct Feed {
    pub(super) total: u64,
    pub(super) start: u64,
    pub(super) papers: Vec<Paper>,
}

#[derive(Default)]
struct Entry {
    id: String,
    title: String,
    summary: String,
    published: String,
    updated: String,
    authors: Vec<String>,
    primary: String,
    categories: Vec<String>,
}

fn decode(reason: &impl ToString) -> Error {
    Error::Decode {
        what: "the arxiv atom feed".into(),
        reason: reason.to_string(),
    }
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn parse(xml: &str) -> Result<Feed> {
    let mut reader = Reader::from_str(xml);
    let mut feed = Feed::default();
    let mut path: Vec<String> = Vec::new();
    let mut entry: Option<Entry> = None;
    let mut text = String::new();
    loop {
        match reader.read_event().map_err(|e| decode(&e))? {
            Event::Start(e) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                if name == "entry" {
                    entry = Some(Entry::default());
                }
                note_categories(&name, &e, entry.as_mut())?;
                path.push(name);
                text.clear();
            }
            Event::Empty(e) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                note_categories(&name, &e, entry.as_mut())?;
            }
            Event::Text(t) => text.push_str(&t.decode().map_err(|e| decode(&e))?),
            Event::CData(t) => text.push_str(&String::from_utf8_lossy(&t.into_inner())),
            Event::GeneralRef(r) => text.push_str(&resolve(&r)?),
            Event::End(_) => {
                let Some(name) = path.pop() else {
                    return Err(decode(&"unbalanced end tag"));
                };
                let parent = path.last().map(String::as_str);
                match (name.as_str(), parent, entry.as_mut()) {
                    ("entry", _, _) => {
                        if let Some(done) = entry.take() {
                            feed.papers.push(finish(done)?);
                        }
                    }
                    ("id", Some("entry"), Some(e)) => text.trim().clone_into(&mut e.id),
                    ("title", Some("entry"), Some(e)) => e.title = collapse(&text),
                    ("summary", Some("entry"), Some(e)) => e.summary = collapse(&text),
                    ("published", Some("entry"), Some(e)) => {
                        text.trim().clone_into(&mut e.published);
                    }
                    ("updated", Some("entry"), Some(e)) => text.trim().clone_into(&mut e.updated),
                    ("name", Some("author"), Some(e)) => e.authors.push(collapse(&text)),
                    ("opensearch:totalResults", _, None) => {
                        feed.total = text.trim().parse().unwrap_or(0);
                    }
                    ("opensearch:startIndex", _, None) => {
                        feed.start = text.trim().parse().unwrap_or(0);
                    }
                    _ => {}
                }
                text.clear();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(feed)
}

fn note_categories(
    name: &str,
    tag: &quick_xml::events::BytesStart<'_>,
    entry: Option<&mut Entry>,
) -> Result<()> {
    let Some(entry) = entry else {
        return Ok(());
    };
    if name != "arxiv:primary_category" && name != "category" {
        return Ok(());
    }
    for attribute in tag.attributes() {
        let attribute = attribute.map_err(|e| decode(&e))?;
        if attribute.key.as_ref() == b"term" {
            let term = attribute
                .unescape_value()
                .map_err(|e| decode(&e))?
                .into_owned();
            if name == "arxiv:primary_category" {
                entry.primary.clone_from(&term);
            }
            if !entry.categories.contains(&term) {
                entry.categories.push(term);
            }
        }
    }
    Ok(())
}

fn resolve(reference: &quick_xml::events::BytesRef<'_>) -> Result<String> {
    if let Some(c) = reference.resolve_char_ref().map_err(|e| decode(&e))? {
        return Ok(c.to_string());
    }
    let name = reference.decode().map_err(|e| decode(&e))?;
    Ok(quick_xml::escape::resolve_predefined_entity(&name)
        .map_or_else(|| format!("&{name};"), str::to_owned))
}

fn finish(entry: Entry) -> Result<Paper> {
    if entry.id.contains("/api/errors") {
        return Err(Error::InvalidInput(format!(
            "arxiv rejected the request: {}",
            collapse(&entry.summary)
        )));
    }
    let id = ArxivId::parse(
        entry
            .id
            .rsplit_once("/abs/")
            .map_or(entry.id.as_str(), |(_, id)| id),
    )
    .map_err(|_| decode(&format!("entry id {:?} is not an arxiv id", entry.id)))?;
    let mut categories = entry.categories;
    if let Some(i) = categories.iter().position(|c| *c == entry.primary) {
        categories.swap(0, i);
    }
    Ok(Paper {
        arxiv_id: id.base().to_owned(),
        version: id.version().unwrap_or(1),
        title: entry.title,
        authors: entry.authors,
        primary_category: entry.primary,
        categories,
        published: entry.published,
        updated: entry.updated,
        summary: entry.summary,
    })
}

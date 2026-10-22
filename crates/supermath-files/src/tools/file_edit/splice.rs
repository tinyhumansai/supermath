//! Exact-string edits, applied in order, with the lines they changed.

use serde::Deserialize;

/// One replacement: `old` must occur exactly once.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Edit {
    pub(super) old: String,
    pub(super) new: String,
}

/// Why an edit list could not be applied; `index` is the zero-based index
/// of the offending edit.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Fault {
    /// Its `old` string is empty, which names nothing to replace.
    EmptyOld {
        /// The edit.
        index: usize,
    },
    /// Its `old` string occurs `occurrences` times (zero, or several), in
    /// the text as the earlier edits left it.
    NotOnce {
        /// The edit.
        index: usize,
        /// How many times `old` occurs, overlaps included.
        occurrences: usize,
    },
}

/// The edited text and the byte span each edit left behind.
pub(super) struct Applied {
    pub(super) text: String,
    spans: Vec<(usize, usize)>,
}

/// How many times `needle` occurs in `haystack`, overlaps included, and where
/// it first does.
fn occurrences(haystack: &str, needle: &str) -> (usize, Option<usize>) {
    let step = needle.chars().next().map_or(1, char::len_utf8);
    let (mut count, mut first, mut from) = (0, None, 0);
    while !needle.is_empty()
        && let Some(found) = haystack.get(from..).and_then(|rest| rest.find(needle))
    {
        count += 1;
        first = first.or(Some(from + found));
        from += found + step;
    }
    (count, first)
}

/// Applies `edits` in order, all or none.
pub(super) fn apply(original: &str, edits: &[Edit]) -> Result<Applied, Fault> {
    let mut text = original.to_owned();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for (index, edit) in edits.iter().enumerate() {
        if edit.old.is_empty() {
            return Err(Fault::EmptyOld { index });
        }
        let at = match occurrences(&text, &edit.old) {
            (1, Some(at)) => at,
            (count, _) => {
                return Err(Fault::NotOnce {
                    index,
                    occurrences: count,
                });
            }
        };
        let (old_end, new_end) = (at + edit.old.len(), at + edit.new.len());
        text.replace_range(at..old_end, &edit.new);
        for span in &mut spans {
            let (start, end) = *span;
            if end <= at {
                continue;
            }
            let shifted_end = if end >= old_end {
                end - edit.old.len() + edit.new.len()
            } else {
                new_end
            };
            *span = if start >= old_end {
                (start - edit.old.len() + edit.new.len(), shifted_end)
            } else {
                (start.min(at), shifted_end.max(new_end))
            };
        }
        spans.push((at, new_end));
    }
    Ok(Applied { text, spans })
}

impl Applied {
    /// The changed line ranges (one-based, inclusive), sorted and merged.
    pub(super) fn changed_lines(&self) -> Vec<(usize, usize)> {
        let total = self.text.lines().count().max(1);
        let line_of = |byte: usize| (1 + self.text[..byte].matches('\n').count()).min(total);
        // The last line an edit touched; a trailing newline belongs to the
        // line it ends.
        let last_line = |start: usize, end: usize| {
            if end > start {
                let upto = &self.text[..end];
                (1 + upto.matches('\n').count() - usize::from(upto.ends_with('\n'))).min(total)
            } else {
                line_of(start)
            }
        };
        let mut ranges: Vec<(usize, usize)> = self
            .spans
            .iter()
            .map(|&(start, end)| (line_of(start), last_line(start, end)))
            .collect();
        ranges.sort_unstable();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in ranges {
            match merged.last_mut() {
                Some(last) if start <= last.1 + 1 => last.1 = last.1.max(end),
                _ => merged.push((start, end)),
            }
        }
        merged
    }
}

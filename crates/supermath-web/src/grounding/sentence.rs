//! Splitting an answer into sentences with their byte spans.

/// Words that end in a full stop without ending the sentence.
const ABBREVIATIONS: [&str; 9] = ["al", "cf", "vs", "etc", "fig", "thm", "prof", "dr", "eq"];

/// Bullet and heading markers stripped from the start of a sentence.
const MARKERS: [&str; 5] = ["* ", "- ", "\u{2022} ", "# ", "> "];

fn ends_sentence(before: &str, ch: char, next: Option<char>) -> bool {
    if !matches!(ch, '.' | '!' | '?') || next.is_some_and(|n| !n.is_whitespace()) {
        return false;
    }
    if ch != '.' {
        return true;
    }
    let token = before
        .rsplit(char::is_whitespace)
        .next()
        .unwrap_or("")
        .trim_start_matches(['(', '[', '*', '_'])
        .to_ascii_lowercase();
    let trimmed = before.trim();
    let list_number = !trimmed.is_empty() && trimmed.bytes().all(|b| b.is_ascii_digit());
    !(list_number || ABBREVIATIONS.contains(&token.as_str()))
}

/// Splits `text` into sentences and returns each one's trimmed byte span.
///
/// A sentence ends at `.`, `!`, or `?` followed by whitespace, and at a line
/// break. A full stop after an abbreviation such as "et al" or after a list
/// number does not end one. Leading bullet markers are left out of the span.
#[must_use]
pub fn split_sentences(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, ch)) = chars.next() {
        let next = chars.peek().map(|(_, n)| *n);
        let end = if ch == '\n' {
            Some(at)
        } else if ends_sentence(&text[start..at], ch, next) {
            Some(at + ch.len_utf8())
        } else {
            None
        };
        if let Some(end) = end {
            push_trimmed(text, start, end, &mut spans);
            start = at + ch.len_utf8();
        }
    }
    push_trimmed(text, start, text.len(), &mut spans);
    spans
}

fn push_trimmed(text: &str, start: usize, end: usize, spans: &mut Vec<(usize, usize)>) {
    let slice = &text[start..end];
    let lead = slice.len() - slice.trim_start().len();
    let mut begin = start + lead;
    let trimmed = slice.trim();
    let mut body = trimmed;
    for marker in MARKERS {
        if let Some(rest) = body.strip_prefix(marker) {
            begin += marker.len();
            body = rest.trim_start();
            begin += rest.len() - body.len();
            break;
        }
    }
    if body.chars().any(char::is_alphanumeric) {
        spans.push((begin, begin + body.len()));
    }
}

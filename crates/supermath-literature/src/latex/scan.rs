//! Small scanners over LaTeX text. All delimiters are ASCII, so byte indices
//! returned here are always `char` boundaries.

/// Removes `%` comments, keeping each line break.
pub(crate) fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.split_inclusive('\n') {
        let bytes = line.as_bytes();
        let mut i = 0;
        let mut cut = line.len();
        while i < bytes.len() {
            match bytes[i] {
                b'\\' => i += 2,
                b'%' => {
                    cut = i;
                    break;
                }
                _ => i += 1,
            }
        }
        out.push_str(&line[..cut]);
        if cut < line.len() && line.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// The control word starting at `at` (which must index a backslash): its
/// letters and the index just past them. A control symbol such as `\\` or
/// `\%` has an empty name.
pub(crate) fn control_word(text: &str, at: usize) -> (&str, usize) {
    let start = at + 1;
    let len = text[start..]
        .bytes()
        .take_while(u8::is_ascii_alphabetic)
        .count();
    (&text[start..start + len], start + len)
}

/// Where to resume after the command at `at` whose control word ended at
/// `end`: past the word, or for a control symbol such as `\\` or `\%` past
/// the symbol, so its second character is never read as a new command.
pub(crate) fn past_command(text: &str, at: usize, end: usize) -> usize {
    if end == at + 1 {
        at + 1 + text[at + 1..].chars().next().map_or(0, char::len_utf8)
    } else {
        end
    }
}

/// The index of the first non-whitespace byte at or after `i`.
pub(crate) fn skip_ws(text: &str, mut i: usize) -> usize {
    let bytes = text.as_bytes();
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Reads a balanced `open ... close` group whose opener is at `start`.
/// Returns the inside and the index just past the closer.
pub(crate) fn read_group(text: &str, start: usize, open: u8, close: u8) -> Option<(&str, usize)> {
    let bytes = text.as_bytes();
    if bytes.get(start) != Some(&open) {
        return None;
    }
    let mut depth = 0usize;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b if b == open => depth += 1,
            b if b == close => {
                depth -= 1;
                if depth == 0 {
                    return Some((&text[start + 1..i], i + 1));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// A `{...}` argument at or after `i`, skipping whitespace.
pub(crate) fn brace_arg(text: &str, i: usize) -> Option<(&str, usize)> {
    read_group(text, skip_ws(text, i), b'{', b'}')
}

/// A `[...]` argument at or after `i`, skipping whitespace, if there is one.
pub(crate) fn bracket_arg(text: &str, i: usize) -> Option<(&str, usize)> {
    read_group(text, skip_ws(text, i), b'[', b']')
}

/// The index of the next `\end{env}` at or after `from`.
pub(crate) fn find_end(text: &str, from: usize, env: &str) -> Option<usize> {
    text[from..]
        .find(&format!("\\end{{{env}}}"))
        .map(|p| from + p)
}

/// Every `\name{...}` argument in `text`, for the commands in `names`.
pub(crate) fn command_args<'a>(text: &'a str, names: &[&str]) -> Vec<&'a str> {
    let mut found = Vec::new();
    let mut i = 0;
    while let Some(p) = text[i..].find('\\') {
        let at = i + p;
        let (name, end) = control_word(text, at);
        i = past_command(text, at, end);
        if names.contains(&name) {
            let mut j = end;
            if text[j..].starts_with('*') {
                j += 1;
            }
            if let Some((arg, after)) = brace_arg(text, j) {
                found.push(arg);
                i = after;
            }
        }
    }
    found
}

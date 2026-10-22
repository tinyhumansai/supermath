//! Reading the import header of a Lean file.

/// Removes `--` line comments and (nested) `/- -/` block comments, keeping
/// every newline so line numbers survive.
pub(crate) fn strip_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let (mut i, mut depth) = (0, 0usize);
    while i < bytes.len() {
        let rest = &bytes[i..];
        if rest.starts_with(b"/-") {
            depth += 1;
            i += 2;
        } else if depth > 0 && rest.starts_with(b"-/") {
            depth -= 1;
            i += 2;
        } else if depth == 0 && rest.starts_with(b"--") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else {
            if depth == 0 || bytes[i] == b'\n' {
                out.push(bytes[i]);
            }
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The modules named by the file's `import` header, in order.
///
/// The header is the run of `import` commands (with optional `prelude` and
/// `public`/`meta`/`private`/`all` modifiers) at the top of the file; the
/// first other token ends it, exactly as Lean's own parser does.
pub(super) fn header_imports(source: &str) -> Vec<String> {
    let cleaned = strip_comments(source);
    let mut tokens = cleaned.split_whitespace().peekable();
    if tokens.peek() == Some(&"prelude") {
        tokens.next();
    }
    let mut modules = Vec::new();
    while let Some(token) = tokens.peek().copied() {
        match token {
            "public" | "meta" | "private" | "all" => {
                tokens.next();
            }
            "import" => {
                tokens.next();
                match tokens.next() {
                    Some(module) => modules.push(module.to_owned()),
                    None => break,
                }
            }
            _ => break,
        }
    }
    modules
}

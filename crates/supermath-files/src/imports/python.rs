//! Reading the import statements of a Python file.

use super::Import;

/// Every `import` and `from … import` statement in the file, at any
/// indentation, skipping comments and triple-quoted strings.
pub(super) fn statements(source: &str) -> Vec<Import> {
    let mut found: Vec<Import> = Vec::new();
    let mut inside: Option<&str> = None;
    // Set while a parenthesised `from … import (` list continues.
    let mut continuing = false;
    for line in source.lines() {
        if let Some(delimiter) = inside {
            if line.matches(delimiter).count() % 2 == 1 {
                inside = None;
            }
            continue;
        }
        let code = line.split('#').next().unwrap_or_default();
        if continuing {
            if let Some(last) = found.last_mut() {
                last.names.extend(names(code));
            }
            continuing = !code.contains(')');
        } else {
            for statement in code.split(';') {
                let parsed = statement_imports(statement.trim());
                if !parsed.is_empty() {
                    continuing = statement.contains('(') && !statement.contains(')');
                    found.extend(parsed);
                }
            }
        }
        inside = ["\"\"\"", "'''"]
            .into_iter()
            .find(|d| line.matches(d).count() % 2 == 1);
    }
    found
}

fn statement_imports(statement: &str) -> Vec<Import> {
    if let Some(rest) = statement.strip_prefix("import ") {
        return rest
            .split(',')
            .filter_map(|m| m.split_whitespace().next())
            .map(|module| Import {
                module: module.to_owned(),
                names: Vec::new(),
            })
            .collect();
    }
    let Some((module, imported)) = statement
        .strip_prefix("from ")
        .and_then(|rest| rest.split_once(" import"))
    else {
        return Vec::new();
    };
    let module = module.trim();
    let well_formed = !module.is_empty()
        && module
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.');
    if well_formed {
        vec![Import {
            module: module.to_owned(),
            names: names(imported),
        }]
    } else {
        Vec::new()
    }
}

fn names(list: &str) -> Vec<String> {
    list.replace(['(', ')'], " ")
        .split(',')
        .filter_map(|n| n.split_whitespace().next())
        .filter(|n| *n != "*" && n.chars().all(|c| c.is_alphanumeric() || c == '_'))
        .map(str::to_owned)
        .collect()
}

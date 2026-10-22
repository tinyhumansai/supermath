//! A small `robots.txt` reader.
//!
//! It finds the group that names this crawler (or `*`), and answers whether
//! a path is allowed by the longest matching rule, an `Allow` winning a tie,
//! with `*` wildcards and a `$` end anchor.

/// The rules that apply to this crawler.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Robots {
    rules: Vec<(bool, String)>,
}

impl Robots {
    /// Reads `body`, keeping the rules of the group for `agent`, or for `*`
    /// when no group names `agent`.
    pub(crate) fn parse(body: &str, agent: &str) -> Self {
        let agent = agent.to_ascii_lowercase();
        let mut named = Vec::new();
        let mut wildcard = Vec::new();
        // The agents of the group being read, and whether it has rules yet.
        let mut group: Vec<String> = Vec::new();
        let mut in_rules = false;
        for line in body.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((field, value)) = line.split_once(':') else {
                continue;
            };
            let (field, value) = (field.trim().to_ascii_lowercase(), value.trim());
            match field.as_str() {
                "user-agent" => {
                    if in_rules {
                        group.clear();
                        in_rules = false;
                    }
                    group.push(value.to_ascii_lowercase());
                }
                "allow" | "disallow" => {
                    in_rules = true;
                    if value.is_empty() {
                        continue;
                    }
                    let rule = (field == "allow", value.to_owned());
                    if group.iter().any(|g| g != "*" && agent.contains(g.as_str())) {
                        named.push(rule);
                    } else if group.iter().any(|g| g == "*") {
                        wildcard.push(rule);
                    }
                }
                _ => {}
            }
        }
        Self {
            rules: if named.is_empty() { wildcard } else { named },
        }
    }

    /// Whether `path` (with its query) may be fetched.
    pub(crate) fn allows(&self, path: &str) -> bool {
        self.rules
            .iter()
            .filter(|(_, pattern)| matches(pattern, path))
            .max_by_key(|(allow, pattern)| (pattern.len(), *allow))
            .is_none_or(|(allow, _)| *allow)
    }
}

fn matches(pattern: &str, path: &str) -> bool {
    let (pattern, anchored) = match pattern.strip_suffix('$') {
        Some(rest) => (rest, true),
        None => (pattern, false),
    };
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or("");
    let Some(mut rest) = path.strip_prefix(first) else {
        return false;
    };
    let tail: Vec<&str> = parts.collect();
    for (i, part) in tail.iter().enumerate() {
        let last = i + 1 == tail.len();
        if last && anchored {
            return rest.ends_with(part);
        }
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    !anchored || rest.is_empty()
}

#[cfg(test)]
mod test;

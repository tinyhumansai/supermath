//! Tests for the `robots.txt` reader.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::Robots;

const BODY: &str = "\
# comment
User-agent: *
Disallow: /private
Allow: /private/open
Disallow: /*.pdf$
Disallow:

User-agent: supermath
User-agent: otherbot
Disallow: /only-us
";

#[test]
fn a_group_naming_us_wins_over_the_wildcard() {
    let robots = Robots::parse(BODY, "supermath/0.1");
    assert!(!robots.allows("/only-us/x"));
    assert!(robots.allows("/private/data"));
}

#[test]
fn the_wildcard_group_applies_to_everyone_else() {
    let robots = Robots::parse(BODY, "somebot");
    assert!(!robots.allows("/private/data"));
    assert!(robots.allows("/private/open/page"));
    assert!(robots.allows("/public"));
    assert!(!robots.allows("/papers/a.pdf"));
    assert!(robots.allows("/papers/a.pdf.html"));
    assert!(robots.allows("/only-us"));
}

#[test]
fn an_empty_or_missing_file_allows_everything() {
    assert!(Robots::parse("", "x").allows("/anything"));
    assert!(Robots::parse("garbage line\nUser-agent: *", "x").allows("/anything"));
    assert!(Robots::parse("User-agent: *\nDisallow:", "x").allows("/anything"));
}

#[test]
fn allow_wins_a_tie_and_wildcards_match_in_the_middle() {
    let robots = Robots::parse(
        "User-agent: *\nDisallow: /a\nAllow: /a\nDisallow: /x*/y",
        "z",
    );
    assert!(robots.allows("/a"));
    assert!(!robots.allows("/xfoo/y"));
    assert!(robots.allows("/xfoo/z"));
    assert!(!Robots::parse("User-agent: *\nDisallow: /", "z").allows("/page"));
}

//! Tests for identifiers, the Atom parser, and query building.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::atom::parse;
use super::{ArxivId, SearchQuery, date_stamp, search_expression};
use crate::Error;

#[test]
fn identifiers_parse_in_every_form_arxiv_uses() {
    let cases = [
        ("2310.01234", "2310.01234", None),
        ("2310.01234v2", "2310.01234", Some(2)),
        ("  arXiv:2310.01234v10 ", "2310.01234", Some(10)),
        ("ARXIV:0704.0001", "0704.0001", None),
        ("https://arxiv.org/abs/2310.01234v3", "2310.01234", Some(3)),
        ("http://arxiv.org/abs/2310.01234", "2310.01234", None),
        (
            "https://arxiv.org/pdf/2310.01234v1.pdf",
            "2310.01234",
            Some(1),
        ),
        ("arxiv.org/pdf/2310.01234", "2310.01234", None),
        ("math/0301234", "math/0301234", None),
        ("math.CV/0301234v2", "math.CV/0301234", Some(2)),
        ("hep-th/9901001", "hep-th/9901001", None),
    ];
    for (raw, base, version) in cases {
        let id = ArxivId::parse(raw).unwrap();
        assert_eq!((id.base(), id.version()), (base, version), "{raw}");
    }
}

#[test]
fn malformed_identifiers_are_invalid_input() {
    for raw in [
        "",
        "v2",
        "2310",
        "2310.012",
        "2310.0123456",
        "23100.01234",
        "231a.01234",
        "2310.01234v0",
        "2310.01234vx",
        "math/123456",
        "Math/0301234",
        "math.cv/0301234",
        "math.C/0301234",
        "/0301234",
        "math/03012a4",
        "2310.01234 v2",
    ] {
        assert!(
            matches!(ArxivId::parse(raw), Err(Error::InvalidInput(_))),
            "{raw:?}"
        );
    }
}

#[test]
fn an_identifier_prints_with_its_version_and_can_be_pinned() {
    let id = ArxivId::parse("2310.01234").unwrap();
    assert_eq!(id.to_string(), "2310.01234");
    assert_eq!(id.with_version(4).to_string(), "2310.01234v4");
}

const FEED: &str = r#"<?xml version="1.0"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>feed title ignored</title>
  <opensearch:totalResults>7</opensearch:totalResults>
  <opensearch:startIndex>2</opensearch:startIndex>
  <entry>
    <id>http://arxiv.org/abs/2310.01234v3</id>
    <title>  A &lt;b&gt; title
      with &#955; &nbsp; and &amp; more </title>
    <summary>S</summary>
    <published>2023-10-02T00:00:00Z</published><updated>2023-11-02T00:00:00Z</updated>
    <author><name> A  B </name><arxiv:affiliation>Somewhere</arxiv:affiliation></author>
    <category term="math.NT"/>
    <arxiv:primary_category term="math.CO"/>
    <category term="math.NT"/>
    <link href="x"/>
  </entry>
  <entry><id>http://arxiv.org/abs/hep-th/9901001</id><title>Old</title></entry>
</feed>"#;

#[test]
fn the_atom_feed_parses_into_papers() {
    let feed = parse(FEED).unwrap();
    assert_eq!((feed.total, feed.start), (7, 2));
    assert_eq!(feed.papers.len(), 2);
    let paper = &feed.papers[0];
    assert_eq!(paper.arxiv_id, "2310.01234");
    assert_eq!(paper.version, 3);
    assert_eq!(paper.title, "A <b> title with λ &nbsp; and & more");
    assert_eq!(paper.authors, ["A B"]);
    assert_eq!(paper.primary_category, "math.CO");
    assert_eq!(paper.categories, ["math.CO", "math.NT"]);
    assert_eq!(paper.published, "2023-10-02T00:00:00Z");
    assert_eq!(paper.updated, "2023-11-02T00:00:00Z");
    let old = &feed.papers[1];
    assert_eq!((old.arxiv_id.as_str(), old.version), ("hep-th/9901001", 1));
}

#[test]
fn bad_feeds_are_decode_errors_and_error_entries_are_invalid_input() {
    for xml in [
        "</feed>",
        "<feed><entry></feed>",
        "<feed><entry><id>not an id</id></entry></feed>",
        "<feed>&#xZZ;</feed>",
        r#"<feed><entry><category term="a" term="b"/></entry></feed>"#,
    ] {
        assert!(matches!(parse(xml), Err(Error::Decode { .. })), "{xml}");
    }
    let error = "<feed><entry><id>http://arxiv.org/api/errors#x</id><summary>bad  input</summary></entry></feed>";
    assert!(matches!(parse(error), Err(Error::InvalidInput(m)) if m.ends_with("bad input")));
}

#[test]
fn an_empty_feed_has_no_papers_and_unparsable_counts_are_zero() {
    let feed =
        parse("<feed><opensearch:totalResults>many</opensearch:totalResults></feed>").unwrap();
    assert_eq!((feed.total, feed.start, feed.papers.len()), (0, 0, 0));
}

fn query() -> SearchQuery {
    SearchQuery::default()
}

#[test]
fn a_search_expression_ands_every_clause() {
    let q = SearchQuery {
        text: Some(r#"  "Sendov" conjecture, (roots)! "#.into()),
        title: Some("critical points".into()),
        author: Some("O'Neil".into()),
        category: Some("math.CV".into()),
        since: Some("2015-01-31".into()),
        ..query()
    };
    assert_eq!(
        search_expression(&q).unwrap(),
        "all:Sendov AND all:conjecture AND all:roots AND ti:critical AND ti:points AND au:O'Neil \
         AND cat:math.CV AND submittedDate:[201501310000 TO 209912312359]"
    );
}

#[test]
fn a_search_needs_something_to_search_for_and_a_sane_category_and_date() {
    assert!(matches!(
        search_expression(&query()),
        Err(Error::InvalidInput(_))
    ));
    let blank = SearchQuery {
        text: Some(" !! ".into()),
        ..query()
    };
    assert!(matches!(
        search_expression(&blank),
        Err(Error::InvalidInput(_))
    ));
    for category in ["", "math cv", "math;DROP"] {
        let q = SearchQuery {
            text: Some("x".into()),
            category: Some(category.into()),
            ..query()
        };
        assert!(search_expression(&q).is_err(), "{category:?}");
    }
    for since in [
        "2015",
        "15-01-01",
        "2015-1-01",
        "2015-00-10",
        "2015-13-10",
        "2015-10-32",
        "2015-10-00",
        "20x5-01-01",
        "2015-01-0a",
    ] {
        assert!(date_stamp(since).is_err(), "{since:?}");
    }
    assert_eq!(date_stamp("1999-12-31").unwrap(), "199912310000");
}

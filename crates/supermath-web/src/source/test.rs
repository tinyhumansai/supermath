//! A test per source class, and the arXiv id forms.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use url::Url;

use super::{CLASS_TABLE, Source, SourceClass, arxiv_id, classify};

fn class_of(url: &str) -> SourceClass {
    classify(&Url::parse(url).unwrap())
}

#[test]
fn preprint_domains() {
    for url in [
        "https://arxiv.org/abs/1908.08483",
        "https://export.arxiv.org/abs/1908.08483",
        "https://hal.science/hal-01234567",
        "https://eprint.iacr.org/2020/001",
    ] {
        assert_eq!(class_of(url), SourceClass::Preprint, "{url}");
    }
}

#[test]
fn journal_domains() {
    for url in [
        "https://doi.org/10.4007/annals.2020.1",
        "https://www.ams.org/journals/jams/2020",
        "https://link.springer.com/article/10.1007/x",
        "https://www.sciencedirect.com/science/article/pii/S0",
        "https://projecteuclid.org/journals/x",
    ] {
        assert_eq!(class_of(url), SourceClass::Journal, "{url}");
    }
}

#[test]
fn qa_domains() {
    for url in [
        "https://mathoverflow.net/questions/12345",
        "https://math.stackexchange.com/questions/1",
    ] {
        assert_eq!(class_of(url), SourceClass::Qa, "{url}");
    }
}

#[test]
fn reference_domains() {
    for url in [
        "https://oeis.org/A000045",
        "https://zbmath.org/?q=an:1",
        "https://mathworld.wolfram.com/Sunflower.html",
        "https://en.wikipedia.org/wiki/Sunflower",
        "https://leanprover-community.github.io/mathlib4_docs/",
    ] {
        assert_eq!(class_of(url), SourceClass::Reference, "{url}");
    }
}

#[test]
fn notes_domains() {
    for url in [
        "https://math.mit.edu/~x/notes.pdf",
        "https://www.maths.ox.ac.uk/people/x",
        "https://www.unimelb.edu.au/maths",
    ] {
        assert_eq!(class_of(url), SourceClass::Notes, "{url}");
    }
}

#[test]
fn blog_domains() {
    for url in [
        "https://terrytao.wordpress.com/2019/10/01/x/",
        "https://gilkalai.wordpress.com/2020/01/01/y/",
        "https://example.blogspot.com/p",
        "https://blog.example.com/post",
    ] {
        assert_eq!(class_of(url), SourceClass::Blog, "{url}");
    }
}

#[test]
fn everything_else_is_other_and_lookalikes_do_not_match() {
    for url in [
        "https://example.com/page",
        "https://notarxiv.org/abs/1908.08483",
        "https://arxiv.org.evil.example/abs/1",
        "https://education.example.com/",
        "mailto:someone@example.com",
    ] {
        assert_eq!(class_of(url), SourceClass::Other, "{url}");
    }
}

#[test]
fn every_class_in_the_table_round_trips_its_name() {
    for (class, domains) in CLASS_TABLE {
        assert_ne!(domains.len(), 0);
        assert_eq!(SourceClass::parse(class.as_str()), Some(*class));
    }
    assert_eq!(SourceClass::parse("notes"), Some(SourceClass::Notes));
    assert_eq!(SourceClass::parse("other"), Some(SourceClass::Other));
    assert_eq!(SourceClass::parse("podcast"), None);
}

#[test]
fn arxiv_ids_in_every_url_form() {
    let id = |u: &str| arxiv_id(&Url::parse(u).unwrap());
    assert_eq!(
        id("https://arxiv.org/abs/1908.08483").as_deref(),
        Some("1908.08483")
    );
    assert_eq!(
        id("https://arxiv.org/abs/1908.08483v3").as_deref(),
        Some("1908.08483")
    );
    assert_eq!(
        id("https://arxiv.org/pdf/1908.08483v2.pdf").as_deref(),
        Some("1908.08483")
    );
    assert_eq!(
        id("https://arxiv.org/abs/math/0309136").as_deref(),
        Some("math/0309136")
    );
    assert_eq!(
        id("https://arxiv.org/abs/math.GT/0309136v1").as_deref(),
        Some("math.GT/0309136")
    );
    assert_eq!(
        id("https://arxiv.org/abs/hep-th/9901001").as_deref(),
        Some("hep-th/9901001")
    );
    assert_eq!(id("https://arxiv.org/list/math/new"), None);
    assert_eq!(id("https://arxiv.org/abs/not-an-id"), None);
    assert_eq!(id("https://example.com/abs/1908.08483"), None);
    assert_eq!(id("https://arxiv.org/abs/12.1"), None);
    assert_eq!(id("https://arxiv.org/abs/v"), None);
}

#[test]
fn a_source_carries_class_and_arxiv_id_on_the_wire() {
    let url = Url::parse("https://arxiv.org/abs/1908.08483").unwrap();
    let source = Source::new(&url, "Improved bounds");
    assert_eq!(
        serde_json::to_value(&source).unwrap(),
        serde_json::json!({
            "url": "https://arxiv.org/abs/1908.08483",
            "title": "Improved bounds",
            "class": "preprint",
            "arxiv_id": "1908.08483"
        })
    );
    let plain = Source::new(&Url::parse("https://example.com/").unwrap(), "x");
    assert!(
        serde_json::to_value(&plain)
            .unwrap()
            .get("arxiv_id")
            .is_none()
    );
}

//! Unit tests for reading a verdict from an agent reply.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Reason, Verdict};

#[test]
fn reads_each_known_verdict_from_the_last_line() {
    assert_eq!(Verdict::parse("Proof...\nVERDICT: proved"), Verdict::Proved);
    assert_eq!(
        Verdict::parse("n = 7 fails\nVERDICT: refuted"),
        Verdict::Refuted
    );
    assert_eq!(
        Verdict::parse("stuck\nVERDICT: not-established"),
        Verdict::NotEstablished(Reason::Stated)
    );
    assert_eq!(
        Verdict::parse("stuck\nverdict: Not Established."),
        Verdict::NotEstablished(Reason::Stated)
    );
}

#[test]
fn ignores_trailing_blank_lines_and_markdown_emphasis() {
    assert_eq!(
        Verdict::parse("x\n**VERDICT: Proved**\n\n  \n"),
        Verdict::Proved
    );
    assert_eq!(Verdict::parse("`VERDICT: refuted`"), Verdict::Refuted);
}

#[test]
fn a_reply_without_a_verdict_line_is_not_established() {
    for reply in ["", "   \n", "I think it is true.", "VERDICT", "Proved."] {
        assert_eq!(
            Verdict::parse(reply),
            Verdict::NotEstablished(Reason::NoVerdict),
            "{reply:?}"
        );
    }
}

#[test]
fn a_verdict_line_that_is_not_last_does_not_count() {
    assert_eq!(
        Verdict::parse("VERDICT: proved\nActually, wait."),
        Verdict::NotEstablished(Reason::NoVerdict)
    );
}

#[test]
fn an_unknown_verdict_word_is_never_read_as_positive() {
    assert_eq!(
        Verdict::parse("VERDICT: probably"),
        Verdict::NotEstablished(Reason::UnknownVerdict("probably".into()))
    );
}

#[test]
fn labels_and_display_name_the_outcome() {
    assert_eq!(Verdict::Proved.label(), "proved");
    assert_eq!(Verdict::Refuted.to_string(), "refuted");
    assert_eq!(
        Verdict::NotEstablished(Reason::Stated).label(),
        "not-established"
    );
    assert_eq!(
        Verdict::NotEstablished(Reason::Stated).to_string(),
        "not established"
    );
    assert!(
        Verdict::NotEstablished(Reason::NoVerdict)
            .to_string()
            .contains("no verdict line")
    );
    assert!(
        Verdict::NotEstablished(Reason::UnknownVerdict("maybe".into()))
            .to_string()
            .contains("`maybe`")
    );
}

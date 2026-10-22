//! Tests for readable-text extraction.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{MathKind, extract_main_text};

#[test]
fn keeps_structure_and_drops_scripts_navigation_and_hidden_elements() {
    let html = r#"<html><head><title> A  Page </title><style>p{}</style></head><body>
        <nav>Menu Home</nav>
        <script>alert("x")</script>
        <!-- a comment -->
        <h1>Heading</h1>
        <p>First   paragraph with <a href="https://evil.example/">a link</a> and <code>code</code>.</p>
        <p hidden>Hidden paragraph</p>
        <p style="display: none">Also hidden</p>
        <div aria-hidden="true">Decor</div>
        <ul><li>one</li><li>two</li></ul>
        <pre>line 1
  line 2</pre>
        <blockquote><p>Quoted words</p></blockquote>
        <table><tr><td>a</td><td>b</td></tr></table>
        <p>Line<br>break</p>
        <footer>Copyright</footer>
    </body></html>"#;
    let page = extract_main_text(html);
    assert_eq!(page.title, "A Page");
    assert_eq!(page.math, MathKind::None);
    for gone in [
        "Menu",
        "alert",
        "comment",
        "Hidden",
        "Also hidden",
        "Decor",
        "Copyright",
        "evil.example",
    ] {
        assert!(!page.text.contains(gone), "{gone} survived: {}", page.text);
    }
    for kept in [
        "# Heading",
        "First paragraph with a link and `code`.",
        "- one\n- two",
        "```\nline 1\n  line 2\n```",
        "> Quoted words",
        "a | b |",
        "Line\nbreak",
    ] {
        assert!(
            page.text.contains(kept),
            "missing {kept:?} in:\n{}",
            page.text
        );
    }
}

#[test]
fn prefers_main_content_and_falls_back_to_the_first_heading_for_a_title() {
    let page = extract_main_text(
        "<body><div>sidebar</div><main><h1>Real Title</h1><p>Body</p></main></body>",
    );
    assert_eq!(page.title, "Real Title");
    assert!(page.text.contains("Body"));
    assert!(!page.text.contains("sidebar"));
    let page = extract_main_text("<article><p>In article</p></article><p>outside</p>");
    assert!(page.text.contains("In article") && !page.text.contains("outside"));
    assert_eq!(extract_main_text("").text, "");
}

#[test]
fn returns_tex_from_katex_mathjax_and_mathml() {
    let html = r#"<body>
        <p>Inline <span class="katex"><span class="katex-mathml"><math><semantics><mrow><mi>x</mi></mrow>
        <annotation encoding="application/x-tex">x^2</annotation></semantics></math></span>
        <span class="katex-html" aria-hidden="true">x2</span></span> here.</p>
        <p><script type="math/tex">a+b</script> and <script type="math/tex; mode=display">\int f</script></p>
        <p><math><mi>y</mi><annotation encoding="application/x-tex">y_1</annotation></math></p>
        <span class="MathJax_Preview">preview junk</span>
        </body>"#;
    let page = extract_main_text(html);
    assert_eq!(page.math, MathKind::Tex);
    assert!(page.text.contains("Inline $x^2$ here."), "{}", page.text);
    assert!(page.text.contains("$a+b$"));
    assert!(page.text.contains("$$\\int f$$"));
    assert!(page.text.contains("$y_1$"));
    assert!(!page.text.contains("preview junk") && !page.text.contains("x2"));
}

#[test]
fn reports_rendered_math_when_the_source_is_lost() {
    let page = extract_main_text(
        r#"<body><p>See <span class="MathJax">x&#178;</span> and <math display="block"><mi>z</mi></math>.</p></body>"#,
    );
    assert_eq!(page.math, MathKind::Rendered);
    assert!(page.text.contains("x²"));
}

#[test]
fn stack_exchange_answers_come_back_by_score_with_acceptance() {
    let html = r#"<html><head><title>Is X true? - MathOverflow</title></head><body>
        <div class="question"><div class="js-post-body"><p>Is X true?</p></div></div>
        <div class="answer js-answer" data-score="3"><div class="js-post-body"><p>Low answer.</p></div></div>
        <div class="answer js-answer accepted-answer" data-score="12"><div class="post-text"><p>Top answer.</p></div></div>
        <div class="answer" data-score="junk"><p>Unscored answer.</p></div>
        </body></html>"#;
    let page = extract_main_text(html);
    let top = page.text.find("Top answer.").unwrap();
    let low = page.text.find("Low answer.").unwrap();
    assert!(top < low);
    assert!(page.text.contains("# Is X true? - MathOverflow"));
    assert!(page.text.contains("## Question\n\nIs X true?"));
    assert!(page.text.contains("## Answer (score 12, accepted)"));
    assert!(page.text.contains("## Answer (score 3)"));
    assert!(page.text.contains("## Answer (score 0)"));
}

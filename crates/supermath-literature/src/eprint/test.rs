//! Tests for unpacking e-prints.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;

use super::{Source, unpack};

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

fn tarball(files: &[(&str, &str)]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (path, text) in files {
        let mut header = tar::Header::new_ustar();
        header.set_size(text.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, path, text.as_bytes())
            .unwrap();
    }
    builder.into_inner().unwrap()
}

fn latex(source: Source) -> crate::SourceTree {
    match source {
        Source::Latex(tree) => tree,
        Source::PdfOnly => panic!("expected LaTeX"),
    }
}

#[test]
fn a_pdf_is_pdf_only_bare_or_gzipped() {
    assert_eq!(unpack(b"%PDF-1.5 ...").unwrap(), Source::PdfOnly);
    assert_eq!(unpack(&gzip(b"%PDF-1.5 ...")).unwrap(), Source::PdfOnly);
}

#[test]
fn a_gzipped_single_file_is_main_tex() {
    let tree = latex(unpack(&gzip(b"\\documentclass{article}")).unwrap());
    assert_eq!(tree.paths(), ["main.tex"]);
}

#[test]
fn a_gzipped_tarball_keeps_only_tex_files_with_normalised_paths() {
    let tar = tarball(&[
        ("./paper/main.tex", "\\input{a}"),
        ("paper/A.TEX", "upper"),
        ("paper/fig.png", "not tex"),
        ("paper/x.ltx", "ltx"),
    ]);
    let tree = latex(unpack(&gzip(&tar)).unwrap());
    assert_eq!(
        tree.paths(),
        ["paper/A.TEX", "paper/main.tex", "paper/x.ltx"]
    );
}

#[test]
fn a_bare_tarball_is_read_too() {
    let tree = latex(unpack(&tarball(&[("main.tex", "hi")])).unwrap());
    assert_eq!(tree.get("main.tex"), Some("hi"));
}

#[test]
fn a_tarball_without_tex_files_is_pdf_only() {
    assert_eq!(
        unpack(&tarball(&[("paper.pdf", "%PDF")])).unwrap(),
        Source::PdfOnly
    );
}

#[test]
fn plain_text_is_one_tex_file() {
    let tree = latex(unpack("caf\u{e9} \\section{A}".as_bytes()).unwrap());
    assert!(tree.get("main.tex").unwrap().contains("section"));
    // Latin-1 bytes are not UTF-8 but are still text.
    assert!(matches!(unpack(b"caf\xe9").unwrap(), Source::Latex(_)));
}

#[test]
fn binary_junk_and_corrupt_archives_are_decode_errors() {
    for bytes in [&[0u8, 1, 2, 3][..], &[][..], &[0x1f, 0x8b, 9, 9, 9][..]] {
        assert!(matches!(unpack(bytes), Err(crate::Error::Decode { .. })));
    }
}

#[test]
fn a_truncated_tarball_is_a_decode_error() {
    let mut tar = tarball(&[("main.tex", &"x".repeat(2000))]);
    tar.truncate(700);
    assert!(matches!(unpack(&tar), Err(crate::Error::Decode { .. })));
}

#[test]
fn the_recorded_fixture_unpacks_to_its_four_files() {
    let bytes = include_bytes!("../../tests/fixtures/eprint_2401.00001v2.tar.gz");
    let tree = latex(unpack(bytes).unwrap());
    assert_eq!(
        tree.paths(),
        [
            "main.tex",
            "sections/intro.tex",
            "sections/main.tex",
            "sections/sub/more.tex"
        ]
    );
}

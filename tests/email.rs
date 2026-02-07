mod common;

use crate::common::assert_linked_with;
use linkify::LinkFinder;
use linkify::LinkKind;

#[test]
fn no_links() {
    assert_not_linked("");
    assert_not_linked("foo");
    assert_not_linked("@");
    assert_not_linked("a@");
    assert_not_linked("@a");
    assert_not_linked("@@@");
}

#[test]
fn simple() {
    assert_linked("foo@example.com", "|foo@example.com|");
    assert_linked("foo.bar@example.com", "|foo.bar@example.com|");
}

#[test]
fn allowed_text() {
    // I know, I know...
    assert_linked(
        "#!$%&'*+-/=?^_`{}|~@example.org",
        "|#!$%&'*+-/=?^_`{}|~@example.org|",
    );
}

#[test]
fn space_separation() {
    assert_linked("foo a@b.com", "foo |a@b.com|");
    assert_linked("a@b.com foo", "|a@b.com| foo");
    assert_linked("\na@b.com", "\n|a@b.com|");
    assert_linked("a@b.com\n", "|a@b.com|\n");
}

#[test]
fn special_separation() {
    assert_linked("(a@example.com)", "(|a@example.com|)");
    assert_linked("\"a@example.com\"", "\"|a@example.com|\"");
    assert_linked("\"a@example.com\"", "\"|a@example.com|\"");
    assert_linked(",a@example.com,", ",|a@example.com|,");
    assert_linked(":a@example.com:", ":|a@example.com|:");
    assert_linked(";a@example.com;", ";|a@example.com|;");
}

#[test]
fn dots() {
    assert_not_linked(".@example.com");
    assert_not_linked("foo.@example.com");
    assert_linked(".foo@example.com", ".|foo@example.com|");
    assert_linked(".foo@example.com", ".|foo@example.com|");
    assert_linked("a..b@example.com", "a..|b@example.com|");
    assert_linked("a@example.com.", "|a@example.com|.");
}

#[test]
fn domain_without_dot() {
    assert_not_linked("a@b");
    assert_not_linked("a@b.");
    assert_linked("a@b.com.", "|a@b.com|.");
}

#[test]
fn dashes() {
    assert_linked("a@example.com-", "|a@example.com|-");
    assert_linked("a@foo-bar.com", "|a@foo-bar.com|");
    assert_not_linked("a@-foo.com");
    assert_not_linked("a@b-.");
}

#[test]
fn domain_must_have_dot_false() {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Email]);
    finder.email_domain_must_have_dot(false);

    assert_linked_with(&finder, "a@b", "|a@b|");
    assert_linked_with(&finder, "a@b.", "|a@b|.");

    assert_linked_with(&finder, "a@b.", "|a@b|.");
}

#[test]
fn multiple() {
    assert_linked(
        "a@example.com b@example.com",
        "|a@example.com| |b@example.com|",
    );
    assert_linked(
        "a@example.com @ b@example.com",
        "|a@example.com| @ |b@example.com|",
    );
}

#[test]
fn multiple_delimited_hard() {
    assert_linked(
        "a@xy.com;b@xy.com,c@xy.com",
        "|a@xy.com|;|b@xy.com|,|c@xy.com|",
    );
}

#[test]
fn international() {
    assert_linked("üñîçøðé@example.com", "|üñîçøðé@example.com|");
    assert_linked("üñîçøðé@üñîçøðé.com", "|üñîçøðé@üñîçøðé.com|");
}

#[test]
fn trigger_overlap() {
    let finder = LinkFinder::new();

    // 'w' is a trigger character for WWW links. Make sure we can rewind enough.
    assert_linked_with(&finder, "www@example.com", "|www@example.com|");
}

#[test]
fn fuzz() {
    assert_linked("a@a.xyϸ", "|a@a.xyϸ|");
}

#[test]
fn non_breaking_space() {
    // Issue #66: non-breaking space should not be included in email addresses
    assert_linked(
        "this is a mail address:\u{a0}test@example.com\u{a0}surrounded by non-breaking spaces",
        "this is a mail address:\u{a0}|test@example.com|\u{a0}surrounded by non-breaking spaces",
    );
    assert_linked("\u{a0}a@b.com\u{a0}", "\u{a0}|a@b.com|\u{a0}");
    assert_linked("a@b.com\u{a0}c@d.com", "|a@b.com|\u{a0}|c@d.com|");
}

#[test]
fn other_unicode_whitespace() {
    // U+202F NARROW NO-BREAK SPACE (common in French typography)
    assert_linked("a@b.com\u{202f}c@d.com", "|a@b.com|\u{202f}|c@d.com|");
    // U+2003 EM SPACE
    assert_linked("a@b.com\u{2003}c@d.com", "|a@b.com|\u{2003}|c@d.com|");
    // U+3000 IDEOGRAPHIC SPACE (CJK)
    assert_linked("a@b.com\u{3000}c@d.com", "|a@b.com|\u{3000}|c@d.com|");
    // U+2028 LINE SEPARATOR
    assert_linked("a@b.com\u{2028}c@d.com", "|a@b.com|\u{2028}|c@d.com|");
    // U+2029 PARAGRAPH SEPARATOR
    assert_linked("a@b.com\u{2029}c@d.com", "|a@b.com|\u{2029}|c@d.com|");
    // U+1680 OGHAM SPACE MARK
    assert_linked("a@b.com\u{1680}c@d.com", "|a@b.com|\u{1680}|c@d.com|");
    // U+205F MEDIUM MATHEMATICAL SPACE
    assert_linked("a@b.com\u{205f}c@d.com", "|a@b.com|\u{205f}|c@d.com|");
}

#[test]
fn unicode_whitespace_in_local_part() {
    // NBSP before @ should not be included in local part
    assert_linked("test\u{a0}@example.com", "test\u{a0}@example.com");
    // International chars still work
    assert_linked("tëst@example.com", "|tëst@example.com|");
    // International char followed by NBSP
    assert_linked("tëst\u{a0}@example.com", "tëst\u{a0}@example.com");
}

#[test]
fn unicode_whitespace_in_domain() {
    // NBSP in domain should stop the email
    assert_linked("test@exam\u{a0}ple.com", "test@exam\u{a0}ple.com");
    // International domain still works
    assert_linked("test@exämple.com", "|test@exämple.com|");
    // International domain followed by NBSP
    assert_linked("test@exämple\u{a0}.com", "test@exämple\u{a0}.com");
}

fn assert_not_linked(s: &str) {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Email]);
    let result = finder.links(s);
    assert_eq!(result.count(), 0, "expected no links in {:?}", s);
}

fn assert_linked(input: &str, expected: &str) {
    let mut finder = LinkFinder::new();
    finder.kinds(&[LinkKind::Email]);
    assert_linked_with(&finder, input, expected);
}

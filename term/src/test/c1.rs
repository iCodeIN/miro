//! Testing C1 control sequences

use super::*;

#[test]
fn test_ind() {
    let mut term = TestTerm::new(4, 4, 0);
    term.print("a\nb\x1bD");
    term.assert_cursor_pos(1, 2, None);
    assert_visible_contents(&term, &["a   ", "b   ", "    ", "    "]);
    term.print("\x1bD");
    term.assert_cursor_pos(1, 3, None);
    term.print("\x1bD");
    term.assert_cursor_pos(1, 3, None);
    assert_visible_contents(&term, &["b   ", "    ", "    ", "    "]);
}

#[test]
fn test_nel() {
    let mut term = TestTerm::new(4, 4, 0);
    term.print("a\nb\x1bE");
    term.assert_cursor_pos(0, 2, None);
    term.print("\x1bE");
    term.assert_cursor_pos(0, 3, None);
    term.print("\x1bE");
    term.assert_cursor_pos(0, 3, None);
    assert_visible_contents(&term, &["b   ", "    ", "    ", "    "]);
}

#[test]
fn test_hts() {
    let mut term = TestTerm::new(3, 25, 0);
    term.print("boo");
    term.print("\x1bH\n");
    term.assert_cursor_pos(0, 1, None);
    term.print("\t");
    term.assert_cursor_pos(3, 1, None);
    term.print("\t");
    term.assert_cursor_pos(8, 1, None);
}

#[test]
fn test_ri() {
    let mut term = TestTerm::new(4, 2, 0);
    term.print("a\nb\nc\nd.");
    assert_visible_contents(&term, &["a ", "b ", "c ", "d."]);
    term.assert_cursor_pos(1, 3, None);
    term.print("\x1bM");
    term.assert_cursor_pos(1, 2, None);
    term.print("\x1bM");
    term.assert_cursor_pos(1, 1, None);
    term.print("\x1bM");
    term.assert_cursor_pos(1, 0, None);
    term.print("\x1bM");
    term.assert_cursor_pos(1, 0, None);
    assert_visible_contents(&term, &["  ", "a ", "b ", "c "]);
}

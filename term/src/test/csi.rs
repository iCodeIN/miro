use super::*;

#[test]
fn test_vpa() {
    let mut term = TestTerm::new(3, 4, 0);
    term.assert_cursor_pos(0, 0, None);
    term.print("a\nb\nc");
    term.assert_cursor_pos(1, 2, None);
    term.print("\x1b[d");
    term.assert_cursor_pos(1, 0, None);
    term.print("\n\n");
    term.assert_cursor_pos(0, 2, None);

    // escapes are 1-based, so check that we're handling that
    // when we parse them!
    term.print("\x1b[2d");
    term.assert_cursor_pos(0, 1, None);
    term.print("\x1b[-2d");
    term.assert_cursor_pos(0, 1, None);
}

#[test]
fn test_ech() {
    let mut term = TestTerm::new(3, 4, 0);
    term.print("hey!wat?");
    term.cup(1, 0);
    term.print("\x1b[2X");
    assert_visible_contents(&term, &["h  !", "wat?", "    "]);
    // check how we handle overflowing the width
    term.print("\x1b[12X");
    assert_visible_contents(&term, &["h   ", "wat?", "    "]);
    term.print("\x1b[-12X");
    assert_visible_contents(&term, &["h   ", "wat?", "    "]);
}

#[test]
fn test_cup() {
    let mut term = TestTerm::new(3, 4, 0);
    term.cup(1, 1);
    term.assert_cursor_pos(1, 1, None);
    term.cup(-1, -1);
    term.assert_cursor_pos(0, 0, None);
    term.cup(2, 2);
    term.assert_cursor_pos(2, 2, None);
    term.cup(-1, -1);
    term.assert_cursor_pos(0, 0, None);
    term.cup(500, 500);
    term.assert_cursor_pos(3, 2, None);
}

#[test]
fn test_hvp() {
    let mut term = TestTerm::new(3, 4, 0);
    term.hvp(1, 1);
    term.assert_cursor_pos(1, 1, None);
    term.hvp(-1, -1);
    term.assert_cursor_pos(0, 0, None);
    term.hvp(2, 2);
    term.assert_cursor_pos(2, 2, None);
    term.hvp(-1, -1);
    term.assert_cursor_pos(0, 0, None);
    term.hvp(500, 500);
    term.assert_cursor_pos(3, 2, None);
}

#[test]
fn test_dl() {
    let mut term = TestTerm::new(3, 1, 0);
    term.print("a\nb\nc");
    term.cup(0, 1);
    term.delete_lines(1);
    assert_visible_contents(&term, &["a", "c", " "]);
    term.assert_cursor_pos(0, 1, None);
    term.cup(0, 0);
    term.delete_lines(2);
    assert_visible_contents(&term, &[" ", " ", " "]);
    term.print("1\n2\n3");
    term.cup(0, 1);
    term.delete_lines(-2);
    assert_visible_contents(&term, &["1", "2", "3"]);
}

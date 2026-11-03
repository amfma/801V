//! Tests for `Diagnostic` and `Error` (src/error.rs).
use super::*;
use crate::token::Literal;

fn token(token_type: TokenType, lexeme: &str, line: usize) -> Token {
    Token::new(token_type, lexeme, line, Literal::None, 1)
}

/* ---------- Diagnostic ---------- */

#[test]
fn diagnostic_at_a_regular_token_quotes_the_lexeme() {
    let d = Diagnostic::at(&token(TokenType::Semicolon, ";", 3), "Expect expression.");
    assert_eq!(d.line, 3);
    assert_eq!(d.location, " at ';'");
    assert_eq!(d.message, "Expect expression.");
}

#[test]
fn diagnostic_at_eof_says_at_end() {
    let d = Diagnostic::at(&token(TokenType::EOF, "", 7), "Oops.");
    assert_eq!(d.line, 7);
    assert_eq!(d.location, " at end");
}

#[test]
fn diagnostic_display_format() {
    let d = Diagnostic::at(&token(TokenType::Identifier, "foo", 12), "Bad thing.");
    assert_eq!(d.to_string(), "[line 12] Error at 'foo': Bad thing.");
    let d = Diagnostic::at(&token(TokenType::EOF, "", 1), "Bad thing.");
    assert_eq!(d.to_string(), "[line 1] Error at end: Bad thing.");
}

#[test]
fn diagnostic_accepts_owned_and_borrowed_messages() {
    let t = token(TokenType::Plus, "+", 1);
    assert_eq!(Diagnostic::at(&t, "a").message, "a");
    assert_eq!(Diagnostic::at(&t, String::from("b")).message, "b");
}

/* ---------- Error constructors ---------- */

#[test]
fn error_syntax_wraps_a_diagnostic() {
    let t = token(TokenType::Number, "42", 2);
    assert_eq!(
        Error::syntax(&t, "msg"),
        Error::Syntax(Diagnostic::at(&t, "msg"))
    );
}

#[test]
fn error_resolve_wraps_a_diagnostic() {
    let t = token(TokenType::This, "this", 2);
    assert_eq!(
        Error::resolve(&t, "msg"),
        Error::Resolve(Diagnostic::at(&t, "msg"))
    );
}

#[test]
fn syntax_and_resolve_errors_are_distinct_variants() {
    let t = token(TokenType::This, "this", 2);
    assert_ne!(Error::syntax(&t, "msg"), Error::resolve(&t, "msg"));
}

/* ---------- Error display ---------- */

#[test]
fn error_display() {
    let t = token(TokenType::Identifier, "x", 4);
    assert_eq!(
        Error::syntax(&t, "Expect expression.").to_string(),
        "[line 4] Error at 'x': Expect expression."
    );
    assert_eq!(
        Error::resolve(&t, "Nope.").to_string(),
        "[line 4] Error at 'x': Nope."
    );
    assert_eq!(Error::RuntimeError.to_string(), "Runtime error.");
    assert_eq!(Error::UndefinedVariable.to_string(), "Undefined variable.");
    assert_eq!(
        Error::Return(Value::Nil).to_string(),
        "Can't return from top-level code."
    );
}

#[test]
fn errors_can_be_cloned_and_compared() {
    let e = Error::Return(Value::Number(1.0));
    assert_eq!(e.clone(), e);
    assert_ne!(Error::RuntimeError, Error::UndefinedVariable);
}

/* ---------- runtime errors ---------- */

#[test]
fn runtime_error_at_a_token_uses_its_line_and_has_no_location() {
    let t = token(TokenType::Identifier, "x", 9);
    match Error::runtime(&t, "Boom.") {
        Error::Runtime(d) => {
            assert_eq!(d.line, 9);
            assert_eq!(d.location, "");
            assert_eq!(d.message, "Boom.");
        }
        other => panic!("expected a runtime error, got {other:?}"),
    }
}

#[test]
fn runtime_error_at_an_explicit_line() {
    assert_eq!(
        Error::runtime_at(5, "Boom."),
        Error::runtime(&token(TokenType::Plus, "+", 5), "Boom.")
    );
}

#[test]
fn runtime_error_display_format() {
    assert_eq!(
        Error::runtime_at(3, "Division by zero.").to_string(),
        "[line 3] Runtime error: Division by zero."
    );
}

#[test]
fn located_and_unlocated_runtime_errors_are_distinct() {
    assert_ne!(Error::runtime_at(1, "x"), Error::RuntimeError);
}

/* ---------- columns and snippets ---------- */

fn diagnostic_at(line: usize, column: Option<usize>) -> Diagnostic {
    Diagnostic {
        line,
        column,
        location: String::new(),
        message: "boom".to_string(),
    }
}

#[test]
fn runtime_error_with_a_column_shows_line_and_column() {
    assert_eq!(
        Error::runtime_at_column(3, 9, "Division by zero.").to_string(),
        "[line 3:9] Runtime error: Division by zero."
    );
}

#[test]
fn a_zero_column_means_unknown_and_is_dropped() {
    let error = Error::runtime_at_column(3, 0, "Division by zero.");
    assert_eq!(error.to_string(), "[line 3] Runtime error: Division by zero.");
    match error {
        Error::Runtime(d) => assert_eq!(d.column, None),
        other => panic!("expected runtime error, got {other:?}"),
    }
}

#[test]
fn errors_without_a_column_keep_the_old_format() {
    assert_eq!(
        Error::runtime_at(3, "x").to_string(),
        "[line 3] Runtime error: x"
    );
    assert_eq!(diagnostic_at(1, None).snippet("print 1;"), None);
}

#[test]
fn snippet_shows_the_source_line_and_a_caret_under_the_column() {
    assert_eq!(
        diagnostic_at(1, Some(9)).snippet("print 1 / 0;"),
        Some("  print 1 / 0;\n          ^".to_string())
    );
}

#[test]
fn snippet_picks_the_reported_line() {
    let source = "var a = 1;\nvar b = a / 0;\nprint b;";
    assert_eq!(
        diagnostic_at(2, Some(11)).snippet(source),
        Some("  var b = a / 0;\n            ^".to_string())
    );
}

#[test]
fn snippet_caret_in_column_one_has_no_padding() {
    assert_eq!(
        diagnostic_at(1, Some(1)).snippet("-true;"),
        Some("  -true;\n  ^".to_string())
    );
}

#[test]
fn snippet_keeps_tabs_so_the_caret_lines_up() {
    assert_eq!(
        diagnostic_at(1, Some(10)).snippet("\tprint 1 / 0;"),
        Some("  \tprint 1 / 0;\n  \t        ^".to_string())
    );
}

#[test]
fn snippet_ignores_a_carriage_return() {
    assert_eq!(
        diagnostic_at(1, Some(3)).snippet("1 + nil;\r\nprint 2;"),
        Some("  1 + nil;\n    ^".to_string())
    );
}

#[test]
fn snippet_is_none_when_the_line_does_not_exist() {
    assert_eq!(diagnostic_at(0, Some(1)).snippet("print 1;"), None);
    assert_eq!(diagnostic_at(5, Some(1)).snippet("print 1;"), None);
    assert_eq!(diagnostic_at(1, Some(1)).snippet(""), None);
}

#[test]
fn snippet_pads_when_the_column_is_past_the_end_of_the_line() {
    assert_eq!(
        diagnostic_at(1, Some(6)).snippet("ab"),
        Some("  ab\n       ^".to_string())
    );
}

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

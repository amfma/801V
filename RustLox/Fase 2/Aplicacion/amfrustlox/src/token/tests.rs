//! Tests for `Scanner` and `Token` (src/token.rs).
use super::TokenType as T;
use super::*;

/* ---------- helpers ---------- */

fn scan(src: &str) -> Vec<Token> {
    Scanner::new(src)
        .scan_tokens()
        .expect("scanning should succeed")
}

fn types(src: &str) -> Vec<TokenType> {
    scan(src).into_iter().map(|t| t.token_type).collect()
}

fn lexemes(src: &str) -> Vec<String> {
    scan(src).into_iter().map(|t| t.lexeme).collect()
}

/* ---------- basics ---------- */

#[test]
fn empty_source_yields_only_eof() {
    let tokens = scan("");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].token_type, T::EOF);
    assert_eq!(tokens[0].lexeme, "");
    assert_eq!(tokens[0].literal, Literal::None);
}

#[test]
fn whitespace_only_yields_only_eof() {
    assert_eq!(types(" \t\r\n  \n"), vec![T::EOF]);
}

#[test]
fn eof_is_always_the_last_token() {
    let tokens = scan("var a = 1;");
    assert_eq!(tokens.last().unwrap().token_type, T::EOF);
    assert_eq!(
        tokens.iter().filter(|t| t.token_type == T::EOF).count(),
        1
    );
}

/* ---------- punctuation & operators ---------- */

#[test]
fn single_character_tokens() {
    assert_eq!(
        types("(){},.-+*;"),
        vec![
            T::LeftParen,
            T::RightParen,
            T::LeftBrace,
            T::RightBrace,
            T::Comma,
            T::Dot,
            T::Minus,
            T::Plus,
            T::Star,
            T::Semicolon,
            T::EOF
        ]
    );
}

#[test]
fn one_or_two_character_operators() {
    assert_eq!(
        types("! != = == < <= > >="),
        vec![
            T::Bang,
            T::BangEqual,
            T::Equal,
            T::EqualEqual,
            T::Less,
            T::LessEqual,
            T::Greater,
            T::GreaterEqual,
            T::EOF
        ]
    );
}

#[test]
fn operators_are_matched_greedily() {
    assert_eq!(types("!=="), vec![T::BangEqual, T::Equal, T::EOF]);
    assert_eq!(types("==="), vec![T::EqualEqual, T::Equal, T::EOF]);
    assert_eq!(types("<=="), vec![T::LessEqual, T::Equal, T::EOF]);
}

#[test]
fn operator_lexemes_are_preserved() {
    assert_eq!(lexemes("!= >="), vec!["!=", ">=", ""]);
}

/* ---------- slash & comments ---------- */

#[test]
fn lone_slash_is_a_slash_token() {
    assert_eq!(types("/"), vec![T::Slash, T::EOF]);
    assert_eq!(
        types("1 / 2"),
        vec![T::Number, T::Slash, T::Number, T::EOF]
    );
}

#[test]
fn line_comment_is_skipped_until_newline() {
    assert_eq!(types("// nothing here\n+"), vec![T::Plus, T::EOF]);
}

#[test]
fn comment_at_end_of_file_without_newline() {
    assert_eq!(types("+ // trailing"), vec![T::Plus, T::EOF]);
    assert_eq!(types("// only a comment"), vec![T::EOF]);
}

/* ---------- whitespace & line tracking ---------- */

#[test]
fn spaces_tabs_and_carriage_returns_are_ignored() {
    assert_eq!(types(" \t\r+ \t"), vec![T::Plus, T::EOF]);
}

#[test]
fn newlines_increment_the_line_counter() {
    let lines: Vec<usize> = scan("a\nb\n\nc").iter().map(|t| t.line).collect();
    assert_eq!(lines, vec![1, 2, 4, 4]); // a, b, c, EOF
}

/* ---------- strings ---------- */

#[test]
fn string_literal() {
    let tokens = scan("\"hello\"");
    assert_eq!(tokens[0].token_type, T::String);
    assert_eq!(tokens[0].lexeme, "\"hello\"");
    assert_eq!(tokens[0].literal, Literal::Str("hello".to_string()));
    assert_eq!(tokens[1].token_type, T::EOF);
}

#[test]
fn empty_string_literal() {
    let tokens = scan("\"\"");
    assert_eq!(tokens[0].token_type, T::String);
    assert_eq!(tokens[0].literal, Literal::Str(String::new()));
}

#[test]
fn string_may_contain_keywords_and_symbols() {
    let tokens = scan("\"var x = 1; // not a comment\"");
    assert_eq!(tokens.len(), 2);
    assert_eq!(
        tokens[0].literal,
        Literal::Str("var x = 1; // not a comment".to_string())
    );
}

#[test]
fn multiline_string_advances_the_line_counter() {
    let tokens = scan("\"a\nb\" x");
    assert_eq!(tokens[0].literal, Literal::Str("a\nb".to_string()));
    assert_eq!(tokens[1].token_type, T::Identifier);
    assert_eq!(tokens[1].line, 2);
}

#[test]
fn two_consecutive_strings() {
    assert_eq!(
        types("\"a\" \"b\""),
        vec![T::String, T::String, T::EOF]
    );
}

/// Documents the CURRENT behaviour: an unterminated string is silently
/// dropped (the scanner's `error` vector is never filled).
#[test]
fn unterminated_string_currently_produces_no_token() {
    assert_eq!(types("\"abc"), vec![T::EOF]);
}

/* ---------- numbers ---------- */

#[test]
fn integer_literal() {
    let tokens = scan("123");
    assert_eq!(tokens[0].token_type, T::Number);
    assert_eq!(tokens[0].lexeme, "123");
    assert_eq!(tokens[0].literal, Literal::Num(123.0));
}

#[test]
fn decimal_literal() {
    let tokens = scan("2.5");
    assert_eq!(tokens[0].token_type, T::Number);
    assert_eq!(tokens[0].lexeme, "2.5");
    assert_eq!(tokens[0].literal, Literal::Num(2.5));
}

#[test]
fn trailing_dot_is_not_part_of_the_number() {
    assert_eq!(types("7."), vec![T::Number, T::Dot, T::EOF]);
}

#[test]
fn leading_dot_is_not_part_of_the_number() {
    assert_eq!(types(".5"), vec![T::Dot, T::Number, T::EOF]);
}

#[test]
fn second_dot_ends_the_number() {
    assert_eq!(
        types("1.2.3"),
        vec![T::Number, T::Dot, T::Number, T::EOF]
    );
    assert_eq!(lexemes("1.2.3"), vec!["1.2", ".", "3", ""]);
}

#[test]
fn number_followed_by_method_style_access() {
    assert_eq!(
        types("12.foo"),
        vec![T::Number, T::Dot, T::Identifier, T::EOF]
    );
}

/* ---------- identifiers & keywords ---------- */

#[test]
fn identifiers() {
    let tokens = scan("foo bar_baz x1");
    let ids: Vec<&str> = tokens
        .iter()
        .filter(|t| t.token_type == T::Identifier)
        .map(|t| t.lexeme.as_str())
        .collect();
    assert_eq!(ids, vec!["foo", "bar_baz", "x1"]);
    assert!(tokens.iter().all(|t| t.literal == Literal::None));
}

#[test]
fn unicode_letters_are_valid_identifier_characters() {
    let tokens = scan("ñandú");
    assert_eq!(tokens[0].token_type, T::Identifier);
    assert_eq!(tokens[0].lexeme, "ñandú");
}

#[test]
fn every_keyword_is_recognised() {
    let keywords = [
        ("and", T::And),
        ("class", T::Class),
        ("else", T::Else),
        ("false", T::False),
        ("for", T::For),
        ("fun", T::Fun),
        ("if", T::If),
        ("nil", T::Nil),
        ("or", T::Or),
        ("print", T::Print),
        ("return", T::Return),
        ("super", T::Super),
        ("this", T::This),
        ("true", T::True),
        ("var", T::Var),
        ("while", T::While),
    ];
    for (word, expected) in keywords {
        let tokens = scan(word);
        assert_eq!(tokens[0].token_type, expected, "keyword `{word}`");
        assert_eq!(tokens[0].lexeme, word);
        assert_eq!(tokens[0].literal, Literal::None);
        assert_eq!(tokens.len(), 2, "keyword `{word}` should be a single token");
    }
}

#[test]
fn keyword_prefixes_are_identifiers() {
    for word in ["orchid", "classy", "fortune", "variable", "nilly", "printer"] {
        assert_eq!(types(word), vec![T::Identifier, T::EOF], "`{word}`");
    }
}

#[test]
fn keywords_are_case_sensitive() {
    assert_eq!(types("While"), vec![T::Identifier, T::EOF]);
    assert_eq!(types("TRUE"), vec![T::Identifier, T::EOF]);
}

#[test]
fn identifier_may_contain_keyword_followed_by_digits() {
    assert_eq!(types("if1"), vec![T::Identifier, T::EOF]);
}

/* ---------- unknown characters ---------- */

#[test]
fn unknown_characters_are_ignored() {
    assert_eq!(types("@ # $ ?"), vec![T::EOF]);
    assert_eq!(types("1 @ 2"), vec![T::Number, T::Number, T::EOF]);
}

/// Documents the CURRENT behaviour: `_` cannot start an identifier, so the
/// leading underscore is silently skipped.
#[test]
fn leading_underscore_is_currently_skipped() {
    assert_eq!(lexemes("_foo"), vec!["foo", ""]);
}

/* ---------- whole snippets ---------- */

#[test]
fn variable_declaration_snippet() {
    assert_eq!(
        types("var x = 10;"),
        vec![T::Var, T::Identifier, T::Equal, T::Number, T::Semicolon, T::EOF]
    );
}

#[test]
fn function_declaration_snippet() {
    assert_eq!(
        types("fun add(a, b) { return a + b; }"),
        vec![
            T::Fun,
            T::Identifier,
            T::LeftParen,
            T::Identifier,
            T::Comma,
            T::Identifier,
            T::RightParen,
            T::LeftBrace,
            T::Return,
            T::Identifier,
            T::Plus,
            T::Identifier,
            T::Semicolon,
            T::RightBrace,
            T::EOF
        ]
    );
}

#[test]
fn class_with_inheritance_snippet() {
    assert_eq!(
        types("class B < A { m() { return super.m(); } }")
            .iter()
            .filter(|t| **t == T::Class || **t == T::Less || **t == T::Super || **t == T::Return)
            .count(),
        4
    );
}

/* ---------- token ids & Token API ---------- */

#[test]
fn token_ids_are_strictly_increasing_within_a_scan() {
    let ids: Vec<usize> = scan("var a = 1; print a;")
        .iter()
        .map(|t| t.idx.0)
        .collect();
    assert!(ids.windows(2).all(|w| w[0] < w[1]), "ids: {ids:?}");
}

#[test]
fn token_ids_are_unique_across_scans() {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for _ in 0..3 {
        for token in scan("a b c") {
            assert!(seen.insert(token.idx), "duplicate token id {:?}", token.idx);
        }
    }
}

#[test]
fn token_new_stores_all_fields() {
    let token = Token::new(T::Identifier, "abc", 7, Literal::None, 99);
    assert_eq!(token.token_type, T::Identifier);
    assert_eq!(token.lexeme, "abc");
    assert_eq!(token.line, 7);
    assert_eq!(token.literal, Literal::None);
    assert_eq!(token.idx, TokenIndex(99));
}

#[test]
fn token_to_string_format() {
    let token = scan("+").remove(0);
    assert_eq!(token.to_string(), "Plus + None");
}

#[test]
fn tokens_with_equal_fields_are_equal_but_ids_make_scans_differ() {
    let a = Token::new(T::Plus, "+", 1, Literal::None, 1);
    let b = Token::new(T::Plus, "+", 1, Literal::None, 1);
    let c = Token::new(T::Plus, "+", 1, Literal::None, 2);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

use std::{clone, fmt::format, string};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::error::Diagnostic;

/* Token ID Count */
static NEXT_TOKEN_ID: AtomicUsize = AtomicUsize::new(1);

fn next_token_id() -> usize {
    NEXT_TOKEN_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Str(String),
    Num(f64),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Identifier,
    String,
    Number,
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    EOF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenIndex(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub line: usize,
    pub literal: Literal,
    pub idx: TokenIndex,
    /// 1-based column of the token's first character on its line, counted in
    /// characters. `0` means unknown (tokens built by hand, e.g. in tests).
    pub column: usize,
}

impl Token {
    pub fn new(
        token_type: TokenType,
        lexeme: impl Into<String>,
        line: usize,
        literal: Literal,
        idx: usize,
    ) -> Self {
        Self {
            token_type,
            lexeme: lexeme.into(),
            line,
            literal,
            idx: TokenIndex(idx),
            column: 0,
        }
    }

    /// Returns the token with its source column set.
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = column;
        self
    }

    pub fn to_string(self) -> String {
        format!("{:#?} {} {:#?}", self.token_type, self.lexeme, self.literal)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scanner {
    source: Vec<char>,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
    /// Index in `source` of the first character of the current line.
    line_start: usize,
    /// Column of the token being scanned (fixed when the token starts, so a
    /// string that spans lines still reports where it began).
    start_column: usize,
    errors: Vec<Diagnostic>,
}

impl Scanner {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            tokens: vec![],
            start: 0,
            current: 0,
            line: 1,
            line_start: 0,
            start_column: 1,
            errors: vec![],
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    /// Scans the whole source. A lexical error does not stop the scan: it is
    /// recorded, the scanner resynchronizes, and scanning continues, so every
    /// lexical error in the source is returned together.
    pub fn scan_tokens(mut self) -> Result<Vec<Token>, Vec<Diagnostic>> {
        while !self.is_at_end() {
            self.start = self.current;
            self.start_column = self.start - self.line_start + 1;
            self.scan_token();
        }

        let eof_column = self.current - self.line_start + 1;
        self.tokens.push(
            Token::new(TokenType::EOF, "", self.line, Literal::None, next_token_id())
                .with_column(eof_column),
        );

        if self.errors.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.errors)
        }
    }

    /// Records a lexical error without interrupting the scan.
    fn error(&mut self, line: usize, message: impl Into<String>) {
        self.errors.push(Diagnostic {
            line,
            column: None,
            location: String::new(),
            message: message.into(),
        });
    }

    /// True for characters that begin a token (or are skipped whitespace).
    fn starts_token(c: char) -> bool {
        matches!(
            c,
            '(' | ')' | '{' | '}' | ',' | '.' | '-' | '+' | '*' | ';' | '!' | '=' | '<' | '>'
                | '/' | '"' | ' ' | '\r' | '\t' | '\n' | '_'
        ) || c.is_alphabetic()
            || c.is_ascii_digit()
    }

    /// Error recovery: after an unexpected character, skip the rest of the
    /// run of characters that cannot start a token, so `@@@` is reported once
    /// instead of three times. Scanning then resumes at the next valid start.
    fn synchronize(&mut self) {
        while !self.is_at_end() && !Self::starts_token(self.peek()) {
            self.advance();
        }
    }

    fn add_token(&mut self, token_type: TokenType, literal: Literal) {
        let lexeme: String = self.source[self.start..self.current].iter().collect();
        self.tokens.push(
            Token::new(token_type, lexeme, self.line, literal, next_token_id())
                .with_column(self.start_column),
        );
    }

    fn scan_token(&mut self) {
        match self.advance() {
            '(' => self.add_token(TokenType::LeftParen, Literal::None),
            ')' => self.add_token(TokenType::RightParen, Literal::None),
            '{' => self.add_token(TokenType::LeftBrace, Literal::None),
            '}' => self.add_token(TokenType::RightBrace, Literal::None),
            ',' => self.add_token(TokenType::Comma, Literal::None),
            '.' => self.add_token(TokenType::Dot, Literal::None),
            '-' => self.add_token(TokenType::Minus, Literal::None),
            '+' => self.add_token(TokenType::Plus, Literal::None),
            '*' => self.add_token(TokenType::Star, Literal::None),
            ';' => self.add_token(TokenType::Semicolon, Literal::None),
            '!' => {
                if self.match_char('=') {
                    self.add_token(TokenType::BangEqual, Literal::None);
                } else {
                    self.add_token(TokenType::Bang, Literal::None);
                };
            }
            '=' => {
                if self.match_char('=') {
                    self.add_token(TokenType::EqualEqual, Literal::None);
                } else {
                    self.add_token(TokenType::Equal, Literal::None);
                };
            }
            '<' => {
                if self.match_char('=') {
                    self.add_token(TokenType::LessEqual, Literal::None);
                } else {
                    self.add_token(TokenType::Less, Literal::None);
                };
            }
            '>' => {
                if self.match_char('=') {
                    self.add_token(TokenType::GreaterEqual, Literal::None);
                } else {
                    self.add_token(TokenType::Greater, Literal::None);
                };
            }
            '/' => {
                if self.match_char('/') {
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(TokenType::Slash, Literal::None);
                };
            }
            ' ' | '\r' | '\t' => {}
            '\n' => {
                self.line += 1;
                self.line_start = self.current;
            }
            '"' => self.string(),

            c if c.is_alphabetic() || c == '_' => {
                self.identifier();
            }

            c if c.is_digit(10) => {
                self.number();
            }

            c => {
                self.error(self.line, format!("Unexpected character '{c}'."));
                self.synchronize();
            }
        }
    }

    fn advance(&mut self) -> char {
        let character: char = self.source[self.current];
        self.current += 1;

        character
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }

        if self.source[self.current] != expected {
            return false;
        }

        self.current += 1;
        true
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            return '\x00';
        } else {
            self.source[self.current]
        }
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            return '\x00';
        }
        self.source[self.current + 1]
    }

    fn string(&mut self) {
        let start_line = self.line;
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
                self.line_start = self.current + 1;
            }

            self.advance();
        }

        if self.is_at_end() {
            // Reported where the string opened; the rest of the source was
            // consumed by the string, so there is nothing left to resync on.
            self.error(start_line, "Unterminated string.");
            return;
        }

        self.advance();

        let str_value: String = self.source[self.start + 1..self.current - 1]
            .iter()
            .collect::<String>();

        self.add_token(TokenType::String, Literal::Str(str_value));
    }

    fn number(&mut self) {
        while self.peek().is_digit(10) {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_digit(10) {
            self.advance();

            while self.peek().is_digit(10) {
                self.advance();
            }
        }

        let number_value: String = self.source[self.start..self.current].iter().collect();

        self.add_token(
            TokenType::Number,
            Literal::Num(number_value.parse().unwrap()),
        );
    }

    fn identifier(&mut self) {
        while self.peek().is_alphanumeric() || self.peek() == '_' {
            self.advance();
        }

        let lexeme: String = self.source[self.start..self.current].iter().collect();

        match lexeme.as_str() {
            "and" => self.add_token(TokenType::And, Literal::None),
            "class" => self.add_token(TokenType::Class, Literal::None),
            "else" => self.add_token(TokenType::Else, Literal::None),
            "false" => self.add_token(TokenType::False, Literal::None),
            "for" => self.add_token(TokenType::For, Literal::None),
            "fun" => self.add_token(TokenType::Fun, Literal::None),
            "if" => self.add_token(TokenType::If, Literal::None),
            "nil" => self.add_token(TokenType::Nil, Literal::None),
            "or" => self.add_token(TokenType::Or, Literal::None),
            "print" => self.add_token(TokenType::Print, Literal::None),
            "return" => self.add_token(TokenType::Return, Literal::None),
            "super" => self.add_token(TokenType::Super, Literal::None),
            "this" => self.add_token(TokenType::This, Literal::None),
            "true" => self.add_token(TokenType::True, Literal::None),
            "var" => self.add_token(TokenType::Var, Literal::None),
            "while" => self.add_token(TokenType::While, Literal::None),
            _ => self.add_token(TokenType::Identifier, Literal::None),
        }
    }
}

   #[cfg(test)]
   mod tests;
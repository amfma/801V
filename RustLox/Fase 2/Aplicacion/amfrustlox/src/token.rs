use std::{clone, fmt::format, string};

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Str(String),
    Num(f64),
    None
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType{
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
    EOF
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type : TokenType,
    pub lexeme : String,
    line: usize,
    literal: Literal,
}

impl Token {
    pub fn new(token_type:TokenType,
    lexeme: impl Into<String>, 
    line: usize,
    literal: Literal) -> Self {
        Self {
            token_type, lexeme: lexeme.into(), line, literal
        }
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
    error: Vec<i32>
}

impl Scanner {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            tokens: vec![],
            start: 0,
            current: 0,
            line: 1,
            error: vec![]
        }
    }

    fn is_at_end(&self) -> bool{
        self.current >= self.source.len() 
    }

    pub fn scan_tokens(mut self) -> Result<Vec<Token>, &'static str> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.tokens.push(Token::new(TokenType::EOF, "", self.line, Literal::None));

        if self.error.len() == 0 {
            Ok(self.tokens)
        } else {
            Err("Error")
        }
    }

    fn add_token(&mut self, token_type: TokenType, literal: Literal) {
        let lexeme: String = self.source[self.start..self.current].iter().collect();
        self.tokens.push(Token::new(token_type, lexeme, self.line, literal));
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
                    while self.peek() != '\n' && self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(TokenType::Slash, Literal::None);
                };
            }
            ' ' | '\r' | '\t' => {},
            '\n' => self.line +=1,
            '"' => self.string(),

            c if c.is_alphabetic() => {
                self.identifier();
            },

            c if c.is_digit(10) => {
                self.number();
            }

            c => {}

        }
    } 

    fn advance(&mut self) -> char {
        let character: char = self.source[self.current];
        self.current += 1;

        character
    }

    fn match_char(&mut self, expected: char) -> bool{
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
        if self.current +1 >= self.source.len() {
            return '\x00';
        }
        self.source[self.current + 1]
    }

    fn string(&mut self) {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }

            self.advance();
        }

        if self.is_at_end() {
            return;
        }

        self.advance();

        let str_value: String = self.source[self.start + 1..self.current - 1].iter().collect::<String>();

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

        self.add_token(TokenType::Number, Literal::Num(number_value.parse().unwrap()));
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
            _ => self.add_token(TokenType::Identifier, Literal::None)
        }
    }

}

pub fn scan(){

}

#[cfg(test)]
mod tests {
    use std::iter::Scan;

use super::*;

    #[test]
    fn single_character_multi_line_test() {
        let scanner: Scanner = Scanner::new("()\n{}\n+-,.\n;*");
        let tokens: Vec<Token> = scanner.scan_tokens().unwrap();

        assert_eq!(tokens, vec![Token::new(TokenType::LeftParen, "(", 1, Literal::None), 
        Token::new(TokenType::RightParen, ")", 1, Literal::None),
        Token::new(TokenType::LeftBrace, "{", 2, Literal::None),
        Token::new(TokenType::RightBrace, "}", 2, Literal::None),
        Token::new(TokenType::Plus, "+", 3, Literal::None),
        Token::new(TokenType::Minus, "-", 3, Literal::None),
        Token::new(TokenType::Comma, ",", 3, Literal::None),
        Token::new(TokenType::Dot, ".", 3, Literal::None),
        Token::new(TokenType::Semicolon, ";", 4, Literal::None),
        Token::new(TokenType::Star, "*", 4, Literal::None),
        Token::new(TokenType::EOF, "", 4, Literal::None)])
    }

    #[test]
    fn single_string_single_line_test() {
        let scanner: Scanner = Scanner::new("\"Hello world\"");
        let tokens: Vec<Token> = scanner.scan_tokens().unwrap();

        assert_eq!(tokens, vec![Token::new(TokenType::String, "\"Hello world\"", 1, Literal::Str("Hello world".to_string())),
        Token::new(TokenType::EOF, "", 1, Literal::None)])
    }

    #[test]
    fn multi_string_single_line_test() {
        let scanner: Scanner = Scanner::new("\"Hola\" \"Chao\"");
        let tokens: Vec<Token> = scanner.scan_tokens().unwrap();

        assert_eq!(tokens,
        vec![Token::new(TokenType::String, "\"Hola\"", 1, Literal::Str("Hola".to_string())),
        Token::new(TokenType::String, "\"Chao\"", 1, Literal::Str("Chao".to_string())),
        Token::new(TokenType::EOF, "", 1, Literal::None)])
    }

    #[test]
    fn string_multi_line_test() {
        let scanner: Scanner = Scanner::new("\"Hola \nChao\"");
        let tokens: Vec<Token> = scanner.scan_tokens().unwrap();

        assert_eq!(tokens, vec![Token::new(TokenType::String, "\"Hola \nChao\"", 2, Literal::Str("Hola \nChao".to_string())),
        Token::new(TokenType::EOF, "", 2, Literal::None)])  
    }

    #[test]
    fn number_test() {
        let scanner: Scanner = Scanner::new("1234 5678.9");
        let tokens: Vec<Token> = scanner.scan_tokens().unwrap();

        assert_eq!(tokens, vec![Token::new(TokenType::Number, "1234" , 1, Literal::Num(1234.0)),
        Token::new(TokenType::Number, "5678.9", 1, Literal::Num(5678.9)),
        Token::new(TokenType::EOF, "", 1, Literal::None)])
    }

    #[test]
    fn identifier_test() {
        let scanner: Scanner = Scanner::new("and class else false for fun if nil or print return super this true var while");
        let tokens: Vec<Token> = scanner.scan_tokens().unwrap();

        assert_eq!(tokens, vec![
            Token::new(TokenType::And, "and", 1, Literal::None),
            Token::new(TokenType::Class, "class", 1, Literal::None),
            Token::new(TokenType::Else, "else", 1, Literal::None),
            Token::new(TokenType::False, "false", 1, Literal::None),
            Token::new(TokenType::For, "for", 1, Literal::None),
            Token::new(TokenType::Fun, "fun", 1, Literal::None),
            Token::new(TokenType::If, "if", 1, Literal::None),
            Token::new(TokenType::Nil, "nil", 1, Literal::None),
            Token::new(TokenType::Or, "or", 1, Literal::None),
            Token::new(TokenType::Print, "print", 1, Literal::None),
            Token::new(TokenType::Return, "return", 1, Literal::None),
            Token::new(TokenType::Super, "super", 1, Literal::None),
            Token::new(TokenType::This, "this", 1, Literal::None),
            Token::new(TokenType::True, "true", 1, Literal::None),
            Token::new(TokenType::Var, "var", 1, Literal::None),
            Token::new(TokenType::While, "while", 1, Literal::None),
            Token::new(TokenType::EOF, "", 1, Literal::None)
        ])
    }

}
use std::fmt::Alignment::Right;

use crate::token::{self, Token, TokenType};
use crate::error::{self, Error, ParserError};

/*
Operator enum describes the Lox operations, they roughly corresponds to the following lexemes
Add: +
Subract: -
Multiply: *
LessEqual: <=
Less: <
GreaterEqual: >=
Greater: >
Equal: ==
Inequal: !=
Negation: !
And: and
Or: or
*/
#[derive(Debug, PartialEq, Clone)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    LessEqual,
    Less,
    GreaterEqual,
    Greater,
    Equal,
    Inequal,
    Negation,
    And,
    Or,
}

/*
Enum for the different expressions that are contained in a lox program.
Nil, Bool, String and Numeric represent the base level expressions.
Grouping represents a group of multiple expressions
Unary expressions are expressions with only one operand. Negation is the only unary expression.
Binary expressions are expressions with two operands and one operation.
In both binary and unary expressions the operand themselves can be expressions, ergo parsing is a recursive operation.
 */
#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Binary {
        left_operand: Box<Expr>,
        operator: Operator,
        right_operand: Box<Expr>,
    },
    Numeric {
        value: String,
    },
    String {
        value: String,
    },
    Grouping {
        expr: Box<Expr>,
    },
    Nil,
    Bool {
        value: bool,
    },
    Unary {
        operator: Operator,
        right_value: Box<Expr>,
    },
}

impl Expr {
    pub fn str(value: &str) -> Expr {
        Expr::String {
            value: String::from(value),
        }
    }

    pub fn bool(value: bool) -> Expr {
        Expr::Bool { value }
    }

    pub fn numeric(value: &str) -> Expr {
        Expr::Numeric {
            value: String::from(value),
        }
    }

    pub fn nil() -> Expr {
        Expr::Nil
    }

    pub fn binary(left: Expr, operator: Operator, right: Expr) -> Expr {
        Expr::Binary {
            left_operand: left.into(),
            operator,
            right_operand: right.into(),
        }
    }

    pub fn unary(operator: Operator, right: Expr) -> Expr {
        Expr::Unary {
            operator,
            right_value: right.into(),
        }
    }

    pub fn grouping(expresion: Expr) -> Expr {
        Expr::Grouping { expr: expresion.into() }
    }
}

impl From<&Token> for Operator {
    fn from(token: &Token) -> Self {
        match token.token_type {
            TokenType::Minus => Operator::Subtract,
            TokenType::Plus => Operator::Add,
            TokenType::Slash => Operator::Divide,
            TokenType::Star => Operator::Multiply,
            TokenType::Bang => Operator::Negation,
            TokenType::BangEqual => Operator::Inequal,
            TokenType::EqualEqual => Operator::Equal,
            TokenType::Greater => Operator::Greater,
            TokenType::GreaterEqual => Operator::GreaterEqual,
            TokenType::Less => Operator::Less,
            TokenType::LessEqual => Operator::Equal,
            _ => panic!("Invalid operator"),
        }
    }
}

#[derive(Debug)]
struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens: tokens,
            current: 0,
        }
    }

    //Token parsing
    // Being the terminals of the expression tree, we start by building the parsing of the base level/literal expressions
    fn primary(&mut self) -> Result<Expr, Error> {
        if self.match_tokens([TokenType::Nil]) {
            Ok(Expr::nil())
        } else if self.match_tokens([TokenType::RightParen]) {
            let grouped_expression = self.expression()?;
            self.consume(TokenType::RightParen, String::from("Error"))?;
            Ok(Expr::grouping(grouped_expression))
        } else if self.match_tokens([TokenType::Number]) {
            let lexeme: String = self.previous().lexeme;
            Ok(Expr::numeric(&lexeme))
        } else if self.match_tokens([TokenType::String]) {
            let lexeme: String = self.previous().lexeme;
            Ok(Expr::str(&lexeme))
        } else if self.match_tokens([TokenType::False]) { 
            Ok(Expr::bool(false))
        } else if self.match_tokens([TokenType::True]){
            Ok(Expr::bool(true))
         } else {
            Err(Error::ParserError { error_type: ParserError::SyntaxError })
        }
    }

    /* 
    An unary expression has one operand and one operation.
    The operations are the negation (!) and making the value negative (-)
    */
    fn unary(&mut self) -> Result<Expr, Error> {
        if self.match_tokens([TokenType::Bang, TokenType::Minus]) {
            let operator = Operator::from(&self.previous());
            let right_operand: Expr = self.unary()?;
            Ok(Expr::unary(operator, right_operand))
        } else {
            Err(Error::syntax_error())
        }
    }

    /*
    For the binary expressions, we need to implement the order of precedence and left-associativite. We could build one method that process it, but we will stick to the canonical jlox implementation
    That means, different methods for the different binary expressions.
    We also include the highest level method: the expression method.
    */

    fn expression(&mut self) -> Result<Expr, Error> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Expr, Error> {
        let left: Expr = self.comparison()?;
        if self.match_tokens([TokenType::BangEqual, TokenType::EqualEqual]) {
            let operator: Operator = Operator::from(&self.previous());
            let right: Expr = self.comparison()?;
            Ok(Expr::binary(left, operator, right))
        } else {
            Err(Error::syntax_error())
        }
    }

    fn comparison(&mut self) -> Result<Expr, Error> {
        let left: Expr = self.term()?;
        if self.match_tokens([TokenType::Greater, TokenType::GreaterEqual, TokenType::Less, TokenType::LessEqual]) {
            let operator: Operator = Operator::from(&self.previous());
            let right: Expr = self.term()?;
            Ok(Expr::binary(left, operator, right))
        } else {
            Err(Error::syntax_error())
        }
    }

    fn term(&mut self) -> Result<Expr, Error> {
        let left: Expr = self.factor()?;
        if self.match_tokens([TokenType::Plus, TokenType::Minus]) {
            let operator: Operator = Operator::from(&self.previous());
            let right: Expr = self.factor()?;
            Ok(Expr::binary(left, operator, right))
        } else {
            Err(Error::syntax_error())
        }
    }

    fn factor(&mut self) -> Result<Expr, Error> {
        let left: Expr = self.unary()?;
        if self.match_tokens([TokenType::Slash, TokenType::Star]) {
            let operator: Operator = Operator::from(&self.previous());
            let right: Expr = self.unary()?;
            Ok (Expr::binary(left, operator, right))
        } else {
            Err(Error::syntax_error())
        }
    }


    //Helper methods
    fn is_at_end(&self) -> bool {
        self.peek() == TokenType::EOF
    }

    fn match_tokens<const N: usize>(&mut self, token_types: [TokenType; N]) -> bool {
        if !self.is_at_end() && token_types.contains(&self.tokens[self.current].token_type) {
            self.current +=1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> TokenType {
        self.tokens[self.current].token_type.clone()
    }

    fn check_token(&mut self, token_type: TokenType) -> bool {
        if !self.is_at_end() && self.tokens[self.current].token_type == token_type {
            self.current +=1;
            true
        } else {
            false
        }
    }

    fn previous(&self) -> Token {
        self.tokens[self.current -1].clone()
    }

    fn consume(&mut self, token_type: TokenType, message: String) -> Result<(), Error> {
        if !self.check_token(token_type.clone()) {
            Err(Error::syntax_error())
        } else {
            Ok(())
        }
    }
}

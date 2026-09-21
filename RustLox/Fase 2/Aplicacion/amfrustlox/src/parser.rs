use crate::token::{self, Token, TokenType};

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
    Or
}


#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Binary {
        left_operand: Box<Expr>,
        operator: Operator,
        right_operand: Box<Expr>
    },
    Numeric {
        value: String
    },
    String {
        value: String
    },
    Grouping {
        expr: Box<Expr>
    },
    Nil,
    Bool {
        value: bool
    },
    Unary {
        operator: Operator,
        right_value: Box<Expr>
    }
}

impl Expr {
    pub fn str(value: &str) -> Expr {
        Expr::String { value: String::from(value) }
    }

    pub fn bool(value: bool) -> Expr {
        Expr::Bool { value }
    }

    pub fn numeric(value: &str) -> Expr {
        Expr::Numeric { value: String::from(value) }
    }

    pub fn nil() -> Expr {
        Expr::Nil
    }

    pub fn binary(left: Expr, operator: Operator, right: Expr)  -> Expr {
        Expr::Binary { left_operand: left.into(), operator, right_operand: right.into() }
    }

    pub fn unary(operator: Operator, right: Expr) -> Expr {
        Expr::Unary { operator, right_value: right.into() }
    }
}

#[derive(Debug)]
struct Parser {
    tokens: Vec<Token>,
    position: usize
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens: tokens,
            position: 0
        }
    }

    //Expressions parsing

    //Helper methods
    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len() || self.tokens[self.position].token_type == TokenType::EOF 
    }
}

#[cfg(test)]
mod test {
    use crate::token;
    use super::*;
}
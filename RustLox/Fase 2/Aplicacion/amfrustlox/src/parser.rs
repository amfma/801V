use crate::error::Error;
use crate::token::{Literal, Token, TokenType};

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
    Variable {
        name: Token,
    },
    Assign {
        name: Token,
        value: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        paren: Token,
        arguments: Vec<Expr>,
    },
    Get {
        expr: Box<Expr>,
        name: Token,
    },
    Set {
        expr: Box<Expr>,
        name: Token,
        value: Box<Expr>,
    },
    This {
        keyword: Token,
    },
    Super {
        keyword: Token,
        method: Token,
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
        Expr::Grouping {
            expr: expresion.into(),
        }
    }

    pub fn variable(name: Token) -> Expr {
        Expr::Variable { name: name }
    }

    pub fn assign(name: Token, value: Expr) -> Expr {
        Expr::Assign {
            name,
            value: value.into(),
        }
    }

    pub fn call(callee: Expr, paren: Token, arguments: Vec<Expr>) -> Expr {
        Expr::Call {
            callee: callee.into(),
            paren,
            arguments,
        }
    }

    pub fn get(expr: Expr, name: Token) -> Expr {
        Expr::Get {
            expr: expr.into(),
            name,
        }
    }

    pub fn set(expr: Expr, name: Token, value: Expr) -> Expr {
        Expr::Set {
            expr: expr.into(),
            name,
            value: value.into(),
        }
    }

    pub fn this(keyword: Token) -> Expr {
        Expr::This { keyword }
    }

    pub fn superkw(keyword: Token, method: Token) -> Expr {
        Expr::Super { keyword, method }
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
            TokenType::LessEqual => Operator::LessEqual,
            TokenType::And => Operator::And,
            TokenType::Or => Operator::Or,
            _ => panic!("Invalid operator"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expression {
        expr: Expr,
    },
    Print {
        expr: Expr,
    },
    Var {
        name: Token,
        initializar: Option<Expr>,
    },
    Block {
        statements: Vec<Stmt>,
    },
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        contion: Expr,
        body: Box<Stmt>,
    },
    Function {
        name: Token,
        params: Vec<Token>,
        body: Vec<Stmt>,
    },
    Return {
        keyword: Token,
        value: Expr,
    },
    Class {
        name: Token,
        methods: Vec<Stmt>,
        superclass: Option<Expr>,
    },
}

/// Maximum number of parameters / arguments, as in the reference Lox.
const MAX_ARITY: usize = 255;

#[derive(Debug)]
pub struct Parser {
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
        } else if self.match_tokens([TokenType::LeftParen]) {
            let grouped_expression = self.expression()?;
            self.consume(TokenType::RightParen, "Expect ')' after expression.")?;
            Ok(Expr::grouping(grouped_expression))
        } else if self.match_tokens([TokenType::Number]) {
            let lexeme: String = self.previous().lexeme;
            Ok(Expr::numeric(&lexeme))
        } else if self.match_tokens([TokenType::String]) {
            match self.previous().literal {
                Literal::Str(value) => Ok(Expr::str(&value)),
                _ => Err(self.error_at_previous("Invalid string literal.")),
            }
        } else if self.match_tokens([TokenType::False]) {
            Ok(Expr::bool(false))
        } else if self.match_tokens([TokenType::True]) {
            Ok(Expr::bool(true))
        } else if self.match_tokens([TokenType::Identifier]) {
            let token: Token = self.previous().clone();
            Ok(Expr::variable(token))
        } else if self.match_tokens([TokenType::This]) {
            Ok(Expr::this(self.previous().clone()))
        } else if self.match_tokens([TokenType::Super]) {
            let keyword = self.previous().clone();
            self.consume(TokenType::Dot, "Expect '.' after 'super'.")?;
            self.consume(TokenType::Identifier, "Expect superclass method name.")?;
            let method = self.previous().clone();
            Ok(Expr::superkw(keyword, method))
        } else {
            Err(self.error_at_current("Expect expression."))
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
            self.call()
        }
    }

    /*
    Call operation, used to call a function.
     */

    fn call(&mut self) -> Result<Expr, Error> {
        let mut expr = self.primary()?;

        loop {
            if self.match_tokens([TokenType::LeftParen]) {
                expr = self.finish_call(expr)?;
            } else if self.match_tokens([TokenType::Dot]) {
                self.consume(TokenType::Identifier, "Expect property name after '.'.")?;
                let name = self.previous().clone();
                expr = Expr::get(expr, name)
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> Result<Expr, Error> {
        let mut arguments = vec![];
        if !self.check(TokenType::RightParen) {
            loop {
                if arguments.len() >= MAX_ARITY {
                    return Err(self.error_at_current("Can't have more than 255 arguments."));
                }
                arguments.push(self.expression()?);
                if !self.match_tokens([TokenType::Comma]) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expect ')' after arguments.")?;
        let paren = self.previous().clone();

        Ok(Expr::call(callee, paren, arguments))
    }

    /*
    For the binary expressions, we need to implement the order of precedence and left-associativite. We could build one method that process it, but we will stick to the canonical jlox implementation
    That means, different methods for the different binary expressions.
    We also include the highest level method: the expression method.
    */

    fn expression(&mut self) -> Result<Expr, Error> {
        self.assignment()
    }

    fn equality(&mut self) -> Result<Expr, Error> {
        let mut left: Expr = self.comparison()?;
        while self.match_tokens([TokenType::BangEqual, TokenType::EqualEqual]) {
            let operator: Operator = Operator::from(&self.previous());
            let right: Expr = self.comparison()?;
            left = Expr::binary(left, operator, right);
        }
        Ok(left)
    }

    fn comparison(&mut self) -> Result<Expr, Error> {
        let mut left: Expr = self.term()?;
        while self.match_tokens([
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let operator: Operator = Operator::from(&self.previous());
            let right: Expr = self.term()?;
            left = Expr::binary(left, operator, right);
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Expr, Error> {
        let mut left: Expr = self.factor()?;
        while self.match_tokens([TokenType::Plus, TokenType::Minus]) {
            let operator: Operator = Operator::from(&self.previous());
            let right: Expr = self.factor()?;
            left = Expr::binary(left, operator, right);
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<Expr, Error> {
        let mut left: Expr = self.unary()?;
        while self.match_tokens([TokenType::Slash, TokenType::Star]) {
            let operator: Operator = Operator::from(&self.previous());
            let right: Expr = self.unary()?;
            left = Expr::binary(left, operator, right);
        }
        Ok(left)
    }

    fn assignment(&mut self) -> Result<Expr, Error> {
        let expr = self.or()?;

        if self.match_tokens([TokenType::Equal]) {
            let equal = self.previous().clone();
            let value = self.assignment()?;

            match expr {
                Expr::Variable { name } => Ok(Expr::assign(name, value)),
                Expr::Get { expr, name } => Ok(Expr::set(*expr, name, value)),
                _ => Err(Error::syntax(&equal, "Invalid assignment target.")),
            }
        } else {
            Ok(expr)
        }
    }

    fn or(&mut self) -> Result<Expr, Error> {
        let mut left = self.and()?;

        while self.match_tokens([TokenType::Or]) {
            let operator = Operator::from(&self.previous());
            let right = self.and()?;
            left = Expr::binary(left, operator, right)
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, Error> {
        let mut left = self.equality()?;

        while self.match_tokens([TokenType::And]) {
            let operator = Operator::from(&self.previous());
            let right = self.equality()?;
            left = Expr::binary(left, operator, right);
        }

        Ok(left)
    }

    /* Statements */
    fn statement(&mut self) -> Result<Stmt, Error> {
        if self.match_tokens([TokenType::Print]) {
            self.print_statement()
        } else if self.match_tokens([TokenType::Return]) {
            self.return_statement()
        } else if self.match_tokens([TokenType::LeftBrace]) {
            Ok(Stmt::Block {
                statements: self.block()?,
            })
        } else if self.match_tokens([TokenType::If]) {
            self.if_statement()
        } else if self.match_tokens([TokenType::While]) {
            self.while_statement()
        } else if self.match_tokens([TokenType::For]) {
            self.for_statement()
        } else {
            self.expression_statement()
        }
    }

    fn print_statement(&mut self) -> Result<Stmt, Error> {
        let value: Expr = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        Ok(Stmt::Print { expr: value })
    }

    fn expression_statement(&mut self) -> Result<Stmt, Error> {
        let expression: Expr = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after expression.")?;
        Ok(Stmt::Expression { expr: expression })
    }

    fn block(&mut self) -> Result<Vec<Stmt>, Error> {
        let mut statements: Vec<Stmt> = vec![];

        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.")?;

        Ok(statements)
    }

    fn if_statement(&mut self) -> Result<Stmt, Error> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after if condition.")?;

        let then_branch = self.statement()?;
        let mut else_branch = None;

        if self.match_tokens([TokenType::Else]) {
            else_branch = Some(self.statement()?);
        }
        let else_branch = else_branch.map(|x| Box::new(x));

        Ok(Stmt::If {
            condition,
            then_branch: then_branch.into(),
            else_branch: else_branch,
        })
    }

    fn while_statement(&mut self) -> Result<Stmt, Error> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition.")?;

        let body = self.statement()?;

        Ok(Stmt::While {
            contion: condition,
            body: Box::new(body),
        })
    }

    fn for_statement(&mut self) -> Result<Stmt, Error> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.")?;

        let initializer = if self.match_tokens([TokenType::Semicolon]) {
            None
        } else if self.match_tokens([TokenType::Var]) {
            Some(self.var_declaration()?)
        } else {
            Some(self.expression_statement()?)
        };

        let condition = if !self.check(TokenType::Semicolon) {
            self.expression()?
        } else {
            Expr::Bool { value: true }
        };
        self.consume(TokenType::Semicolon, "Expect ';' after loop condition.")?;

        let increment = if !self.check(TokenType::RightParen) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(TokenType::RightParen, "Expect ')' after for clauses.")?;

        let mut body = self.statement()?;
        if let Some(increment) = increment {
            body = Stmt::Block {
                statements: vec![body, Stmt::Expression { expr: increment }],
            };
        }

        let looped = Stmt::While {
            contion: condition,
            body: Box::new(body),
        };

        Ok(match initializer {
            Some(init) => Stmt::Block {
                statements: vec![init, looped],
            },
            None => looped,
        })
    }

    fn return_statement(&mut self) -> Result<Stmt, Error> {
        let keyword: Token = self.previous().clone();
        let mut value = Expr::Nil;
        if !self.check(TokenType::Semicolon) {
            value = self.expression()?;
        }

        self.consume(TokenType::Semicolon, "Expect ';' after return value.")?;
        Ok(Stmt::Return { keyword, value })
    }

    /* Declarations */
    fn declaration(&mut self) -> Result<Stmt, Error> {
        if self.match_tokens([TokenType::Class]) {
            self.class_declaration()
        } else if self.match_tokens([TokenType::Fun]) {
            self.function("function")
        } else if self.match_tokens([TokenType::Var]) {
            self.var_declaration()
        } else {
            self.statement()
        }
    }

    fn class_declaration(&mut self) -> Result<Stmt, Error> {
        self.consume(TokenType::Identifier, "Expect class name.")?;
        let name = self.previous().clone();

        let superclass = if self.match_tokens([TokenType::Less]) {
            self.consume(TokenType::Identifier, "Expect superclass name.")?;
            Some(Expr::variable(self.previous().clone()))
        } else {
            None
        };

        self.consume(TokenType::LeftBrace, "Expect '{' before class body.")?;
        let mut methods = vec![];
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            methods.push(self.function("method")?);
        }

        self.consume(TokenType::RightBrace, "Expect '}' after class body.")?;
        Ok(Stmt::Class {
            name,
            methods,
            superclass,
        })
    }

    fn var_declaration(&mut self) -> Result<Stmt, Error> {
        self.consume(TokenType::Identifier, "Expect variable name.")?;
        let name = self.previous();

        let mut initializer = None;

        if self.match_tokens([TokenType::Equal]) {
            initializer = Some(self.expression()?);
        }

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        )?;

        Ok(Stmt::Var {
            name: name,
            initializar: initializer,
        })
    }

    fn function(&mut self, kind: &str) -> Result<Stmt, Error> {
        self.consume(TokenType::Identifier, &format!("Expect {kind} name."))?;
        let name = self.previous().clone();
        self.consume(
            TokenType::LeftParen,
            &format!("Expect '(' after {kind} name."),
        )?;

        let mut parameters: Vec<Token> = vec![];
        if !self.check(TokenType::RightParen) {
            loop {
                if parameters.len() >= MAX_ARITY {
                    return Err(self.error_at_current("Can't have more than 255 parameters."));
                }
                self.consume(TokenType::Identifier, "Expect parameter name.")?;
                parameters.push(self.previous().clone());

                if !self.match_tokens([TokenType::Comma]) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expect ')' after parameters.")?;

        self.consume(
            TokenType::LeftBrace,
            &format!("Expect '{{' before {kind} body."),
        )?;

        let body = self.block()?;

        Ok(Stmt::Function {
            name,
            params: parameters,
            body,
        })
    }

    //Helper methods
    fn is_at_end(&self) -> bool {
        self.peek() == TokenType::EOF
    }

    fn match_tokens<const N: usize>(&mut self, token_types: [TokenType; N]) -> bool {
        if !self.is_at_end() && token_types.contains(&self.tokens[self.current].token_type) {
            self.current += 1;
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
            self.current += 1;
            true
        } else {
            false
        }
    }

    fn check(&mut self, token_type: TokenType) -> bool {
        !self.is_at_end() && self.tokens[self.current].token_type == token_type
    }

    fn previous(&self) -> Token {
        self.tokens[self.current - 1].clone()
    }

    /// Syntax error located at the token about to be consumed.
    fn error_at_current(&self, message: &str) -> Error {
        Error::syntax(&self.tokens[self.current], message)
    }

    /// Syntax error located at the token that was just consumed.
    fn error_at_previous(&self, message: &str) -> Error {
        Error::syntax(&self.tokens[self.current - 1], message)
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<(), Error> {
        if self.check_token(token_type) {
            Ok(())
        } else {
            Err(self.error_at_current(message))
        }
    }

    /* Output methods. This methods generate an output to be used by the interpreter
    parse_expression is used for the simple evaluator/interpreter,
    parse is the final method, it returns a collection of statements to be consumed by the interpreter */
    pub fn parse_expression(&mut self) -> Result<Expr, Error> {
        self.expression()
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, Error> {
        let mut statements: Vec<Stmt> = vec![];
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        Ok(statements)
    }
}

pub struct ast {
    pub start: Vec<Expr>,
}

#[cfg(test)]
mod tests;

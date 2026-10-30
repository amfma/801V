use std::{collections::HashMap, hash::Hash};

use crate::{error::Error, interpreter::Interpreter, parser::{Expr, Stmt}, token::Token};

enum State {
    Defined,
    Declared,
}

#[derive(Copy, Clone)]
enum FunctionType {
    None,
    Function,
    Initializer
}

pub struct Resolver<'a> {
    interpreter: &'a mut Interpreter,
    scopes: Vec<HashMap<String, State>>,
    current_function: FunctionType
}

impl<'a> Resolver<'a> {
    pub fn new(interpreter: &'a mut Interpreter) -> Self {
        Resolver { interpreter, 
            scopes: vec![],
            current_function: FunctionType::None } 
    }

    pub fn resolve(&mut self, statements: &Vec<Stmt>) -> Result<Vec<Stmt>, Error> {
        self.resolve_statements(statements)?;
        Ok(statements.clone())
    }

    fn resolve_statements(&mut self, statements: &Vec<Stmt>) -> Result<(), Error> {
        for statement in statements {
            self.resolve_statement(statement)?;
        }
        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop().unwrap();
    }

    fn declare(&mut self, name: &Token) -> Result<(), Error> {
        if self.scopes.is_empty() {
            return Ok(());
        }

        let last = self.scopes.last_mut().unwrap();
        if last.contains_key(&name.lexeme) {
            Err(Error::RuntimeError)
        } else {
            last.insert(name.lexeme.clone(), State::Declared);
            Ok(())
        }
    }

    fn define(&mut self, name: &Token) {
        if !self.scopes.is_empty() {
            self.scopes.last_mut().unwrap().insert(name.lexeme.clone(), State::Defined);
        }
    }

    fn resolve_local(&mut self, name: &Token) {
        for i in (0..self.scopes.len()).rev() {
            if self.scopes[i].contains_key(&name.lexeme) {
                self.interpreter.resolve(name, self.scopes.len()-1-i);
                return;
            }
        }
    }

    fn resolve_function(&mut self, _name: &Token, parameters: &Vec<Token>, body: &Vec<Stmt>, function_type: FunctionType) -> Result<(), Error> {
        let enclosing = self.current_function;
        self.current_function = function_type;

        self.begin_scope();
        for param in parameters {
            self.declare(param)?;
            self.define(param);
        }

        self.resolve_statements(body)?;
        self.end_scope();
        self.current_function = enclosing;
        Ok(())

    }

    fn resolve_statement(&mut self, statement: &Stmt) -> Result<(), Error> {
        match statement {
            Stmt::Expression { expr } => {
                self.resolve_expression(expr)?;
            },
            Stmt::Print { expr } => self.resolve_expression(expr)?,
            Stmt::Var { name, initializar } => {
                self.declare(name)?;
                match initializar {
                    Some(expression) => self.resolve_expression(expression)?,
                    None => self.resolve_expression(&Expr::Nil)?,
                };
                self.define(name);
            },
            Stmt::Block { statements } => {
                self.begin_scope();
                self.resolve_statements(statements)?;
                self.end_scope();
            },
            Stmt::If { condition, then_branch, else_branch } => {
                self.resolve_expression(condition)?;
                self.resolve_statement(then_branch)?;
                match else_branch {
                    Some(statement) => self.resolve_statement(statement)?,
                    None => return Ok(()),
                };
                
            },
            Stmt::While { contion, body } => {
                self.resolve_expression(contion)?;
                self.resolve_statement(body)?
            },
            Stmt::Function { name, params, body } => {
                self.declare(&name)?;
                self.define(&name);
                self.resolve_function(name, params, body, FunctionType::Function)?;
            },
            Stmt::Return { keyword, value } => {
                match value {
                    Expr::Nil => {
                        return Ok(());
                    },
                    _ => {
                        if let FunctionType::Initializer = self.current_function {
                            return Err(Error:: RuntimeError)
                        } else {
                            self.resolve_expression(value)?;
                        }
                    }
                }
            },
        }
        Ok(())
    }

    fn resolve_expression(&mut self, expr: &Expr) -> Result<(), Error> {
        match expr {
            Expr::Binary { left_operand, operator, right_operand } => {
                self.resolve_expression(left_operand)?;
                self.resolve_expression(right_operand)
            },
            Expr::Numeric { value } => Ok(()),
            Expr::String { value } => Ok(()),
            Expr::Grouping { expr } => self.resolve_expression(expr),
            Expr::Nil => Ok(()),
            Expr::Bool { value } => Ok(()),
            Expr::Unary { operator, right_value } => {
                self.resolve_expression(right_value)
            },
            Expr::Variable { name } => {
                if let Some(last) = self.scopes.last() {
                    if let Some(State::Declared) = last.get(&name.lexeme) {
                        return Err(Error::RuntimeError)
                    }
                }

                self.resolve_local(name);
                Ok(())
            },
            Expr::Assign { name, value } => {
                self.resolve_expression(value)?;
                self.resolve_local(name);
                Ok(())
            },
            Expr::Call { callee, paren, arguments } => {
                self.resolve_expression(callee)?;
                for argument in arguments {
                    self.resolve_expression(argument)?;
                }
                Ok(())
            },
        }
    }
}
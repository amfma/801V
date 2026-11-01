use std::{collections::HashMap, hash::Hash};

use crate::{
    callable::Class,
    error::Error::{self, RuntimeError},
    interpreter::Interpreter,
    parser::{Expr, Stmt},
    token::Token,
};

enum State {
    Defined,
    Declared,
}

#[derive(Copy, Clone, Debug)]
enum FunctionType {
    None,
    Function,
    Initializer,
    Method,
}

#[derive(Copy, Clone, Debug)]
enum ClassType {
    None,
    Class,
    Subclass,
}

pub struct Resolver<'a> {
    interpreter: &'a mut Interpreter,
    scopes: Vec<HashMap<String, State>>,
    current_function: FunctionType,
    current_class: ClassType,
}

impl<'a> Resolver<'a> {
    pub fn new(interpreter: &'a mut Interpreter) -> Self {
        Resolver {
            interpreter,
            scopes: vec![],
            current_function: FunctionType::None,
            current_class: ClassType::None,
        }
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
            self.scopes
                .last_mut()
                .unwrap()
                .insert(name.lexeme.clone(), State::Defined);
        }
    }

    fn resolve_local(&mut self, name: &Token) {
        for i in (0..self.scopes.len()).rev() {
            if self.scopes[i].contains_key(&name.lexeme) {
                self.interpreter.resolve(name, self.scopes.len() - 1 - i);
                return;
            }
        }
    }

    fn resolve_function(
        &mut self,
        _name: &Token,
        parameters: &Vec<Token>,
        body: &Vec<Stmt>,
        function_type: FunctionType,
    ) -> Result<(), Error> {
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
            }
            Stmt::Print { expr } => self.resolve_expression(expr)?,
            Stmt::Var { name, initializar } => {
                self.declare(name)?;
                match initializar {
                    Some(expression) => self.resolve_expression(expression)?,
                    None => self.resolve_expression(&Expr::Nil)?,
                };
                self.define(name);
            }
            Stmt::Block { statements } => {
                self.begin_scope();
                self.resolve_statements(statements)?;
                self.end_scope();
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expression(condition)?;
                self.resolve_statement(then_branch)?;
                match else_branch {
                    Some(statement) => self.resolve_statement(statement)?,
                    None => return Ok(()),
                };
            }
            Stmt::While { contion, body } => {
                self.resolve_expression(contion)?;
                self.resolve_statement(body)?
            }
            Stmt::Function { name, params, body } => {
                self.declare(&name)?;
                self.define(&name);
                self.resolve_function(name, params, body, FunctionType::Function)?;
            }
            Stmt::Return { keyword, value } => match value {
                Expr::Nil => {
                    return Ok(());
                }
                _ => {
                    if let FunctionType::Initializer = self.current_function {
                        return Err(Error::RuntimeError);
                    } else {
                        self.resolve_expression(value)?;
                    }
                }
            },
            Stmt::Class {
                name,
                methods,
                superclass,
            } => {
                let enclosing_class = self.current_class;
                self.current_class = ClassType::Class;
                self.declare(name)?;
                self.define(name);

                if let Some(superclass_expr) = superclass {
                    if let Expr::Variable { name:superclass_name } = superclass_expr {
                        if name.lexeme == superclass_name.lexeme {
                            return Err(Error::RuntimeError)
                        }
                    }
                    self.current_class = ClassType::Subclass;
                    self.resolve_expression(superclass_expr)?;

                    self.begin_scope();
                    self.scopes.last_mut().unwrap().insert("super".to_string(), State::Defined);
                }

                self.begin_scope();
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert("this".to_string(), State::Defined);

                for method in methods {
                    if let Stmt::Function { name, params, body } = method {
                        let function_type = if name.lexeme == "init" {
                            FunctionType::Initializer
                        } else {
                            FunctionType::Method
                        };
                        self.resolve_function(name, params, body, function_type)?;
                    } else {
                        panic!("Unexpected statement in class body")
                    }
                }
                self.end_scope();

                if superclass.is_some() {
                    self.end_scope();
                }
                self.current_class = enclosing_class;
            }
        }
        Ok(())
    }

    fn resolve_expression(&mut self, expr: &Expr) -> Result<(), Error> {
        match expr {
            Expr::Binary {
                left_operand,
                operator,
                right_operand,
            } => {
                self.resolve_expression(left_operand)?;
                self.resolve_expression(right_operand)
            }
            Expr::Numeric { value } => Ok(()),
            Expr::String { value } => Ok(()),
            Expr::Grouping { expr } => self.resolve_expression(expr),
            Expr::Nil => Ok(()),
            Expr::Bool { value } => Ok(()),
            Expr::Unary {
                operator,
                right_value,
            } => self.resolve_expression(right_value),
            Expr::Variable { name } => {
                if let Some(last) = self.scopes.last() {
                    if let Some(State::Declared) = last.get(&name.lexeme) {
                        return Err(Error::RuntimeError);
                    }
                }

                self.resolve_local(name);
                Ok(())
            }
            Expr::Assign { name, value } => {
                self.resolve_expression(value)?;
                self.resolve_local(name);
                Ok(())
            }
            Expr::Call {
                callee,
                paren,
                arguments,
            } => {
                self.resolve_expression(callee)?;
                for argument in arguments {
                    self.resolve_expression(argument)?;
                }
                Ok(())
            }
            Expr::Get { expr, name } => self.resolve_expression(expr),
            Expr::Set { expr, name, value } => {
                self.resolve_expression(expr)?;
                self.resolve_expression(value)
            }
            Expr::This { keyword } => {
                if let ClassType::None = self.current_class {
                    return Err(Error::RuntimeError);
                }
                self.resolve_local(keyword);
                Ok(())
            }
            Expr::Super { keyword, method } => match self.current_class {
                ClassType::None => Err(Error::RuntimeError),
                ClassType::Class => Err(Error::RuntimeError),
                ClassType::Subclass => {
                    self.resolve_local(keyword);
                    Ok(())
                }
            },
        }
    }
}

   #[cfg(test)]
   mod tests;
use std::{cell::RefCell, fmt::{format, write}, rc::Rc};

use crate::{enviroment::Enviroment, error::{self, Error}, parser::{self, Expr, Operator, Stmt}};

type Output = Result<Value, error::Error>;

pub struct Interpreter {
    gobal: Enviroment
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Value {
    Nil,
    Boolean(bool),
    String(String),
    Number(f64)
}

impl Value {
    pub fn is_truthy(&self ) -> bool {
        match &self {
            Value::Nil | Value::Boolean(false) => false,
            _ => true
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Value::Nil => write!(f, "nil"),
            Value::Boolean(value) => write!(f, "{value}", ),
            Value::String(value) => write!(f, "{value}"),
            Value::Number(value) => write!(f, "{value}"),
        }
    }
} 


impl Interpreter {
    pub fn new() -> Self {
        Self {
            gobal : Enviroment::new(),
        }
    }

    fn execute_statement(&mut self, statement: &Stmt) -> Result<(), Error> {
        match statement {
            Stmt::Expression { expr } => {
                self.evaluate_expression(expr)?;
            },
            Stmt::Print { expr } => {
                let value = self.evaluate_expression(expr)?;
                println!("{}", value);
            },
            Stmt::Var { name, initializar } => {
                let value = match initializar {
                    Some(value) => self.evaluate_expression(value)?,
                    None => Value::Nil,
                };

                self.gobal.define(name.to_string(), value);
            },
            Stmt::Block { statements } => {
                let env = Enviroment::new();
                self.execute_block(statements, env);
            },
            Stmt::If { condition, then_branch, else_branch } => {
                if self.evaluate_expression(condition)?.is_truthy() {
                    self.execute_statement(then_branch)?;
                } else if else_branch.is_some() {
                    self.execute_statement(then_branch)?;
                }
            },
        }

        Ok(())
    }

    fn execute_block(&mut self, statements: &Vec<Stmt>, env: Enviroment) {
        let previous = self.gobal.clone();
        self.gobal = env;
                for statement in statements {
            self.execute_statement(statement);
        }
        self.gobal = previous;
    }

    fn evaluate_expression(&mut self, expr: &Expr) -> Output {
        match expr {
            Expr::Binary { left_operand, operator, right_operand } => {
                let left = self.evaluate_expression(&left_operand)?;
                let right = self.evaluate_expression(&right_operand)?;

                match (&left, operator, &right) {
                    (Value::Number(left), Operator::Add, Value::Number(right)) => Ok(Value::Number(left + right)),
                    (Value::Number(left), Operator::Subtract, Value::Number(right)) => Ok(Value::Number(left - right)),
                    (Value::Number(left), Operator::Multiply, Value::Number(right)) => Ok(Value::Number(left * right)),
                    (Value::Number(left), Operator::Divide, Value::Number(right)) => {
                        if *right == 0.0 {
                            return Err(Error::RuntimeError);
                        } else {
                            return Ok(Value::Number(left / right));
                        }
                    },
                    (Value::String(left), Operator::Add, Value::String(right)) => Ok(Value::String(format!("{left} {right}"))),
                    (_, Operator::Greater, _) => Ok(Value::Boolean(left > right)),
                    (_, Operator::GreaterEqual, _) => Ok(Value::Boolean(left >= right)),
                    (_, Operator::Less, _) => Ok(Value::Boolean(left < right)),
                    (_, Operator::LessEqual, _) => Ok(Value::Boolean(left <= right)),
                    (_, Operator::Equal, _) => Ok(Value::Boolean(left == right)),
                    (_, Operator::Inequal, _) => Ok(Value::Boolean(left != right)),
                    (_, Operator::Or, _) => {
                        if left.is_truthy() {
                            Ok(left)
                        } else {
                            Ok(right)
                        }
                    },
                    (_, Operator::And, _) => {
                        if !left.is_truthy() {
                            Ok(left)
                        } else {
                            Ok(right)
                        }
                    },
                    _ => {
                        return Err(Error::RuntimeError)
                    }
                }
            },
            Expr::Numeric { value } => Ok(Value::Number(value.parse().unwrap())),
            Expr::String { value } => Ok(Value::String(value.clone())),
            Expr::Grouping { expr } => Ok(self.evaluate_expression(&expr)?),
            Expr::Nil => Ok(Value::Nil),
            Expr::Bool { value } => Ok(Value::Boolean(*value)),
            Expr::Unary { operator, right_value } => {
                let right = self.evaluate_expression(&right_value)?;

                match (operator, &right) {
                    (Operator::Subtract, Value::Number(right)) => Ok(Value::Number(-right)),
                    (Operator::Negation, right) => Ok(Value::Boolean(!right.is_truthy())),
                    _ => return Err(Error::RuntimeError)
                }
            },
            Expr::Variable { name } => {
                let val = self.gobal.get(name.to_string());
                match val {
                    Some(value) => Ok(value),
                    None => Err(Error::RuntimeError)
                }
            },
            Expr::Assign { name, value } => {
                let value = self.evaluate_expression(&value)?;
                self.gobal.assign(name.to_string(), value.clone());
                Ok(value)
            },
        }
    }

    pub fn interpret_expression(&mut self, expression: &Expr) {
        let result = self.evaluate_expression(expression);
        match result {
            Ok(value) => println!("{}", value),
            Err(_) => println!("Error"),
        }
    }

    pub fn interpret(&mut self, statements: &Vec<Stmt>) {
        for statement in statements {
            self.execute_statement(statement);
        }
    }
}
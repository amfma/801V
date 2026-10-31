use std::hash::Hash;
use std::ptr;
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    fmt::{Alignment::Right, format, write},
    io::Write,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};


use crate::callable::Class;
use crate::{
    callable::{self, BoxedFunction, Callable, CallableType, Function, Native, Instance},
    enviroment::Enviroment,
    error::{
        self,
        Error::{self, RuntimeError},
    },
    parser::{self, Expr, Operator, Stmt},
    token::{Token, TokenIndex},
};

type Output = Result<Value, error::Error>;

pub struct Interpreter {
    enviroment: Rc<RefCell<Enviroment>>,
    globals: Rc<RefCell<Enviroment>>,
    stdout: Rc<RefCell<dyn Write>>,
    locals: HashMap<TokenIndex, usize>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Boolean(bool),
    String(String),
    Number(f64),
    Callable(Rc<dyn Callable>),
    Instance(Rc<RefCell<Instance>>)
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean(l0), Self::Boolean(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::Number(l0), Self::Number(r0)) => l0 == r0,
            (Self::Callable(l0), Self::Callable(r0)) => Rc::ptr_eq(l0, r0),
            (Self::Instance(l0), Self::Instance(r0)) => Rc::ptr_eq(l0, r0),
            (Value::Nil, Value::Nil) => true,
            (Value::Nil, _) => false,
            (_, Value::Nil) => false,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match &self {
            Value::Nil | Value::Boolean(false) => false,
            _ => true,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Value::Nil => write!(f, "nil"),
            Value::Boolean(value) => write!(f, "{value}",),
            Value::String(value) => write!(f, "{value}"),
            Value::Number(value) => write!(f, "{value}"),
            Value::Callable(value) => write!(f, "{:?}", value),
            Value::Instance(ref_cell) => write!(f, "{}", RefCell::borrow(ref_cell)),
        }
    }
}

impl Interpreter {
    pub fn new(stdout: Rc<RefCell<dyn Write>>) -> Self {
        let globals = Rc::new(RefCell::new(Enviroment::new(None)));
        let clock_out = stdout.clone();

        let clock: BoxedFunction = Box::new(move |_| {
            let start = SystemTime::now();
            let since_epoch = start.duration_since(UNIX_EPOCH).unwrap();
            writeln!(clock_out.borrow_mut(), "{:?}\n", since_epoch).unwrap();
            Ok(Value::Nil)
        });

        let clock_callable = Native::new(clock, String::from("clock"), 0);
        globals.borrow_mut().define(
            "clock".to_string(),
            Value::Callable(Rc::new(clock_callable)),
        );

        Interpreter {
            locals: HashMap::new(),
            enviroment: globals.clone(),
            globals: globals,
            stdout,
        }
    }

    fn execute_statement(&mut self, statement: &Stmt) -> Result<(), Error> {
        match statement {
            Stmt::Expression { expr } => {
                self.evaluate_expression(expr)?;
            }
            Stmt::Print { expr } => {
                let value = self.evaluate_expression(expr)?;
                println!("{}", value);
            }
            Stmt::Var { name, initializar } => {
                let value = match initializar {
                    Some(value) => self.evaluate_expression(value)?,
                    None => Value::Nil,
                };

                self.enviroment
                    .borrow_mut()
                    .define(name.lexeme.to_string(), value);
            }
            Stmt::Block { statements } => {
                let env = self.enviroment.clone();
                self.execute_block(statements, env)?;
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if self.evaluate_expression(condition)?.is_truthy() {
                    self.execute_statement(then_branch)?;
                } else if let Some(alternative) = else_branch {
                    self.execute_statement(alternative)?;
                }
            }
            Stmt::While { contion, body } => loop {
                match self.evaluate_expression(contion) {
                    Ok(expression) => {
                        if expression.is_truthy() {
                            self.execute_statement(body)?;
                        } else {
                            return Ok(());
                        }
                    }
                    Err(_) => return Err(Error::RuntimeError),
                }
            },
            Stmt::Function { name, params, body } => {
                let boxed_function =
                    Rc::new(Function::new(self.enviroment.clone(), name, params, body));
                let callable = Value::Callable(boxed_function);
                self.enviroment
                    .borrow_mut()
                    .define(name.lexeme.clone(), callable);
            }
            Stmt::Return { keyword, value } => {
                let value = self.evaluate_expression(value)?;
                return Err(Error::Return(value));
            }
            Stmt::Class { name, methods } => {
                self.enviroment.borrow_mut().define(name.lexeme.clone(), Value::Nil);
                let closure = Rc::new(RefCell::new(Enviroment::new(Some(Rc::clone(&self.enviroment)))));
                let mut final_methods = HashMap::new();
                for method in methods {
                    if let Stmt::Function { name, params, body } = method {
                        final_methods.insert(
                            name.lexeme.clone(), 
                        Rc::new(Function::new(Rc::clone(&self.enviroment), name, params, body)));
                    } else {
                        panic!("Unexpected statement in class body")
                    }
                }

                let class = Class::new(name.lexeme.clone(), final_methods);

                if self.enviroment.borrow_mut().assign(name.lexeme.clone(), Value::Callable(class)).is_err() {
                    return Err(Error::RuntimeError);
                }
                
            },
        }

        Ok(())
    }

    fn execute_block(
        &mut self,
        statements: &Vec<Stmt>,
        env: Rc<RefCell<Enviroment>>,
    ) -> Result<(), Error> {
        let new_env = Rc::new(RefCell::new(Enviroment::new(Some(env))));
        self.execute_function_block(statements, new_env)
    }

    pub fn execute_function_block(
        &mut self,
        statements: &Vec<Stmt>,
        env: Rc<RefCell<Enviroment>>,
    ) -> Result<(), Error> {
        let current = self.enviroment.clone();
        self.enviroment = env;

        for statement in statements {
            if let err @ Err(_) = self.execute_statement(statement) {
                self.enviroment = current;
                return err;
            }
        }

        self.enviroment = current;
        Ok(())
    }

    fn evaluate_expression(&mut self, expr: &Expr) -> Output {
        match expr {
            Expr::Binary {
                left_operand,
                operator,
                right_operand,
            } => {
                let left = self.evaluate_expression(&left_operand)?;
                let right = self.evaluate_expression(&right_operand)?;

                match (&left, operator, &right) {
                    (Value::Number(left), Operator::Add, Value::Number(right)) => {
                        Ok(Value::Number(left + right))
                    }
                    (Value::Number(left), Operator::Subtract, Value::Number(right)) => {
                        Ok(Value::Number(left - right))
                    }
                    (Value::Number(left), Operator::Multiply, Value::Number(right)) => {
                        Ok(Value::Number(left * right))
                    }
                    (Value::Number(left), Operator::Divide, Value::Number(right)) => {
                        if *right == 0.0 {
                            return Err(Error::RuntimeError);
                        } else {
                            return Ok(Value::Number(left / right));
                        }
                    }
                    (Value::String(left), Operator::Add, Value::String(right)) => {
                        Ok(Value::String(String::from(left) + right))
                    }
                    /* Number comparasions */
                    (Value::Number(left), Operator::LessEqual, Value::Number(right)) => {
                        Ok(Value::Boolean(left <= right))
                    }
                    (Value::Number(left), Operator::GreaterEqual, Value::Number(right)) => {
                        Ok(Value::Boolean(left >= right))
                    }
                    (Value::Number(left), Operator::Greater, Value::Number(right)) => {
                        Ok(Value::Boolean(left > right))
                    }
                    (Value::Number(left), Operator::Less, Value::Number(right)) => {
                        Ok(Value::Boolean(left < right))
                    }
                    /* String comparisions */
                    (Value::String(left), Operator::Greater, Value::String(right)) => {
                        Ok(Value::Boolean(left > right))
                    }
                    (Value::String(left), Operator::GreaterEqual, Value::String(right)) => {
                        Ok(Value::Boolean(left >= right))
                    }
                    (Value::String(left), Operator::Less, Value::String(right)) => {
                        Ok(Value::Boolean(left < right))
                    }
                    (Value::String(left), Operator::LessEqual, Value::String(right)) => {
                        Ok(Value::Boolean(left <= right))
                    }
                    /* Type agnostic comparisions */
                    (_, Operator::Equal, _) => Ok(Value::Boolean(left == right)),
                    (_, Operator::Inequal, _) => Ok(Value::Boolean(left != right)),
                    (_, Operator::Or, _) => {
                        if left.is_truthy() {
                            Ok(left)
                        } else {
                            Ok(right)
                        }
                    }
                    (_, Operator::And, _) => {
                        if !left.is_truthy() {
                            Ok(left)
                        } else {
                            Ok(right)
                        }
                    }
                    _ => return Err(Error::RuntimeError),
                }
            }
            Expr::Numeric { value } => Ok(Value::Number(value.parse().unwrap())),
            Expr::String { value } => Ok(Value::String(value.clone())),
            Expr::Grouping { expr } => Ok(self.evaluate_expression(&expr)?),
            Expr::Nil => Ok(Value::Nil),
            Expr::Bool { value } => Ok(Value::Boolean(*value)),
            Expr::Unary {
                operator,
                right_value,
            } => {
                let right = self.evaluate_expression(&right_value)?;

                match (operator, &right) {
                    (Operator::Subtract, Value::Number(right)) => Ok(Value::Number(-right)),
                    (Operator::Negation, right) => Ok(Value::Boolean(!right.is_truthy())),
                    _ => return Err(Error::RuntimeError),
                }
            }
            Expr::Variable { name } => match self.lookup_variable(name) {
                Some(value) => Ok(value),
                None => Err(Error::RuntimeError),
            },
            Expr::Assign { name, value } => {
                let value = self.evaluate_expression(value)?;
                match self.locals.get(&name.idx) {
                    Some(distance) => {
                        self.enviroment.borrow_mut().assign_at(
                            name.lexeme.clone(),
                            *distance,
                            value.clone(),
                        );
                    }
                    None => {
                        self.globals
                            .borrow_mut()
                            .assign(name.lexeme.clone(), value.clone());
                    }
                }
                Ok(value)
            }
            Expr::Call {
                callee,
                paren,
                arguments,
            } => {
                let callee = self.evaluate_expression(callee)?;
                let mut evaluated_arguments = vec![];
                for arg in arguments {
                    evaluated_arguments.push(self.evaluate_expression(arg)?);
                }
                match callee {
                    Value::Callable(function) => {
                        if function.arity() != evaluated_arguments.len() {
                            Err(Error::RuntimeError)
                        } else {
                            function.call(self, &evaluated_arguments)
                        }
                    }
                    _ => Err(Error::RuntimeError),
                }
            }
            Expr::Get { expr, name } => {
                if let Value::Instance(instance) = self.evaluate_expression(expr)? {
                    match Instance::get(&instance, name.lexeme.clone()) {
                        Some(value) => Ok(value),
                        None => Err(Error::RuntimeError)
                    }
                } else {
                    Err(Error::RuntimeError)
                }
            },
            Expr::Set { expr, name, value } => {
                match self.evaluate_expression(expr)? {
                    Value::Instance(instance) => {
                        let value = self.evaluate_expression(value)?;
                        Ok(instance.borrow_mut().set(name.lexeme.clone(), value))
                    }
                    _ => Err(Error::RuntimeError),
                }
            },
            Expr::This { keyword } => {
                match self.lookup_variable(keyword) {
                    Some(value) => Ok(value),
                    None => Err(Error::RuntimeError)
                }
            },
        }
    }

    fn lookup_variable(&self, token: &Token) -> Option<Value> {
        match self.locals.get(&token.idx) {
            Some(distance) => self
                .enviroment
                .borrow()
                .get_at(*distance, token.lexeme.clone()),
            None => self.globals.borrow().get(token.lexeme.clone()),
        }
    }

    pub fn resolve(&mut self, token: &Token, depth: usize) {
        self.locals.insert(token.idx, depth);
    }

    pub fn interpret(&mut self, statements: &Vec<Stmt>) {
        for statement in statements {
            self.execute_statement(statement);
        }
    }
}

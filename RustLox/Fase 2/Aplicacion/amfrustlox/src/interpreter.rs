use std::{
    cell::RefCell,
    collections::HashMap,
    io::Write,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    callable::{BoxedFunction, Callable, CallableType, Class, Function, Instance, Method, Native},
    enviroment::Enviroment,
    error::Error,
    fxhash::FxHashMap,
    parser::{Expr, Operator, Stmt},
    token::{Token, TokenIndex},
};

type Output = Result<Value, Error>;

pub struct Interpreter {
    enviroment: Rc<RefCell<Enviroment>>,
    globals: Rc<RefCell<Enviroment>>,
    stdout: Rc<RefCell<dyn Write>>,
    stderr: Rc<RefCell<dyn Write>>,
    locals: FxHashMap<TokenIndex, usize>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Boolean(bool),
    /// Shared, immutable text: cloning a string value (reading a variable,
    /// comparing, passing it to a function) copies a pointer, not the characters.
    String(Rc<str>),
    Number(f64),
    Callable(Rc<dyn Callable>),
    Instance(Rc<RefCell<Instance>>),
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

fn undefined_variable(name: &Token) -> Error {
    Error::runtime(name, format!("Undefined variable '{}'.", name.lexeme))
}

impl Interpreter {
    /// Diagnostics go to the process stderr.
    pub fn new(stdout: Rc<RefCell<dyn Write>>) -> Self {
        Self::with_streams(stdout, Rc::new(RefCell::new(std::io::stderr())))
    }

    /// Both output streams injected (used by tests and tooling).
    pub fn with_streams(stdout: Rc<RefCell<dyn Write>>, stderr: Rc<RefCell<dyn Write>>) -> Self {
        let globals = Rc::new(RefCell::new(Enviroment::new(None)));

        let clock: BoxedFunction = Box::new(|_| {
            let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            Ok(Value::Number(since_epoch.as_secs_f64()))
        });

        let clock_callable = Native::new(clock, String::from("clock"), 0);
        globals.borrow_mut().define(
            "clock".to_string(),
            Value::Callable(Rc::new(clock_callable)),
        );

        Interpreter {
            locals: FxHashMap::default(),
            enviroment: globals.clone(),
            globals: globals,
            stdout,
            stderr,
        }
    }

    fn execute_statement(&mut self, statement: &Stmt) -> Result<(), Error> {
        match statement {
            Stmt::Expression { expr } => {
                self.evaluate_expression(expr)?;
            }
            Stmt::Print { expr } => {
                let value = self.evaluate_expression(expr)?;
                writeln!(self.stdout.borrow_mut(), "{}", value).map_err(|_| Error::RuntimeError)?;
            }

            Stmt::Var { name, initializar } => {
                let value = match initializar {
                    Some(value) => self.evaluate_expression(value)?,
                    None => Value::Nil,
                };

                self.enviroment
                    .borrow_mut()
                    .define_str(&name.lexeme, value);
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
            Stmt::While { contion, body } => {
                while self.evaluate_expression(contion)?.is_truthy() {
                    self.execute_statement(body)?;
                }
            }
            Stmt::Function { name, params, body } => {
                let boxed_function =
                    Rc::new(Function::new(self.enviroment.clone(), name, params, body));
                let callable = Value::Callable(boxed_function);
                self.enviroment
                    .borrow_mut()
                    .define_str(&name.lexeme, callable);
            }
            Stmt::Return { keyword: _, value } => {
                let value = self.evaluate_expression(value)?;
                return Err(Error::Return(value));
            }
            Stmt::Class {
                name,
                methods,
                superclass: superclass_expr,
            } => {
                self.enviroment
                    .borrow_mut()
                    .define_str(&name.lexeme, Value::Nil);
                let closure = Rc::new(RefCell::new(Enviroment::new(Some(Rc::clone(
                    &self.enviroment,
                )))));

                let superclass = match superclass_expr {
                    None => None,
                    Some(expr) => {
                        let token = match expr {
                            Expr::Variable { name: super_name } => super_name,
                            _ => name,
                        };
                        match self.evaluate_expression(expr)? {
                            Value::Callable(callable)
                                if callable.ctype() == CallableType::Class =>
                            {
                                closure.borrow_mut().define_str(
                                    "super",
                                    Value::Callable(Rc::clone(&callable)),
                                );
                                callable.as_class()
                            }
                            _ => {
                                return Err(Error::runtime(token, "Superclass must be a class."));
                            }
                        }
                    }
                };
                let method_env = if superclass.is_some() {
                    Rc::clone(&closure)
                } else {
                    Rc::clone(&self.enviroment)
                };
                let mut final_methods = HashMap::new();
                for method in methods {
                    if let Stmt::Function { name, params, body } = method {
                        final_methods.insert(
                            name.lexeme.clone(),
                            Rc::new(Function::new(Rc::clone(&method_env), name, params, body)),
                        );
                    } else {
                        panic!("Unexpected statement in class body")
                    }
                }

                let class = Class::new(name.lexeme.clone(), final_methods, superclass);

                if self
                    .enviroment
                    .borrow_mut()
                    .assign(name.lexeme.clone(), Value::Callable(class))
                    .is_err()
                {
                    return Err(undefined_variable(name));
                }
            }
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
                line,
                column,
            } => {
                let left = self.evaluate_expression(left_operand)?;

                // Short-circuit: only evaluate the right side when it can matter.
                match operator {
                    Operator::Or => {
                        return if left.is_truthy() {
                            Ok(left)
                        } else {
                            self.evaluate_expression(right_operand)
                        };
                    }
                    Operator::And => {
                        return if !left.is_truthy() {
                            Ok(left)
                        } else {
                            self.evaluate_expression(right_operand)
                        };
                    }
                    _ => {}
                }

                let right = self.evaluate_expression(right_operand)?;

                // Fast path for the overwhelmingly common number/number case.
                if let (Value::Number(l), Value::Number(r)) = (&left, &right) {
                    let (l, r) = (*l, *r);
                    match operator {
                        Operator::Add => return Ok(Value::Number(l + r)),
                        Operator::Subtract => return Ok(Value::Number(l - r)),
                        Operator::Multiply => return Ok(Value::Number(l * r)),
                        Operator::Less => return Ok(Value::Boolean(l < r)),
                        Operator::LessEqual => return Ok(Value::Boolean(l <= r)),
                        Operator::Greater => return Ok(Value::Boolean(l > r)),
                        Operator::GreaterEqual => return Ok(Value::Boolean(l >= r)),
                        Operator::Equal => return Ok(Value::Boolean(l == r)),
                        Operator::Inequal => return Ok(Value::Boolean(l != r)),
                        _ => {}
                    }
                }
                Self::binary_slow(left, operator, right, line.0, column.0)
            }
            Expr::Numeric { value } => Ok(Value::Number(*value)),
            Expr::String { value } => Ok(Value::String(Rc::clone(value))),
            Expr::Grouping { expr } => self.evaluate_expression(expr),
            Expr::Nil => Ok(Value::Nil),
            Expr::Bool { value } => Ok(Value::Boolean(*value)),
            Expr::Unary {
                operator,
                right_value,
                line,
                column,
            } => {
                let right = self.evaluate_expression(right_value)?;
                match (operator, &right) {
                    (Operator::Subtract, Value::Number(right)) => Ok(Value::Number(-right)),
                    (Operator::Negation, right) => Ok(Value::Boolean(!right.is_truthy())),
                    _ => Err(Self::unary_error(operator, line.0, column.0)),
                }
            }
            Expr::Variable { name } => match self.lookup_variable(name) {
                Some(value) => Ok(value),
                None => Err(undefined_variable(name)),
            },
            Expr::Assign { name, value } => self.eval_assign(name, value),
            Expr::Call {
                callee,
                paren,
                arguments,
            } => self.eval_call(callee, paren, arguments),
            Expr::Get { expr, name } => self.eval_get(expr, name),
            Expr::Set { expr, name, value } => self.eval_set(expr, name, value),
            Expr::This { keyword } => match self.lookup_variable(keyword) {
                Some(value) => Ok(value),
                None => Err(undefined_variable(keyword)),
            },
            Expr::Super { keyword, method } => self.eval_super(keyword, method),
        }
    }

    #[cold]
    #[inline(never)]
    fn unary_error(operator: &Operator, line: usize, column: usize) -> Error {
        match operator {
            Operator::Subtract => Error::runtime_at_column(line, column, "Operand must be a number."),
            _ => Error::runtime_at_column(line, column, "Invalid unary operator."),
        }
    }

    /// Everything the number/number fast path does not cover (strings,
    /// division, type errors).
    #[inline(never)]
    fn binary_slow(left: Value, operator: &Operator, right: Value, line: usize, column: usize) -> Output {
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
                    Err(Error::runtime_at_column(line, column, "Division by zero."))
                } else {
                    Ok(Value::Number(left / right))
                }
            }
            (Value::String(left), Operator::Add, Value::String(right)) => {
                let mut joined = String::with_capacity(left.len() + right.len());
                joined.push_str(left);
                joined.push_str(right);
                Ok(Value::String(Rc::from(joined)))
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
            /* Everything else is a type error */
            (_, Operator::Add, _) => Err(Error::runtime_at_column(
                line,
                column,
                "Operands must be two numbers or two strings.",
            )),
            (
                _,
                Operator::Less
                | Operator::LessEqual
                | Operator::Greater
                | Operator::GreaterEqual,
                _,
            ) => Err(Error::runtime_at_column(
                line,
                column,
                "Operands must be two numbers or two strings.",
            )),
            (_, Operator::Subtract | Operator::Multiply | Operator::Divide, _) => {
                Err(Error::runtime_at_column(line, column, "Operands must be numbers."))
            }
            _ => Err(Error::runtime_at_column(line, column, "Invalid binary operator.")),
        }
    }

    #[inline(never)]
    fn eval_assign(&mut self, name: &Token, value: &Expr) -> Output {
        let value = self.evaluate_expression(value)?;
        match self.locals.get(&name.idx) {
            Some(distance) => {
                self.enviroment
                    .borrow_mut()
                    .assign_at(&name.lexeme, *distance, value.clone());
            }
            None => {
                self.globals
                    .borrow_mut()
                    .assign(&name.lexeme, value.clone())
                    .map_err(|_| undefined_variable(name))?;
            }
        }
        Ok(value)
    }

    #[inline(never)]
    fn eval_call(&mut self, callee: &Expr, paren: &Token, arguments: &[Expr]) -> Output {
        let callee = self.evaluate_expression(callee)?;
        let mut evaluated_arguments = Vec::with_capacity(arguments.len());
        for arg in arguments {
            evaluated_arguments.push(self.evaluate_expression(arg)?);
        }
        match callee {
            Value::Callable(function) => {
                if function.arity() != evaluated_arguments.len() {
                    Err(Error::runtime(
                        paren,
                        format!(
                            "Expected {} arguments but got {}.",
                            function.arity(),
                            evaluated_arguments.len()
                        ),
                    ))
                } else {
                    function
                        .call(self, &evaluated_arguments)
                        .map_err(|error| match error {
                            // Natives have no position of their own: use the call site.
                            Error::RuntimeError => {
                                Error::runtime(paren, "Native function failed.")
                            }
                            other => other,
                        })
                }
            }
            _ => Err(Error::runtime(
                paren,
                "Can only call functions and classes.",
            )),
        }
    }

    #[inline(never)]
    fn eval_get(&mut self, expr: &Expr, name: &Token) -> Output {
        match self.evaluate_expression(expr)? {
            Value::Instance(instance) => match Instance::get(&instance, &name.lexeme) {
                Some(value) => Ok(value),
                None => Err(Error::runtime(
                    name,
                    format!("Undefined property '{}'.", name.lexeme),
                )),
            },
            _ => Err(Error::runtime(name, "Only instances have properties.")),
        }
    }

    #[inline(never)]
    fn eval_set(&mut self, expr: &Expr, name: &Token, value: &Expr) -> Output {
        match self.evaluate_expression(expr)? {
            Value::Instance(instance) => {
                let value = self.evaluate_expression(value)?;
                Ok(instance.borrow_mut().set(&name.lexeme, value))
            }
            _ => Err(Error::runtime(name, "Only instances have fields.")),
        }
    }

    #[inline(never)]
    fn eval_super(&mut self, keyword: &Token, method: &Token) -> Output {
        let distance = *self.locals.get(&keyword.idx).unwrap();
        let superclass = self
            .enviroment
            .borrow()
            .get_at(distance, "super")
            .unwrap();
        let this = self
            .enviroment
            .borrow()
            .get_at(distance - 1, "this")
            .unwrap();

        match (superclass, this) {
            (Value::Callable(callable), Value::Instance(instance)) => {
                let supe = callable.as_class().unwrap();
                let fun = match supe.find_method(&method.lexeme) {
                    Some(method) => method,
                    _ => {
                        return Err(Error::runtime(
                            method,
                            format!("Undefined property '{}'.", method.lexeme),
                        ));
                    }
                };

                Ok(Value::Callable(Rc::new(Method::bind(
                    fun,
                    instance,
                    method.lexeme == "init",
                ))))
            }
            (supe, this) => panic!(
                "Unexpected literal types for super({:?}) and this({:?})",
                supe, this
            ),
        }
    }

    fn lookup_variable(&self, token: &Token) -> Option<Value> {
        match self.locals.get(&token.idx) {
            Some(distance) => self
                .enviroment
                .borrow()
                .get_at(*distance, &token.lexeme),
            None => self.globals.borrow().get(&token.lexeme),
        }
    }

    pub fn resolve(&mut self, token: &Token, depth: usize) {
        self.locals.insert(token.idx, depth);
    }

    /// Writes one diagnostic line to the stderr writer (I/O errors are ignored).
    pub fn report_error(&self, message: &str) {
        let _ = writeln!(self.stderr.borrow_mut(), "{message}");
    }

    /// Runs the statements, stopping at the first runtime error.
    pub fn interpret(&mut self, statements: &Vec<Stmt>) -> Result<(), Error> {
        for statement in statements {
            self.execute_statement(statement)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

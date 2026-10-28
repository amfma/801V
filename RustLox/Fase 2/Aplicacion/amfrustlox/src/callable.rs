use std::fmt::Debug;
use std::fmt::Formatter;
use std::rc::Rc;
use std::{cell::RefCell, env};

use crate::{
    enviroment::Enviroment,
    error::Error,
    interpreter::{self, Interpreter, Value},
    parser::Stmt,
    token::Token,
};

pub type BoxedFunction = Box<dyn Fn(&[Value]) -> Result<Value, Error>>;

#[derive(Debug, PartialEq)]
pub enum CallableType {
    Function,
    Class,
}

pub trait Callable {
    fn ctype(&self) -> CallableType {
        CallableType::Function
    }

    fn arity(&self) -> usize;

    fn call(self: Rc<Self>, interpreter: &mut Interpreter, arguments: &Vec<Value>) -> Result<Value, Error>;

    fn name(&self) -> String;
}

impl Debug for dyn Callable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{:?} {}>", self.ctype(), self.name())
    }
}

#[derive(Debug)]
pub struct Function {
    pub name: Token,
    pub params: Vec<Token>,
    pub body: Vec<Stmt>,
    pub closure: Rc<RefCell<Enviroment>>,
}

impl Callable for Function {
    fn arity(&self) -> usize {
        self.params.len()
    }

    fn call(self: Rc<Self>, interpreter: &mut Interpreter, arguments: &Vec<Value>) -> Result<Value, Error> {
        let closure = Rc::clone(&self.closure);
        execute(self, closure, interpreter, arguments)
    }

    fn name(&self) -> String {
        String::from(&self.name.lexeme)
    }
    
    fn ctype(&self) -> CallableType {
        CallableType::Function
    }
}

impl Function {
    pub fn new(closure: Rc<RefCell<Enviroment>>, name: &Token, params: &Vec<Token>, body: &Vec<Stmt>) -> Self {
        Self {
            name: name.clone(),
            params: params.clone(),
            body: body.clone(),
            closure,
        }
    }
}

pub struct Native {
    func: BoxedFunction,
    name: String,
    arity: usize,
}

impl Native {
    pub fn new(func: BoxedFunction, name: String, arity: usize) -> Self {
        Self { func, name, arity }
    }
}

impl Callable for Native {
    fn arity(&self) -> usize {
        self.arity
    }

    fn call(self: Rc<Self>, _interpreter: &mut Interpreter, arguments: &Vec<Value>) -> Result<Value, Error> {
        (self.func)(arguments)
    }

    fn name(&self) -> String {
        String::from(&self.name)
    }
}

/* Helpers */

fn execute(
    function: Rc<Function>,
    closure: Rc<RefCell<Enviroment>>,
    interpreter: &mut Interpreter,
    arguments: &Vec<Value>,
) -> Result<Value, Error> {
    let mut enviroment = Enviroment::new(Some(closure));

    for (param, argument) in function.params.iter().zip(arguments) {
        enviroment.define(String::from(&param.lexeme), argument.clone());
    }

    match interpreter.execute_function_block(&function.body, Rc::new(RefCell::new(enviroment))) {
        Ok(()) => Ok(Value::Nil),
        Err(Error::Return(value)) => Ok(value),
        Err(err) => Err(err),
    }
}

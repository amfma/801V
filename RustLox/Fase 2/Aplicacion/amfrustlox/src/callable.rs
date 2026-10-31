use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::hash::Hash;
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

    fn as_class(self: Rc<Self>) -> Option<Rc<Class>>;
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
    
    fn as_class(self: Rc<Self>) -> Option<Rc<Class>> {
        None
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

    fn as_class(self: Rc<Self>) -> Option<Rc<Class>> {
        None
    }
}

#[derive(Debug)]
pub struct Class {
    name: String,
    methods: HashMap<String, Rc<Function>>,
    superclass: Option<Rc<Class>>
}

impl Callable for Class {
    fn ctype(&self) -> CallableType {
        CallableType::Class
    }

    fn arity(&self) -> usize {
        match self.find_method(String::from("init")) {
            Some(initializer)  => initializer.arity(),
            _ => 0
        }
    }

    fn call(self: Rc<Self>, interpreter: &mut Interpreter, arguments: &Vec<Value>) -> Result<Value, Error> {
        let instance = Instance::new(Rc::clone(&self));
        if let Some(initializer) = self.find_method(String::from("init")) {
            Rc::new(Method::bind(initializer, Rc::clone(&instance), true)).call(interpreter, arguments)?;
        }
        Ok(Value::Instance(instance))
    }

    fn name(&self) -> String {
        self.name.clone()
    }
    
    fn as_class(self: Rc<Self>) -> Option<Rc<Class>> {
        Some(self)
    }
}

impl Class {
    pub fn new(name: String, methods: HashMap<String, Rc<Function>>, superclass:Option<Rc<Class>>) -> Rc<Self> {
        Rc::new(Class {
            name: name,
            methods,
            superclass
        })
    }

    pub fn find_method(&self, name:String) -> Option<Rc<Function>> {
        if let Some(function) = self.methods.get(&name) {
            Some(Rc::clone(function))
        } else if let Some(superclass) = &self.superclass {
            superclass.find_method(name)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Instance {
    class: Rc<Class>,
    fields: HashMap<String, Value>
}

impl Instance {
    pub fn new(class: Rc<Class>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Instance { class, fields: HashMap::new() }))
    } 

    pub fn get(instance: &Rc<RefCell<Self>>, name: String) -> Option<Value> {
        if let Some(field) = instance.borrow().fields.get(&name) {
            Some(field.clone())
        } else {
             instance.borrow().class.find_method(name.clone()).map(|function| {
                Value::Callable(Rc::new(Method::bind(
                    function,
                    Rc::clone(instance),
                    name == "init")))})
        }
    }

    pub fn set(&mut self, name: String, value: Value) -> Value {
        self.fields.insert(name.clone(), value.clone());
        value
    }
}

impl Display for Instance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Instance {}.class", self.class.name())
    }
}

pub struct Method {
    closure: Rc<RefCell<Enviroment>>,
    function: Rc<Function>,
    is_init: bool
}

impl Callable for Method {
    fn arity(&self) -> usize {
        self.function.arity()
    }

    fn call(self: Rc<Self>, interpreter: &mut Interpreter, arguments: &Vec<Value>) -> Result<Value, Error> {
        let resolved = execute(Rc::clone(&self.function), Rc::clone(&self.closure), interpreter, arguments);

        match resolved {
            Ok(_) if self.is_init => Ok(self.closure.borrow().get_at(0, String::from("this")).unwrap()),
            _ => resolved
        }
    }

    fn name(&self) -> String {
        self.function.name()
    }
    
    fn as_class(self: Rc<Self>) -> Option<Rc<Class>> {
        None
    }
}

impl Method {
    pub fn bind(
        function: Rc<Function>,
        instance: Rc<RefCell<Instance>>,
        is_init: bool
    ) -> Self {
        let closure = Rc::new(RefCell::new(Enviroment::new(Some(Rc::clone(&function.closure)))));
        closure.borrow_mut().define(String::from("this"), Value::Instance(instance));

        Method {
            closure,
            function,
            is_init
        }
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

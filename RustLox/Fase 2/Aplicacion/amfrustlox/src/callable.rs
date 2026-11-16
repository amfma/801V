use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::rc::Rc;
use std::cell::RefCell;

use crate::{
    environment::Environment,
    error::Error,
    fxhash::FxHashMap,
    interpreter::{Interpreter, Value},
    parser::Stmt,
    token::Token,
};

pub type BoxedFunction = Box<dyn Fn(&[Value]) -> Result<Value, Error>>;

thread_local! {
    /// The name every bound method environment stores its receiver under.
    static THIS: Rc<str> = Rc::from("this");
}

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

    fn call(self: Rc<Self>, interpreter: &mut Interpreter, arguments: &[Value]) -> Result<Value, Error>;

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
    pub closure: Rc<RefCell<Environment>>,
    /// `params` names as shared strings, so a call binds them without allocating.
    param_names: Vec<Rc<str>>,
}

impl Callable for Function {
    fn arity(&self) -> usize {
        self.params.len()
    }

    fn call(self: Rc<Self>, interpreter: &mut Interpreter, arguments: &[Value]) -> Result<Value, Error> {
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
    pub fn new(closure: Rc<RefCell<Environment>>, name: &Token, params: &[Token], body: &[Stmt]) -> Self {
        Self {
            name: name.clone(),
            params: params.to_vec(),
            body: body.to_vec(),
            closure,
            param_names: params.iter().map(|p| Rc::from(p.lexeme.as_str())).collect(),
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

    fn call(self: Rc<Self>, _interpreter: &mut Interpreter, arguments: &[Value]) -> Result<Value, Error> {
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
    methods: FxHashMap<String, Rc<Function>>,
    superclass: Option<Rc<Class>>
}

impl Callable for Class {
    fn ctype(&self) -> CallableType {
        CallableType::Class
    }

    fn arity(&self) -> usize {
        match self.find_method("init") {
            Some(initializer)  => initializer.arity(),
            _ => 0
        }
    }

    fn call(self: Rc<Self>, interpreter: &mut Interpreter, arguments: &[Value]) -> Result<Value, Error> {
        let instance = Instance::new(Rc::clone(&self));
        if let Some(initializer) = self.find_method("init") {
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
            name,
            methods: methods.into_iter().collect(),
            superclass
        })
    }

    pub fn find_method(&self, name: impl AsRef<str>) -> Option<Rc<Function>> {
        self.find_method_str(name.as_ref())
    }

    fn find_method_str(&self, name: &str) -> Option<Rc<Function>> {
        if let Some(function) = self.methods.get(name) {
            Some(Rc::clone(function))
        } else if let Some(superclass) = &self.superclass {
            superclass.find_method_str(name)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Instance {
    class: Rc<Class>,
    fields: FxHashMap<String, Value>
}

impl Instance {
    pub fn new(class: Rc<Class>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Instance { class, fields: FxHashMap::default() }))
    } 

    pub fn get(instance: &Rc<RefCell<Self>>, name: impl AsRef<str>) -> Option<Value> {
        let name = name.as_ref();
        if let Some(field) = instance.borrow().fields.get(name) {
            Some(field.clone())
        } else {
            let method = instance.borrow().class.find_method(name);
            method.map(|function| {
                Value::Callable(Rc::new(Method::bind(
                    function,
                    Rc::clone(instance),
                    name == "init")))})
        }
    }

    pub fn set(&mut self, name: impl AsRef<str>, value: Value) -> Value {
        let name = name.as_ref();
        // Only allocate the key the first time a field is created.
        match self.fields.get_mut(name) {
            Some(slot) => *slot = value.clone(),
            None => {
                self.fields.insert(name.to_string(), value.clone());
            }
        }
        value
    }
}

impl Display for Instance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Instance {}.class", self.class.name())
    }
}

pub struct Method {
    closure: Rc<RefCell<Environment>>,
    function: Rc<Function>,
    is_init: bool
}

impl Callable for Method {
    fn arity(&self) -> usize {
        self.function.arity()
    }

    fn call(self: Rc<Self>, interpreter: &mut Interpreter, arguments: &[Value]) -> Result<Value, Error> {
        let resolved = execute(Rc::clone(&self.function), Rc::clone(&self.closure), interpreter, arguments);

        match resolved {
            Ok(_) if self.is_init => Ok(self.closure.borrow().get_at(0, "this").unwrap()),
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
        let closure = Rc::new(RefCell::new(Environment::with_capacity(Some(Rc::clone(&function.closure)), 1)));
        THIS.with(|this| closure.borrow_mut().define_shared(Rc::clone(this), Value::Instance(instance)));

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
    closure: Rc<RefCell<Environment>>,
    interpreter: &mut Interpreter,
    arguments: &[Value],
) -> Result<Value, Error> {
    let mut environment = Environment::with_capacity(Some(closure), function.param_names.len());

    for (name, argument) in function.param_names.iter().zip(arguments) {
        environment.define_shared(Rc::clone(name), argument.clone());
    }

    match interpreter.execute_function_block(&function.body, Rc::new(RefCell::new(environment))) {
        Ok(()) => Ok(Value::Nil),
        Err(Error::Return(value)) => Ok(value),
        Err(err) => Err(err),
    }
}

   #[cfg(test)]
   mod tests;
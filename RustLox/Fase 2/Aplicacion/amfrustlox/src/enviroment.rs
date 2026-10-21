use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::interpreter::Value;

#[derive(Debug, Clone)]
pub struct Enviroment {
    values: HashMap<String, Value>,
    enclosing: Option<Rc<RefCell<Enviroment>>>,
}

impl Enviroment {
    pub fn new(enclosing: Option<Rc<RefCell<Enviroment>>>) -> Self {
        match enclosing {
            Some(en) => {Enviroment { values: HashMap::new(), enclosing: Some(en) }},
            None => {Enviroment { values: HashMap::new(), enclosing: None }},
        }
    }

    pub fn define(&mut self, key: String, value: Value) {
        self.values.insert(key, value);
    }

    pub fn get(&self, key: String) -> Option<Value> {
        if let Some(val) = self.values.get(&key) {
            Some(val.clone())
        } else if let Some(enclosing) = &self.enclosing {
            enclosing.as_ref().borrow().get(key)
        } else {
            println!("Undefined variable {}.", key);
            None
        }
    }

    pub fn assign(&mut self, key: String, value: Value) {
        if self.values.contains_key(&key) {
            self.values.insert(key, value);
            return
        } else if let Some(enclosing) = &self.enclosing {
            enclosing.as_ref().borrow_mut().assign(key, value)
        } else {
            println!("Undefined variable {}.", key)
        }
    }
}

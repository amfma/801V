use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::interpreter::Value;

#[derive(Debug)]
pub struct Enviroment {
    values: HashMap<String, Value>
}

impl Enviroment {
    pub fn new() -> Self {
        Enviroment { values: HashMap::new() }
    }

    pub fn define(&mut self, key: String, value: Value) {
        self.values.insert(key, value);
    }

    pub fn get(&self, key: String) -> Option<Value> {
        if let Some(val) = self.values.get(&key) {
            Some(val.clone())
        } else {
            None
        }
    }

    pub fn assign(&mut self, key: String, value: Value) {
        if self.values.contains_key(&key) {
            self.values.insert(key, value);
        } else {
            println!("Undefined variable {}.", key)
        }
    }
}
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{error::Error, interpreter::Value};

#[derive(Debug, Clone)]
pub struct Enviroment {
    values: HashMap<String, Value>,
    enclosing: Option<Rc<RefCell<Enviroment>>>,
}

impl Enviroment {
    pub fn new(enclosing: Option<Rc<RefCell<Enviroment>>>) -> Self {
        match enclosing {
            Some(en) => Enviroment {
                values: HashMap::new(),
                enclosing: Some(en),
            },
            None => Enviroment {
                values: HashMap::new(),
                enclosing: None,
            },
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
            None
        }
    }

    pub fn get_at(&self, distance: usize, key: String) -> Option<Value> {
        if distance == 0 {
            self.values.get(&key).cloned()
        } else {
            self.enclosing
                .as_ref()
                .and_then(|parent| parent.borrow().get_at(distance - 1, key))
        }
    }

    /// Assigns to an existing binding. The caller decides how to report a
    /// failure (it knows the source position); nothing is printed here.
    pub fn assign(&mut self, key: String, value: Value) -> Result<(), Error> {
        if self.values.contains_key(&key) {
            self.values.insert(key, value);
            Ok(())
        } else if let Some(enclosing) = &self.enclosing {
            enclosing.as_ref().borrow_mut().assign(key, value)
        } else {
            Err(Error::UndefinedVariable)
        }
    }

    /// Assigns at a resolver-computed distance. A missing binding is ignored
    /// silently; the resolver guarantees it exists.
    pub fn assign_at(&mut self, key: String, distance: usize, value: Value) {
        if distance == 0 {
            if let Some(val) = self.values.get_mut(&key) {
                *val = value;
            }
        } else if let Some(parent) = &self.enclosing {
            parent
                .as_ref()
                .borrow_mut()
                .assign_at(key, distance - 1, value);
        }
    }
}

#[cfg(test)]
mod tests;

use std::{cell::RefCell, rc::Rc};

use crate::{error::Error, fxhash::FxHashMap, interpreter::Value};

/// Environments with at most this many bindings keep them in a plain `Vec` and
/// find them by linear search: for a handful of short names that beats hashing
/// and needs a single allocation. Larger ones (the globals, a big block) switch
/// to a hash map the first time they grow past the limit.
const SMALL_LIMIT: usize = 8;

/// Binding names are `Rc<str>` so hot paths (function parameters, `this`) can
/// share one allocation instead of building a new `String` per call.
#[derive(Debug, Clone)]
enum Storage {
    Small(Vec<(Rc<str>, Value)>),
    Large(FxHashMap<Rc<str>, Value>),
}

impl Storage {
    #[inline]
    fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Storage::Small(entries) => entries
                .iter()
                .find(|(name, _)| &**name == key)
                .map(|(_, value)| value),
            Storage::Large(map) => map.get(key),
        }
    }

    #[inline]
    fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        match self {
            Storage::Small(entries) => entries
                .iter_mut()
                .find(|(name, _)| &**name == key)
                .map(|(_, value)| value),
            Storage::Large(map) => map.get_mut(key),
        }
    }

    fn insert(&mut self, key: Rc<str>, value: Value) {
        match self {
            Storage::Small(entries) => {
                if let Some(slot) = entries.iter_mut().find(|(name, _)| **name == *key) {
                    slot.1 = value;
                } else if entries.len() < SMALL_LIMIT {
                    entries.push((key, value));
                } else {
                    let mut map: FxHashMap<Rc<str>, Value> = entries.drain(..).collect();
                    map.insert(key, value);
                    *self = Storage::Large(map);
                }
            }
            Storage::Large(map) => {
                map.insert(key, value);
            }
        }
    }
}

/// Lookup methods take `impl AsRef<str>` so callers can pass a `&str` (no
/// allocation, used by the interpreter's hot paths) or a `String` alike.
#[derive(Debug, Clone)]
pub struct Enviroment {
    values: Storage,
    enclosing: Option<Rc<RefCell<Enviroment>>>,
}

impl Enviroment {
    pub fn new(enclosing: Option<Rc<RefCell<Enviroment>>>) -> Self {
        Enviroment {
            values: Storage::Small(Vec::new()),
            enclosing,
        }
    }

    /// Like `new`, reserving room for `capacity` bindings up front.
    pub fn with_capacity(enclosing: Option<Rc<RefCell<Enviroment>>>, capacity: usize) -> Self {
        let values = if capacity <= SMALL_LIMIT {
            Storage::Small(Vec::with_capacity(capacity))
        } else {
            Storage::Large(FxHashMap::default())
        };
        Enviroment { values, enclosing }
    }

    pub fn define(&mut self, key: String, value: Value) {
        self.values.insert(Rc::from(key), value);
    }

    /// Defines a binding from a borrowed name (one allocation, no `String`).
    pub fn define_str(&mut self, key: &str, value: Value) {
        self.values.insert(Rc::from(key), value);
    }

    /// Defines a binding re-using an existing shared name (no allocation).
    pub fn define_shared(&mut self, key: Rc<str>, value: Value) {
        self.values.insert(key, value);
    }

    pub fn get(&self, key: impl AsRef<str>) -> Option<Value> {
        self.get_str(key.as_ref())
    }

    fn get_str(&self, key: &str) -> Option<Value> {
        if let Some(val) = self.values.get(key) {
            Some(val.clone())
        } else if let Some(enclosing) = &self.enclosing {
            enclosing.as_ref().borrow().get_str(key)
        } else {
            None
        }
    }

    pub fn get_at(&self, distance: usize, key: impl AsRef<str>) -> Option<Value> {
        self.get_at_str(distance, key.as_ref())
    }

    fn get_at_str(&self, distance: usize, key: &str) -> Option<Value> {
        if distance == 0 {
            self.values.get(key).cloned()
        } else {
            self.enclosing
                .as_ref()
                .and_then(|parent| parent.borrow().get_at_str(distance - 1, key))
        }
    }

    /// Assigns to an existing binding. The caller decides how to report a
    /// failure (it knows the source position); nothing is printed here.
    pub fn assign(&mut self, key: impl AsRef<str>, value: Value) -> Result<(), Error> {
        self.assign_str(key.as_ref(), value)
    }

    fn assign_str(&mut self, key: &str, value: Value) -> Result<(), Error> {
        if let Some(slot) = self.values.get_mut(key) {
            *slot = value;
            Ok(())
        } else if let Some(enclosing) = &self.enclosing {
            enclosing.as_ref().borrow_mut().assign_str(key, value)
        } else {
            Err(Error::UndefinedVariable)
        }
    }

    /// Assigns at a resolver-computed distance. A missing binding is ignored
    /// silently; the resolver guarantees it exists.
    pub fn assign_at(&mut self, key: impl AsRef<str>, distance: usize, value: Value) {
        self.assign_at_str(key.as_ref(), distance, value);
    }

    fn assign_at_str(&mut self, key: &str, distance: usize, value: Value) {
        if distance == 0 {
            if let Some(val) = self.values.get_mut(key) {
                *val = value;
            }
        } else if let Some(parent) = &self.enclosing {
            parent
                .as_ref()
                .borrow_mut()
                .assign_at_str(key, distance - 1, value);
        }
    }
}

#[cfg(test)]
mod tests;

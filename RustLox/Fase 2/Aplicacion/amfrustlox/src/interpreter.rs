use crate::{parser::{Expr, ast, Operator}, error};

type Output = Result<Value, error::Error>;

pub struct Interpreter {

}

pub enum Value {
    Nil,
    Boolean(bool),
    String(String),
    Number(f64)
}


impl Interpreter {
    pub fn evaluate(&self, expr: Expr) -> Output {
        
    }
}
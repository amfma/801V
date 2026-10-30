use crate::interpreter::Value;

pub enum Error {
    ParserError {error_type: ParserError},
    RuntimeError,
    Return(Value),
    UndefinedVariable
}

pub enum ParserError {
    SyntaxError,
    UnterminatedCharacter,
}

impl Error {
    pub fn syntax_error() -> Error {
        Self::ParserError { error_type: ParserError::SyntaxError }
    }
}
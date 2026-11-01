use std::fmt;

use crate::{
    interpreter::Value,
    token::{Token, TokenType},
};

/// A located, human-readable problem found before the program runs.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub line: usize,
    /// Either `" at end"` or `" at '<lexeme>'"` (note the leading space).
    pub location: String,
    pub message: String,
}

impl Diagnostic {
    pub fn at(token: &Token, message: impl Into<String>) -> Self {
        let location = if token.token_type == TokenType::EOF {
            " at end".to_string()
        } else {
            format!(" at '{}'", token.lexeme)
        };
        Diagnostic {
            line: token.line,
            location,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[line {}] Error{}: {}", self.line, self.location, self.message)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Produced by the parser.
    Syntax(Diagnostic),
    /// Produced by the resolver.
    Resolve(Diagnostic),
    RuntimeError,
    Return(Value),
    UndefinedVariable,
}

impl Error {
    pub fn syntax(token: &Token, message: impl Into<String>) -> Error {
        Error::Syntax(Diagnostic::at(token, message))
    }

    pub fn resolve(token: &Token, message: impl Into<String>) -> Error {
        Error::Resolve(Diagnostic::at(token, message))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Syntax(d) | Error::Resolve(d) => write!(f, "{d}"),
            Error::RuntimeError => write!(f, "Runtime error."),
            Error::Return(_) => write!(f, "Can't return from top-level code."),
            Error::UndefinedVariable => write!(f, "Undefined variable."),
        }
    }
}

#[cfg(test)]
mod tests;

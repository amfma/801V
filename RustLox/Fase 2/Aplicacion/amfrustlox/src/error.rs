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
    /// A located failure while the program runs (type errors, undefined names, ...).
    Runtime(Diagnostic),
    /// A failure with no source position (e.g. a native function failing).
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

    /// Runtime error located at the line of `token`.
    pub fn runtime(token: &Token, message: impl Into<String>) -> Error {
        Error::runtime_at(token.line, message)
    }

    /// Runtime error located at an explicit line.
    pub fn runtime_at(line: usize, message: impl Into<String>) -> Error {
        Error::Runtime(Diagnostic {
            line,
            location: String::new(),
            message: message.into(),
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Syntax(d) | Error::Resolve(d) => write!(f, "{d}"),
            Error::Runtime(d) => {
                write!(f, "[line {}] Runtime error{}: {}", d.line, d.location, d.message)
            }
            Error::RuntimeError => write!(f, "Runtime error."),
            Error::Return(_) => write!(f, "Can't return from top-level code."),
            Error::UndefinedVariable => write!(f, "Undefined variable."),
        }
    }
}

#[cfg(test)]
mod tests;

use std::fmt;

use crate::{
    interpreter::Value,
    token::{Token, TokenType},
};

/// A located, human-readable problem found before the program runs.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub line: usize,
    /// 1-based column of the offending token, when known. Only runtime
    /// errors raised at an operator carry one for now.
    pub column: Option<usize>,
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
            column: None,
            location,
            message: message.into(),
        }
    }
}

impl Diagnostic {
    /// The offending source line with a caret under the reported column,
    /// e.g. `  print "a" - 2;` then `            ^`. `None` when there is no
    /// column or `source` has no such line.
    pub fn snippet(&self, source: &str) -> Option<String> {
        let column = self.column?;
        let text = source.lines().nth(self.line.checked_sub(1)?)?;
        // Keep tabs as tabs so the caret lines up in a terminal.
        let padding: String = text
            .chars()
            .take(column.saturating_sub(1))
            .map(|c| if c == '\t' { '\t' } else { ' ' })
            .chain(std::iter::repeat(' '))
            .take(column.saturating_sub(1))
            .collect();
        Some(format!("  {text}\n  {padding}^"))
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
            column: None,
            location: String::new(),
            message: message.into(),
        })
    }

    /// Runtime error located at an explicit line and column. A column of `0`
    /// means unknown and is dropped.
    pub fn runtime_at_column(
        line: usize,
        column: usize,
        message: impl Into<String>,
    ) -> Error {
        Error::Runtime(Diagnostic {
            line,
            column: (column > 0).then_some(column),
            location: String::new(),
            message: message.into(),
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Syntax(d) | Error::Resolve(d) => write!(f, "{d}"),
            Error::Runtime(d) => match d.column {
                Some(column) => write!(
                    f,
                    "[line {}:{}] Runtime error{}: {}",
                    d.line, column, d.location, d.message
                ),
                None => write!(
                    f,
                    "[line {}] Runtime error{}: {}",
                    d.line, d.location, d.message
                ),
            },
            Error::RuntimeError => write!(f, "Runtime error."),
            Error::Return(_) => write!(f, "Can't return from top-level code."),
            Error::UndefinedVariable => write!(f, "Undefined variable."),
        }
    }
}

#[cfg(test)]
mod tests;

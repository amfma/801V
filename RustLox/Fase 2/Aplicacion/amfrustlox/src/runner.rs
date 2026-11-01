//! The scan -> parse -> resolve -> interpret pipeline, plus the REPL and
//! file runner built on top of it. Kept out of `main.rs` so it can be tested.
//!
//! Output streams:
//!   * program output (`print`)              -> the interpreter's `stdout` writer
//!   * parse / resolve diagnostics           -> the interpreter's `stderr` writer
//!   * REPL prompt and one-line error summary -> the writer given to `repl`
use std::{
    cell::RefCell,
    fmt,
    io::{self, BufRead, Write},
    rc::Rc,
};

use crate::{interpreter::Interpreter, parser::Parser, resolver::Resolver, token::Scanner};

/// The stage of the pipeline that rejected a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunError {
    Scan,
    Parse,
    Resolve,
    Runtime,
}

impl RunError {
    /// Process exit code, following the conventions used by the Lox book (sysexits).
    pub fn exit_code(self) -> i32 {
        match self {
            RunError::Scan | RunError::Parse | RunError::Resolve => 65,
            RunError::Runtime => 70,
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Scan => write!(f, "Scan error."),
            RunError::Parse => write!(f, "Parse error."),
            RunError::Resolve => write!(f, "Resolver error."),
            RunError::Runtime => write!(f, "Runtime error."),
        }
    }
}

/// Runs `source` on an existing interpreter (so state persists between calls).
/// Parse and resolve diagnostics are written to the interpreter's stderr writer.
pub fn run_source(interpreter: &mut Interpreter, source: &str) -> Result<(), RunError> {
    let tokens = Scanner::new(source)
        .scan_tokens()
        .map_err(|_| RunError::Scan)?;

    let statements = match Parser::new(tokens).parse() {
        Ok(statements) => statements,
        Err(error) => {
            interpreter.report_error(&error.to_string());
            return Err(RunError::Parse);
        }
    };

    // Bind first so the resolver's borrow of the interpreter ends before we report.
    let resolved = Resolver::new(interpreter).resolve(&statements);
    let statements = match resolved {
        Ok(statements) => statements,
        Err(error) => {
            interpreter.report_error(&error.to_string());
            return Err(RunError::Resolve);
        }
    };

    interpreter
        .interpret(&statements)
        .map_err(|_| RunError::Runtime)
}

/// Everything a run produced, for tests and tooling.
#[derive(Debug, Clone, PartialEq)]
pub struct Captured {
    pub stdout: String,
    pub stderr: String,
    pub result: Result<(), RunError>,
}

/// Runs `source` on a fresh interpreter, capturing both output streams.
pub fn run_captured_streams(source: &str) -> Captured {
    let out = Rc::new(RefCell::new(Vec::<u8>::new()));
    let err = Rc::new(RefCell::new(Vec::<u8>::new()));
    let out_sink: Rc<RefCell<dyn Write>> = out.clone();
    let err_sink: Rc<RefCell<dyn Write>> = err.clone();
    let mut interpreter = Interpreter::with_streams(out_sink, err_sink);
    let result = run_source(&mut interpreter, source);
    let stdout = String::from_utf8_lossy(&out.borrow()).into_owned();
    let stderr = String::from_utf8_lossy(&err.borrow()).into_owned();
    Captured {
        stdout,
        stderr,
        result,
    }
}

/// Runs `source` on a fresh interpreter and returns what it printed.
pub fn run_captured(source: &str) -> (String, Result<(), RunError>) {
    let captured = run_captured_streams(source);
    (captured.stdout, captured.result)
}

/// Runs a script file and returns the process exit code.
pub fn run_file(interpreter: &mut Interpreter, path: &str) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("Could not read '{path}': {error}");
            return 74;
        }
    };
    match run_source(interpreter, &source) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            error.exit_code()
        }
    }
}

/// Interactive loop. Returns `Ok(())` on end of input (Ctrl-D).
/// Errors in a line are summarised on `prompt` and the loop continues.
pub fn repl<R: BufRead, W: Write>(
    input: &mut R,
    prompt: &mut W,
    interpreter: &mut Interpreter,
) -> io::Result<()> {
    loop {
        write!(prompt, ">>> ")?;
        prompt.flush()?;

        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            writeln!(prompt)?;
            return Ok(());
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Err(error) = run_source(interpreter, &line) {
            writeln!(prompt, "{error}")?;
        }
    }
}

#[cfg(test)]
mod tests;

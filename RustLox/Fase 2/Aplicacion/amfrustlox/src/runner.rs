//! The scan -> parse -> resolve -> interpret pipeline, plus the REPL and
//! file runner built on top of it. Kept out of `main.rs` so it can be tested.
//!
//! Output streams:
//!   * program output (`print`)              -> the interpreter's `stdout` writer
//!   * parse / resolve / runtime diagnostics -> the interpreter's `stderr` writer
//!   * REPL prompt                           -> the writer given to `repl`
use std::{
    cell::RefCell,
    fmt,
    io::{self, BufRead, Write},
    rc::Rc,
    thread,
};

use crate::{
    error::Error, interpreter::Interpreter, parser::Parser, resolver::Resolver, token::Scanner,
};

/// Stack size of the thread the interpreter runs on. The evaluator recurses on
/// the Rust stack, so this has to cover `interpreter::MAX_CALL_DEPTH` nested
/// Lox calls (plus deeply nested expressions) with a wide margin. It is only
/// reserved address space; pages are committed as they are used.
pub const STACK_SIZE: usize = 256 * 1024 * 1024;

/// glibc gives every non-main thread its own malloc arena, which trims and
/// re-grows its heap far more eagerly than the main arena does; for an
/// allocation-heavy interpreter that costs up to ~30%. Capping the arena
/// count at 1 makes the interpreter thread share the main arena, exactly as if
/// it had run on the main thread.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn use_main_malloc_arena() {
    unsafe extern "C" {
        fn mallopt(param: i32, value: i32) -> i32;
    }
    const M_ARENA_MAX: i32 = -8;
    // SAFETY: `mallopt` is a plain libc call with integer arguments; it is
    // called before the interpreter thread (or any other thread) allocates.
    unsafe {
        mallopt(M_ARENA_MAX, 1);
    }
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn use_main_malloc_arena() {}

/// Runs `work` on a thread with `STACK_SIZE` bytes of stack and returns its
/// result. A panic in `work` is propagated to the caller.
pub fn with_big_stack<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    use_main_malloc_arena();
    let handle = thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(work)
        .expect("could not start the interpreter thread");
    match handle.join() {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

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
/// Parse, resolve and runtime diagnostics are written to the interpreter's stderr writer.
pub fn run_source(interpreter: &mut Interpreter, source: &str) -> Result<(), RunError> {
    // Scan and parse errors are collected, so report every one of them.
    let tokens = match Scanner::new(source).scan_tokens() {
        Ok(tokens) => tokens,
        Err(diagnostics) => {
            for diagnostic in &diagnostics {
                interpreter.report_error(&diagnostic.to_string());
            }
            return Err(RunError::Scan);
        }
    };

    let statements = match Parser::new(tokens).parse() {
        Ok(statements) => statements,
        Err(errors) => {
            for error in &errors {
                interpreter.report_error(&error.to_string());
            }
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

    match interpreter.interpret(&statements) {
        Ok(()) => Ok(()),
        Err(error) => {
            interpreter.report_error(&error.to_string());
            // Runtime errors raised at an operator also show the source line
            // with a caret under the operator.
            if let Error::Runtime(diagnostic) = &error
                && let Some(snippet) = diagnostic.snippet(source) {
                    interpreter.report_error(&snippet);
                }
            Err(RunError::Runtime)
        }
    }
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
    let source = source.to_owned();
    with_big_stack(move || run_captured_streams_here(&source))
}

fn run_captured_streams_here(source: &str) -> Captured {
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
        // The diagnostic was already written to the interpreter's stderr.
        Err(error) => error.exit_code(),
    }
}

/// Interactive loop. Returns `Ok(())` on end of input (Ctrl-D).
/// Errors in a line are reported on the interpreter's stderr and the loop continues.
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
        // `read_line` keeps the line terminator; strip it so an "at end" error
        // is reported on line 1 instead of on a phantom line 2.
        let source = line.trim_end_matches(['\n', '\r']);
        let _ = run_source(interpreter, source);
    }
}

#[cfg(test)]
mod tests;

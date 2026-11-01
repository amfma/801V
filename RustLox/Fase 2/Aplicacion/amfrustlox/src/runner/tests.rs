//! Tests for the pipeline, REPL and file runner (src/runner.rs).
use super::*;
use std::{io::Cursor, path::PathBuf};

fn new_interpreter() -> (Interpreter, Rc<RefCell<Vec<u8>>>) {
    let buffer = Rc::new(RefCell::new(Vec::<u8>::new()));
    let sink: Rc<RefCell<dyn Write>> = buffer.clone();
    (Interpreter::new(sink), buffer)
}

fn text(buffer: &Rc<RefCell<Vec<u8>>>) -> String {
    String::from_utf8(buffer.borrow().clone()).expect("output should be utf-8")
}

/// Returns (what the program printed, what the REPL itself wrote).
fn repl_session(input: &str) -> (String, String) {
    let (mut interpreter, output) = new_interpreter();
    let mut prompt = Vec::<u8>::new();
    let mut reader = Cursor::new(input.to_string());
    repl(&mut reader, &mut prompt, &mut interpreter).expect("repl should not hit an I/O error");
    (text(&output), String::from_utf8(prompt).unwrap())
}

fn temp_script(tag: &str, contents: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("amfrustlox_{}_{}.lox", tag, std::process::id()));
    std::fs::write(&path, contents).expect("temp file should be writable");
    path
}

/* =====================================================================
   RunError
   ===================================================================== */

#[test]
fn exit_codes() {
    assert_eq!(RunError::Scan.exit_code(), 65);
    assert_eq!(RunError::Parse.exit_code(), 65);
    assert_eq!(RunError::Resolve.exit_code(), 65);
    assert_eq!(RunError::Runtime.exit_code(), 70);
}

#[test]
fn error_display() {
    assert_eq!(RunError::Scan.to_string(), "Scan error.");
    assert_eq!(RunError::Parse.to_string(), "Parse error.");
    assert_eq!(RunError::Resolve.to_string(), "Resolver error.");
    assert_eq!(RunError::Runtime.to_string(), "Runtime error.");
}

/* =====================================================================
   run_source
   ===================================================================== */

#[test]
fn run_source_ok() {
    let (mut interpreter, output) = new_interpreter();
    assert_eq!(run_source(&mut interpreter, "print 1 + 2;"), Ok(()));
    assert_eq!(text(&output), "3\n");
}

#[test]
fn run_source_empty_program_is_ok() {
    let (mut interpreter, output) = new_interpreter();
    assert_eq!(run_source(&mut interpreter, ""), Ok(()));
    assert_eq!(run_source(&mut interpreter, "// just a comment"), Ok(()));
    assert_eq!(text(&output), "");
}

#[test]
fn run_source_reports_parse_errors() {
    let (mut interpreter, _) = new_interpreter();
    assert_eq!(run_source(&mut interpreter, "var = 1;"), Err(RunError::Parse));
    assert_eq!(run_source(&mut interpreter, "print 1"), Err(RunError::Parse));
}

#[test]
fn run_source_reports_resolver_errors() {
    let (mut interpreter, _) = new_interpreter();
    assert_eq!(
        run_source(&mut interpreter, "{ var a = 1; var a = 2; }"),
        Err(RunError::Resolve)
    );
    assert_eq!(run_source(&mut interpreter, "print this;"), Err(RunError::Resolve));
}

#[test]
fn run_source_reports_runtime_errors() {
    let (mut interpreter, _) = new_interpreter();
    assert_eq!(run_source(&mut interpreter, "print 1 / 0;"), Err(RunError::Runtime));
    assert_eq!(run_source(&mut interpreter, "print missing;"), Err(RunError::Runtime));
    assert_eq!(run_source(&mut interpreter, "var a = 1; a();"), Err(RunError::Runtime));
}

/// Documents CURRENT behaviour: the resolver accepts a top-level `return`,
/// which then surfaces as a runtime error.
#[test]
fn top_level_return_is_a_runtime_error() {
    let (mut interpreter, _) = new_interpreter();
    assert_eq!(run_source(&mut interpreter, "return 1;"), Err(RunError::Runtime));
}

#[test]
fn a_parse_error_executes_nothing() {
    let (mut interpreter, output) = new_interpreter();
    assert_eq!(
        run_source(&mut interpreter, "print \"never\"; var = 1;"),
        Err(RunError::Parse)
    );
    assert_eq!(text(&output), "");
}

#[test]
fn a_runtime_error_stops_execution_but_keeps_earlier_effects() {
    let (mut interpreter, output) = new_interpreter();
    let result = run_source(
        &mut interpreter,
        "var a = 1; print a; var b = missing; print \"unreachable\"; var c = 3;",
    );
    assert_eq!(result, Err(RunError::Runtime));
    assert_eq!(text(&output), "1\n");
    // `a` was defined before the error, `c` after it:
    assert_eq!(run_source(&mut interpreter, "print a;"), Ok(()));
    assert_eq!(run_source(&mut interpreter, "print c;"), Err(RunError::Runtime));
}

#[test]
fn state_persists_between_calls() {
    let (mut interpreter, output) = new_interpreter();
    assert_eq!(run_source(&mut interpreter, "var a = 1;"), Ok(()));
    assert_eq!(run_source(&mut interpreter, "fun inc(n) { return n + 1; }"), Ok(()));
    assert_eq!(run_source(&mut interpreter, "class Box { get() { return a; } }"), Ok(()));
    assert_eq!(run_source(&mut interpreter, "print inc(a); print Box().get();"), Ok(()));
    assert_eq!(text(&output), "2\n1\n");
}

#[test]
fn interpreter_is_still_usable_after_each_kind_of_error() {
    let (mut interpreter, output) = new_interpreter();
    let _ = run_source(&mut interpreter, "var = ;");
    let _ = run_source(&mut interpreter, "{ var a = a; }");
    let _ = run_source(&mut interpreter, "1 / 0;");
    assert_eq!(run_source(&mut interpreter, "print \"alive\";"), Ok(()));
    assert_eq!(text(&output), "alive\n");
}

/* =====================================================================
   run_captured
   ===================================================================== */

#[test]
fn run_captured_returns_output_and_result() {
    let (output, result) = run_captured("print 1; print \"two\";");
    assert_eq!(output, "1\ntwo\n");
    assert_eq!(result, Ok(()));
}

#[test]
fn run_captured_keeps_output_produced_before_an_error() {
    let (output, result) = run_captured("print 1; print 1 / 0; print 2;");
    assert_eq!(output, "1\n");
    assert_eq!(result, Err(RunError::Runtime));
}

#[test]
fn run_captured_uses_a_fresh_interpreter_each_time() {
    let (_, first) = run_captured("var a = 1;");
    let (_, second) = run_captured("print a;");
    assert_eq!(first, Ok(()));
    assert_eq!(second, Err(RunError::Runtime));
}

/* =====================================================================
   repl
   ===================================================================== */

#[test]
fn repl_terminates_on_end_of_input() {
    let (output, prompt) = repl_session("");
    assert_eq!(output, "");
    assert_eq!(prompt, ">>> \n"); // one prompt, then a newline on EOF
}

#[test]
fn repl_runs_each_line_and_prompts_before_each() {
    let (output, prompt) = repl_session("print 1;\nprint 2;\n");
    assert_eq!(output, "1\n2\n");
    assert_eq!(prompt.matches(">>> ").count(), 3); // two lines + the final EOF read
}

#[test]
fn repl_keeps_state_between_lines() {
    let (output, _) = repl_session("var a = 1;\nvar b = a + 1;\nprint b;\n");
    assert_eq!(output, "2\n");
}

#[test]
fn repl_handles_a_last_line_without_newline() {
    let (output, _) = repl_session("print 7;");
    assert_eq!(output, "7\n");
}

#[test]
fn repl_skips_blank_lines() {
    let (output, prompt) = repl_session("\n   \nprint 1;\n");
    assert_eq!(output, "1\n");
    assert!(!prompt.contains("error"));
}

#[test]
fn repl_reports_parse_errors_and_continues() {
    let (output, prompt) = repl_session("print 1\nprint 2;\n");
    assert_eq!(output, "2\n");
    assert!(prompt.contains("Parse error."));
}

#[test]
fn repl_reports_resolver_errors_and_continues() {
    let (output, prompt) = repl_session("{ var a = a; }\nprint 2;\n");
    assert_eq!(output, "2\n");
    assert!(prompt.contains("Resolver error."));
}

#[test]
fn repl_reports_runtime_errors_and_continues() {
    let (output, prompt) = repl_session("print missing;\nprint 2;\n");
    assert_eq!(output, "2\n");
    assert!(prompt.contains("Runtime error."));
}

#[test]
fn repl_state_survives_a_failed_line() {
    let (output, _) = repl_session("var a = 5;\nprint a / 0;\nprint a;\n");
    assert_eq!(output, "5\n");
}

/* =====================================================================
   run_file
   ===================================================================== */

#[test]
fn run_file_success_returns_zero_and_prints() {
    let path = temp_script("ok", "print \"from file\";");
    let (mut interpreter, output) = new_interpreter();
    assert_eq!(run_file(&mut interpreter, path.to_str().unwrap()), 0);
    assert_eq!(text(&output), "from file\n");
    let _ = std::fs::remove_file(path);
}

#[test]
fn run_file_parse_error_returns_65() {
    let path = temp_script("parse", "var = 1;");
    let (mut interpreter, _) = new_interpreter();
    assert_eq!(run_file(&mut interpreter, path.to_str().unwrap()), 65);
    let _ = std::fs::remove_file(path);
}

#[test]
fn run_file_resolver_error_returns_65() {
    let path = temp_script("resolve", "{ var a = a; }");
    let (mut interpreter, _) = new_interpreter();
    assert_eq!(run_file(&mut interpreter, path.to_str().unwrap()), 65);
    let _ = std::fs::remove_file(path);
}

#[test]
fn run_file_runtime_error_returns_70() {
    let path = temp_script("runtime", "print 1 / 0;");
    let (mut interpreter, _) = new_interpreter();
    assert_eq!(run_file(&mut interpreter, path.to_str().unwrap()), 70);
    let _ = std::fs::remove_file(path);
}

#[test]
fn run_file_missing_file_returns_74() {
    let (mut interpreter, _) = new_interpreter();
    assert_eq!(run_file(&mut interpreter, "/definitely/not/here.lox"), 74);
}

/* =====================================================================
   Diagnostics routing (stderr writer)
   ===================================================================== */

fn new_streams() -> (Interpreter, Rc<RefCell<Vec<u8>>>, Rc<RefCell<Vec<u8>>>) {
    let out = Rc::new(RefCell::new(Vec::<u8>::new()));
    let err = Rc::new(RefCell::new(Vec::<u8>::new()));
    let out_sink: Rc<RefCell<dyn Write>> = out.clone();
    let err_sink: Rc<RefCell<dyn Write>> = err.clone();
    (Interpreter::with_streams(out_sink, err_sink), out, err)
}

#[test]
fn parse_errors_are_written_to_stderr_not_stdout() {
    let c = run_captured_streams("print 1");
    assert_eq!(c.result, Err(RunError::Parse));
    assert_eq!(c.stdout, "");
    assert_eq!(c.stderr, "[line 1] Error at end: Expect ';' after value.\n");
}

#[test]
fn parse_error_line_numbers_are_reported() {
    let c = run_captured_streams("var a = 1;\nvar b = ;\n");
    assert_eq!(c.result, Err(RunError::Parse));
    assert_eq!(c.stderr, "[line 2] Error at ';': Expect expression.\n");
}

#[test]
fn resolver_errors_are_written_to_stderr_not_stdout() {
    let c = run_captured_streams("{ var a = a; }");
    assert_eq!(c.result, Err(RunError::Resolve));
    assert_eq!(c.stdout, "");
    assert_eq!(
        c.stderr,
        "[line 1] Error at 'a': Can't read local variable in its own initializer.\n"
    );
}

#[test]
fn successful_runs_write_nothing_to_stderr() {
    let c = run_captured_streams("print 1; var a = 2;");
    assert_eq!(c.result, Ok(()));
    assert_eq!(c.stdout, "1\n");
    assert_eq!(c.stderr, "");
}

/// Documents CURRENT behaviour: runtime errors carry no message yet, so only
/// the `RunError::Runtime` result signals them.
#[test]
fn runtime_errors_do_not_write_to_stderr_yet() {
    let c = run_captured_streams("print 1; print 1 / 0;");
    assert_eq!(c.result, Err(RunError::Runtime));
    assert_eq!(c.stdout, "1\n");
    assert_eq!(c.stderr, "");
}

#[test]
fn diagnostics_accumulate_across_runs_on_one_interpreter() {
    let (mut interpreter, out, err) = new_streams();
    assert_eq!(run_source(&mut interpreter, "print 1"), Err(RunError::Parse));
    assert_eq!(run_source(&mut interpreter, "print this;"), Err(RunError::Resolve));
    assert_eq!(run_source(&mut interpreter, "print 2;"), Ok(()));
    assert_eq!(
        text(&err),
        "[line 1] Error at end: Expect ';' after value.\n\
         [line 1] Error at 'this': Can't use 'this' outside of a class.\n"
    );
    assert_eq!(text(&out), "2\n");
}

#[test]
fn a_rejected_program_leaves_the_interpreter_usable_and_silent_on_stdout() {
    let (mut interpreter, out, _) = new_streams();
    let _ = run_source(&mut interpreter, "var = 1;");
    assert_eq!(text(&out), "");
    assert_eq!(run_source(&mut interpreter, "print \"ok\";"), Ok(()));
    assert_eq!(text(&out), "ok\n");
}

#[test]
fn run_captured_still_returns_the_tuple_form() {
    let (stdout, result) = run_captured("print 1");
    assert_eq!(stdout, "");
    assert_eq!(result, Err(RunError::Parse));
}

#[test]
fn repl_sends_diagnostics_to_stderr_and_a_summary_to_the_prompt() {
    let (mut interpreter, out, err) = new_streams();
    let mut prompt = Vec::<u8>::new();
    let mut reader = Cursor::new("print 1\nprint 2;\n".to_string());
    repl(&mut reader, &mut prompt, &mut interpreter).unwrap();

    assert_eq!(text(&out), "2\n");
    assert_eq!(text(&err), "[line 1] Error at end: Expect ';' after value.\n");
    let prompt = String::from_utf8(prompt).unwrap();
    assert!(prompt.contains("Parse error."));
    assert!(!prompt.contains("Expect ';'"), "details belong on stderr");
}

#[test]
fn repl_line_numbers_restart_for_each_line() {
    let (mut interpreter, _, err) = new_streams();
    let mut prompt = Vec::<u8>::new();
    let mut reader = Cursor::new("print ;\nprint ;\n".to_string());
    repl(&mut reader, &mut prompt, &mut interpreter).unwrap();
    assert_eq!(
        text(&err),
        "[line 1] Error at ';': Expect expression.\n[line 1] Error at ';': Expect expression.\n"
    );
}

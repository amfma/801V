//! Black-box tests that run the compiled `amfrustlox` binary.
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_amfrustlox"))
}

fn lox(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("lox")
        .join(name)
}

#[test]
fn runs_a_script_file() {
    let out = bin().arg(lox("fibonacci.lox")).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 10);
    assert_eq!(stdout.lines().next(), Some("0"));
    assert_eq!(stdout.lines().last(), Some("34"));
}

#[test]
fn runtime_error_exits_with_70() {
    let out = bin().arg(lox("error_runtime_division.lox")).output().unwrap();
    assert_eq!(out.status.code(), Some(70));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "before\n");
}

#[test]
fn parse_error_exits_with_65_and_reports_on_stderr() {
    let out = bin().arg(lox("error_parse.lox")).output().unwrap();
    assert_eq!(out.status.code(), Some(65));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "", "stdout must stay clean");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("[line 2] Error at '=': Expect variable name."),
        "stderr was: {stderr:?}"
    );
}

#[test]
fn resolver_error_exits_with_65_and_reports_on_stderr() {
    let out = bin().arg(lox("error_resolve_duplicate.lox")).output().unwrap();
    assert_eq!(out.status.code(), Some(65));
    assert_eq!(String::from_utf8(out.stdout).unwrap(), "");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains(
            "[line 3] Error at 'a': Already a variable with this name in this scope."
        ),
        "stderr was: {stderr:?}"
    );
}

#[test]
fn missing_script_exits_with_74() {
    let out = bin().arg("/definitely/not/here.lox").output().unwrap();
    assert_eq!(out.status.code(), Some(74));
}

#[test]
fn too_many_arguments_exits_with_64() {
    let out = bin().args(["a.lox", "b.lox"]).output().unwrap();
    assert_eq!(out.status.code(), Some(64));
}

#[test]
fn repl_executes_lines_and_exits_cleanly_on_eof() {
    let mut child = bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"var a = 20;\nprint a + 1;\n")
        .unwrap(); // stdin is dropped here => EOF
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "REPL must exit 0 on EOF");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("21"), "stdout was: {stdout:?}");
}

#[test]
fn repl_survives_errors() {
    let mut child = bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"print 1 / 0;\nprint 5;\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("Runtime error."));
    assert!(stdout.contains('5'));
}

#[test]
fn repl_reports_syntax_errors_on_stderr() {
    let mut child = bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"print 1\nprint 2;\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("Expect ';' after value."), "stderr was: {stderr:?}");
    assert!(!stdout.contains("Expect ';'"), "diagnostics must not reach stdout");
    assert!(stdout.contains("Parse error."));
    assert!(stdout.contains('2'));
}

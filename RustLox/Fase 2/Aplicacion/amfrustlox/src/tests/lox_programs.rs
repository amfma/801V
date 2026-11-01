//! Runs every `tests/lox/*.lox` program and checks its printed output.
//!
//! Conventions inside a .lox file:
//!   // expect: <text>        one expected line of output (in order of appearance)
//!   // expect error: <kind>  the run must fail with scan|parse|resolve|runtime
//!   // expect diagnostic: <text>  one expected line written to stderr (parse/resolve messages)
use amfrustlox::runner::{run_captured_streams, RunError};
use std::{fs, path::PathBuf};

struct Expectation {
    output: Vec<String>,
    diagnostics: Vec<String>,
    error: Option<RunError>,
}

fn parse_expectations(source: &str) -> Expectation {
    const ERROR_MARK: &str = "// expect error: ";
    const DIAGNOSTIC_MARK: &str = "// expect diagnostic: ";
    const OUTPUT_MARK: &str = "// expect: ";

    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    let mut error = None;
    for line in source.lines() {
        if let Some(pos) = line.find(ERROR_MARK) {
            error = Some(match line[pos + ERROR_MARK.len()..].trim() {
                "scan" => RunError::Scan,
                "parse" => RunError::Parse,
                "resolve" => RunError::Resolve,
                "runtime" => RunError::Runtime,
                other => panic!("unknown error kind `{other}` in: {line}"),
            });
        } else if let Some(pos) = line.find(DIAGNOSTIC_MARK) {
            diagnostics.push(line[pos + DIAGNOSTIC_MARK.len()..].trim_end().to_string());
        } else if let Some(pos) = line.find(OUTPUT_MARK) {
            output.push(line[pos + OUTPUT_MARK.len()..].trim_end().to_string());
        }
    }
    Expectation {
        output,
        diagnostics,
        error,
    }
}

fn lox_files() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("lox");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "lox"))
        .collect();
    files.sort();
    files
}

#[test]
fn all_lox_programs_behave_as_expected() {
    let files = lox_files();
    assert!(!files.is_empty(), "no .lox programs found");

    let mut failures = Vec::new();
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let source = fs::read_to_string(path).unwrap();
        let expected = parse_expectations(&source);
        let captured = run_captured_streams(&source);

        let actual: Vec<String> = captured.stdout.lines().map(String::from).collect();
        let actual_diagnostics: Vec<String> =
            captured.stderr.lines().map(String::from).collect();
        let result = captured.result;
        if actual != expected.output {
            failures.push(format!(
                "{name}: wrong output\n  expected: {:?}\n  actual:   {:?}",
                expected.output, actual
            ));
        }
        if actual_diagnostics != expected.diagnostics {
            failures.push(format!(
                "{name}: wrong diagnostics\n  expected: {:?}\n  actual:   {:?}",
                expected.diagnostics, actual_diagnostics
            ));
        }
        let actual_error = result.err();
        if actual_error != expected.error {
            failures.push(format!(
                "{name}: wrong outcome\n  expected: {:?}\n  actual:   {:?}",
                expected.error, actual_error
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}\n", failures.join("\n\n"));
}

#[test]
fn print_output_is_ordered_across_statements() {
    let (output, result) = amfrustlox::runner::run_captured("print 1; print 2; print 3;");
    assert_eq!(result, Ok(()));
    assert_eq!(output, "1\n2\n3\n");
}

#[test]
fn output_is_captured_per_run() {
    let (first, _) = amfrustlox::runner::run_captured("print \"a\";");
    let (second, _) = amfrustlox::runner::run_captured("print \"b\";");
    assert_eq!(first, "a\n");
    assert_eq!(second, "b\n");
}

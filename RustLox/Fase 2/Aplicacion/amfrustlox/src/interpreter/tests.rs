//! Tests for `Interpreter` and `Value` (src/interpreter.rs).
//!
//! `print` writes through `println!`, so it cannot be captured. Instead the
//! tests run a program and then inspect the *global variables* it left behind.
//! Runtime errors are checked through `execute_statement` directly, because
//! `Interpreter::interpret` deliberately swallows them.
use super::*;
use crate::{
    parser::Parser,
    resolver::Resolver,
    token::{Literal, Scanner, TokenType},
};
use std::{cell::RefCell, io::Write, rc::Rc};

/* ---------- helpers ---------- */

fn new_interpreter() -> (Interpreter, Rc<RefCell<Vec<u8>>>) {
    let buffer = Rc::new(RefCell::new(Vec::<u8>::new()));
    let sink: Rc<RefCell<dyn Write>> = buffer.clone();
    (Interpreter::new(sink), buffer)
}

fn parse_and_resolve(interpreter: &mut Interpreter, src: &str) -> Vec<Stmt> {
    let tokens = Scanner::new(src).scan_tokens().expect("scan should succeed");
    let statements = match Parser::new(tokens).parse() {
        Ok(s) => s,
        Err(_) => panic!("parse error in: {src}"),
    };
    match Resolver::new(interpreter).resolve(&statements) {
        Ok(s) => s,
        Err(_) => panic!("resolver error in: {src}"),
    }
}

/// Runs `src` on an existing interpreter, stopping at the first runtime error.
fn try_run_in(interpreter: &mut Interpreter, src: &str) -> Result<(), Error> {
    let statements = parse_and_resolve(interpreter, src);
    for statement in &statements {
        interpreter.execute_statement(statement)?;
    }
    Ok(())
}

/// Runs `src` on a fresh interpreter; panics on runtime errors.
fn run(src: &str) -> Interpreter {
    let (mut interpreter, _) = new_interpreter();
    if try_run_in(&mut interpreter, src).is_err() {
        panic!("unexpected runtime error while running: {src}");
    }
    interpreter
}

/// True when running `src` ends in a runtime error.
fn fails(src: &str) -> bool {
    let (mut interpreter, _) = new_interpreter();
    try_run_in(&mut interpreter, src).is_err()
}

fn global(interpreter: &Interpreter, name: &str) -> Option<Value> {
    interpreter.globals.borrow().get(name.to_string())
}

/// Evaluates an expression by storing it in a global and reading it back.
fn eval(expr: &str) -> Value {
    let interpreter = run(&format!("var r = {};", expr));
    global(&interpreter, "r").expect("`r` should be defined")
}

fn eval_fails(expr: &str) -> bool {
    fails(&format!("var r = {};", expr))
}

fn num(n: f64) -> Value {
    Value::Number(n)
}

fn text(s: &str) -> Value {
    Value::String(s.to_string())
}

fn boolean(b: bool) -> Value {
    Value::Boolean(b)
}

/* =====================================================================
   Value
   ===================================================================== */

#[test]
fn truthiness_only_nil_and_false_are_falsy() {
    assert!(!Value::Nil.is_truthy());
    assert!(!Value::Boolean(false).is_truthy());
    assert!(Value::Boolean(true).is_truthy());
    assert!(num(0.0).is_truthy());
    assert!(text("").is_truthy());
    assert!(text("x").is_truthy());
}

#[test]
fn value_display() {
    assert_eq!(Value::Nil.to_string(), "nil");
    assert_eq!(boolean(true).to_string(), "true");
    assert_eq!(boolean(false).to_string(), "false");
    assert_eq!(text("hi").to_string(), "hi");
    assert_eq!(num(3.0).to_string(), "3");
    assert_eq!(num(2.5).to_string(), "2.5");
}

#[test]
fn value_equality_same_types() {
    assert_eq!(Value::Nil, Value::Nil);
    assert_eq!(num(1.0), num(1.0));
    assert_ne!(num(1.0), num(2.0));
    assert_eq!(text("a"), text("a"));
    assert_ne!(text("a"), text("b"));
    assert_eq!(boolean(true), boolean(true));
    assert_ne!(boolean(true), boolean(false));
}

#[test]
fn value_equality_different_types_is_false() {
    assert_ne!(num(1.0), text("1"));
    assert_ne!(Value::Nil, boolean(false));
    assert_ne!(Value::Nil, num(0.0));
    assert_ne!(boolean(false), num(0.0));
}

#[test]
fn callable_display_uses_type_and_name() {
    let interpreter = run("");
    let clock = global(&interpreter, "clock").expect("clock is predefined");
    assert_eq!(clock.to_string(), "<Function clock>");
}

/* =====================================================================
   Construction & globals
   ===================================================================== */

#[test]
fn new_interpreter_predefines_clock() {
    let interpreter = run("");
    match global(&interpreter, "clock") {
        Some(Value::Callable(c)) => {
            assert_eq!(c.arity(), 0);
            assert_eq!(c.name(), "clock");
        }
        other => panic!("clock should be a callable, got {other:?}"),
    }
}

#[test]
fn clock_writes_to_the_injected_stdout() {
    let (mut interpreter, buffer) = new_interpreter();
    assert!(try_run_in(&mut interpreter, "clock();").is_ok());
    assert!(!buffer.borrow().is_empty());
}

#[test]
fn clock_rejects_arguments() {
    assert!(fails("clock(1);"));
}

#[test]
#[ignore = "known gap: clock prints the time and returns nil instead of returning a number"]
fn clock_returns_a_number() {
    assert!(matches!(eval("clock()"), Value::Number(_)));
}

#[test]
fn resolve_records_the_depth_for_a_token() {
    let (mut interpreter, _) = new_interpreter();
    let token = Token::new(TokenType::Identifier, "x", 1, Literal::None, 424_242);
    interpreter.resolve(&token, 3);
    assert_eq!(interpreter.locals.get(&token.idx), Some(&3));
}

#[test]
fn state_persists_between_runs_like_a_repl() {
    let (mut interpreter, _) = new_interpreter();
    assert!(try_run_in(&mut interpreter, "var a = 1;").is_ok());
    assert!(try_run_in(&mut interpreter, "var b = a + 1;").is_ok());
    assert!(try_run_in(&mut interpreter, "fun inc(x) { return x + 1; }").is_ok());
    assert!(try_run_in(&mut interpreter, "var c = inc(b);").is_ok());
    assert_eq!(global(&interpreter, "c"), Some(num(3.0)));
}

#[test]
fn interpret_keeps_going_after_a_runtime_error() {
    let (mut interpreter, _) = new_interpreter();
    let statements =
        parse_and_resolve(&mut interpreter, "var a = 1; var b = missing; var c = 3;");
    interpreter.interpret(&statements);
    assert_eq!(global(&interpreter, "a"), Some(num(1.0)));
    assert_eq!(global(&interpreter, "b"), None);
    assert_eq!(global(&interpreter, "c"), Some(num(3.0)));
}

#[test]
fn environment_is_restored_after_a_runtime_error_in_a_block() {
    let (mut interpreter, _) = new_interpreter();
    let result = try_run_in(&mut interpreter, "{ var a = 1; var b = 1 / 0; }");
    assert!(result.is_err());
    assert!(Rc::ptr_eq(&interpreter.enviroment, &interpreter.globals));
}

#[test]
fn environment_is_restored_after_a_runtime_error_in_a_function() {
    let (mut interpreter, _) = new_interpreter();
    assert!(try_run_in(&mut interpreter, "fun boom() { return 1 / 0; }").is_ok());
    assert!(try_run_in(&mut interpreter, "boom();").is_err());
    assert!(Rc::ptr_eq(&interpreter.enviroment, &interpreter.globals));
}

/* =====================================================================
   Expressions: literals & arithmetic
   ===================================================================== */

#[test]
fn literals() {
    assert_eq!(eval("42"), num(42.0));
    assert_eq!(eval("2.5"), num(2.5));
    assert_eq!(eval("\"hi\""), text("hi"));
    assert_eq!(eval("true"), boolean(true));
    assert_eq!(eval("false"), boolean(false));
    assert_eq!(eval("nil"), Value::Nil);
}

#[test]
fn arithmetic() {
    assert_eq!(eval("1 + 2"), num(3.0));
    assert_eq!(eval("5 - 3"), num(2.0));
    assert_eq!(eval("2 * 3"), num(6.0));
    assert_eq!(eval("8 / 2"), num(4.0));
    assert_eq!(eval("7 / 2"), num(3.5));
}

#[test]
fn arithmetic_precedence_and_grouping() {
    assert_eq!(eval("1 + 2 * 3"), num(7.0));
    assert_eq!(eval("(1 + 2) * 3"), num(9.0));
    assert_eq!(eval("1 + 6 - 4 / 2"), num(5.0));
    assert_eq!(eval("10 - 4 - 3"), num(3.0));
    assert_eq!(eval("8 / 2 / 2"), num(2.0));
}

#[test]
fn unary_minus() {
    assert_eq!(eval("-5"), num(-5.0));
    assert_eq!(eval("--5"), num(5.0));
    assert_eq!(eval("-(2 + 3)"), num(-5.0));
}

#[test]
fn string_concatenation() {
    assert_eq!(eval(r#""foo" + "bar""#), text("foobar"));
    assert_eq!(eval(r#""" + "x""#), text("x"));
}

#[test]
fn division_by_zero_is_a_runtime_error() {
    assert!(eval_fails("1 / 0"));
    assert!(eval_fails("1 / (2 - 2)"));
}

#[test]
fn invalid_operand_types_are_runtime_errors() {
    assert!(eval_fails(r#"1 + "a""#));
    assert!(eval_fails(r#""a" + 1"#));
    assert!(eval_fails("true + 1"));
    assert!(eval_fails("nil + nil"));
    assert!(eval_fails(r#""a" - "b""#));
    assert!(eval_fails(r#"-"a""#));
    assert!(eval_fails("-true"));
    assert!(eval_fails(r#"1 < "a""#));
    assert!(eval_fails("nil > 1"));
}

/* =====================================================================
   Expressions: comparison, equality, logic
   ===================================================================== */

#[test]
fn number_comparisons() {
    assert_eq!(eval("1 < 2"), boolean(true));
    assert_eq!(eval("2 < 2"), boolean(false));
    assert_eq!(eval("2 <= 2"), boolean(true));
    assert_eq!(eval("3 > 4"), boolean(false));
    assert_eq!(eval("4 > 3"), boolean(true));
    assert_eq!(eval("4 >= 4"), boolean(true));
    assert_eq!(eval("3 >= 4"), boolean(false));
}

#[test]
fn string_comparisons() {
    assert_eq!(eval(r#""a" < "b""#), boolean(true));
    assert_eq!(eval(r#""b" < "a""#), boolean(false));
    assert_eq!(eval(r#""a" <= "a""#), boolean(true));
    assert_eq!(eval(r#""b" > "a""#), boolean(true));
    assert_eq!(eval(r#""a" >= "b""#), boolean(false));
}

#[test]
fn equality() {
    assert_eq!(eval("1 == 1"), boolean(true));
    assert_eq!(eval("1 == 2"), boolean(false));
    assert_eq!(eval("1 != 2"), boolean(true));
    assert_eq!(eval("1 != 1"), boolean(false));
    assert_eq!(eval(r#""a" == "a""#), boolean(true));
    assert_eq!(eval("nil == nil"), boolean(true));
    assert_eq!(eval("true == true"), boolean(true));
}

#[test]
fn equality_never_coerces_between_types() {
    assert_eq!(eval(r#"1 == "1""#), boolean(false));
    assert_eq!(eval("nil == false"), boolean(false));
    assert_eq!(eval("0 == false"), boolean(false));
    assert_eq!(eval(r#""" == nil"#), boolean(false));
    assert_eq!(eval("nil != false"), boolean(true));
}

#[test]
fn logical_not() {
    assert_eq!(eval("!true"), boolean(false));
    assert_eq!(eval("!false"), boolean(true));
    assert_eq!(eval("!nil"), boolean(true));
    assert_eq!(eval("!0"), boolean(false));
    assert_eq!(eval(r#"!"""#), boolean(false));
    assert_eq!(eval("!!1"), boolean(true));
}

#[test]
fn logical_or_returns_an_operand() {
    assert_eq!(eval(r#"nil or "x""#), text("x"));
    assert_eq!(eval(r#""a" or "b""#), text("a"));
    assert_eq!(eval("false or false"), boolean(false));
    assert_eq!(eval("false or nil"), Value::Nil);
}

#[test]
fn logical_and_returns_an_operand() {
    assert_eq!(eval("false and 1"), boolean(false));
    assert_eq!(eval("nil and 1"), Value::Nil);
    assert_eq!(eval("1 and 2"), num(2.0));
    assert_eq!(eval("true and false"), boolean(false));
}

#[test]
fn logical_operator_precedence() {
    assert_eq!(eval("true or false and false"), boolean(true));
    assert_eq!(eval("false and false or true"), boolean(true));
}

#[test]
#[ignore = "known bug: `or` evaluates its right operand even when the left is truthy"]
fn or_short_circuits() {
    let interpreter = run(r#"
        var calls = 0;
        fun bump() { calls = calls + 1; return true; }
        var r = true or bump();
    "#);
    assert_eq!(global(&interpreter, "calls"), Some(num(0.0)));
}

#[test]
#[ignore = "known bug: `and` evaluates its right operand even when the left is falsy"]
fn and_short_circuits() {
    let interpreter = run(r#"
        var calls = 0;
        fun bump() { calls = calls + 1; return true; }
        var r = false and bump();
    "#);
    assert_eq!(global(&interpreter, "calls"), Some(num(0.0)));
}

/* =====================================================================
   Variables & assignment
   ===================================================================== */

#[test]
fn var_with_initializer() {
    let interpreter = run("var a = 1;");
    assert_eq!(global(&interpreter, "a"), Some(num(1.0)));
}

#[test]
fn var_without_initializer_is_nil() {
    let interpreter = run("var a;");
    assert_eq!(global(&interpreter, "a"), Some(Value::Nil));
}

#[test]
fn var_initializer_may_use_other_variables() {
    let interpreter = run("var a = 2; var b = a * 3;");
    assert_eq!(global(&interpreter, "b"), Some(num(6.0)));
}

#[test]
fn globals_can_be_redefined() {
    let interpreter = run("var a = 1; var a = 2;");
    assert_eq!(global(&interpreter, "a"), Some(num(2.0)));
}

#[test]
fn assignment_updates_the_variable() {
    let interpreter = run("var a = 1; a = 2;");
    assert_eq!(global(&interpreter, "a"), Some(num(2.0)));
}

#[test]
fn assignment_is_an_expression() {
    let interpreter = run("var a; var b; b = a = 3;");
    assert_eq!(global(&interpreter, "a"), Some(num(3.0)));
    assert_eq!(global(&interpreter, "b"), Some(num(3.0)));
}

#[test]
fn reading_an_undefined_variable_is_a_runtime_error() {
    assert!(fails("var a = missing;"));
    assert!(fails("print missing;"));
}

#[test]
fn assigning_an_undefined_variable_is_a_runtime_error() {
    assert!(fails("missing = 1;"));
}

/* =====================================================================
   Blocks & scoping
   ===================================================================== */

#[test]
fn block_variable_shadows_the_global() {
    let interpreter = run("var a = 1; var inner; { var a = 2; inner = a; }");
    assert_eq!(global(&interpreter, "a"), Some(num(1.0)));
    assert_eq!(global(&interpreter, "inner"), Some(num(2.0)));
}

#[test]
fn block_can_assign_to_an_outer_global() {
    let interpreter = run("var a = 1; { a = 2; }");
    assert_eq!(global(&interpreter, "a"), Some(num(2.0)));
}

#[test]
fn block_variables_do_not_leak() {
    assert!(fails("{ var inner = 1; } var r = inner;"));
}

#[test]
fn deeply_nested_blocks_read_outer_locals() {
    let interpreter = run("var r; { var a = 1; { { r = a; } } }");
    assert_eq!(global(&interpreter, "r"), Some(num(1.0)));
}

#[test]
fn inner_block_can_assign_outer_local() {
    let interpreter = run("var r; { var a = 1; { a = 2; } r = a; }");
    assert_eq!(global(&interpreter, "r"), Some(num(2.0)));
}

#[test]
fn sibling_blocks_have_independent_scopes() {
    let interpreter = run("var x; var y; { var a = 1; x = a; } { var a = 2; y = a; }");
    assert_eq!(global(&interpreter, "x"), Some(num(1.0)));
    assert_eq!(global(&interpreter, "y"), Some(num(2.0)));
}

/* =====================================================================
   Control flow
   ===================================================================== */

#[test]
fn if_takes_the_then_branch() {
    let interpreter = run("var r; if (true) r = 1; else r = 2;");
    assert_eq!(global(&interpreter, "r"), Some(num(1.0)));
}

#[test]
fn if_takes_the_else_branch() {
    let interpreter = run("var r; if (false) r = 1; else r = 2;");
    assert_eq!(global(&interpreter, "r"), Some(num(2.0)));
}

#[test]
fn if_without_else_skips_when_falsy() {
    let interpreter = run("var r = 0; if (false) r = 1;");
    assert_eq!(global(&interpreter, "r"), Some(num(0.0)));
}

#[test]
fn if_uses_lox_truthiness() {
    let interpreter = run("var a = 0; var b = 0; var c = 0; if (0) a = 1; if (\"\") b = 1; if (nil) c = 1;");
    assert_eq!(global(&interpreter, "a"), Some(num(1.0)));
    assert_eq!(global(&interpreter, "b"), Some(num(1.0)));
    assert_eq!(global(&interpreter, "c"), Some(num(0.0)));
}

#[test]
fn nested_if_else_chain() {
    let interpreter = run(r#"
        var r;
        var n = 5;
        if (n < 3) r = "small";
        else if (n < 10) r = "medium";
        else r = "large";
    "#);
    assert_eq!(global(&interpreter, "r"), Some(text("medium")));
}

#[test]
fn while_loop_runs_until_the_condition_is_falsy() {
    let interpreter = run("var i = 0; while (i < 3) i = i + 1;");
    assert_eq!(global(&interpreter, "i"), Some(num(3.0)));
}

#[test]
fn while_with_falsy_condition_never_runs() {
    let interpreter = run("var n = 0; while (false) n = 1;");
    assert_eq!(global(&interpreter, "n"), Some(num(0.0)));
}

#[test]
fn while_with_block_body() {
    let interpreter = run(r#"
        var i = 0; var sum = 0;
        while (i < 5) { sum = sum + i; i = i + 1; }
    "#);
    assert_eq!(global(&interpreter, "sum"), Some(num(10.0)));
}

#[test]
fn for_loop_sums_a_range() {
    let interpreter = run("var s = 0; for (var i = 0; i < 4; i = i + 1) s = s + i;");
    assert_eq!(global(&interpreter, "s"), Some(num(6.0)));
}

#[test]
fn for_loop_variable_is_scoped_to_the_loop() {
    assert!(fails("for (var i = 0; i < 2; i = i + 1) {} var r = i;"));
}

#[test]
fn for_loop_with_block_body() {
    let interpreter = run(r#"
        var product = 1;
        for (var i = 1; i <= 5; i = i + 1) { product = product * i; }
    "#);
    assert_eq!(global(&interpreter, "product"), Some(num(120.0)));
}

#[test]
fn errors_inside_loops_propagate() {
    assert!(fails("while (true) { var x = 1 / 0; }"));
    assert!(fails("var i = 0; while (i < 3) { i = missing; }"));
}

/* =====================================================================
   Functions
   ===================================================================== */

#[test]
fn function_call_with_return_value() {
    let interpreter = run("fun add(a, b) { return a + b; } var r = add(1, 2);");
    assert_eq!(global(&interpreter, "r"), Some(num(3.0)));
}

#[test]
fn function_without_return_yields_nil() {
    let interpreter = run("fun f() {} var r = f();");
    assert_eq!(global(&interpreter, "r"), Some(Value::Nil));
}

#[test]
fn bare_return_yields_nil() {
    let interpreter = run("fun f() { return; } var r = f();");
    assert_eq!(global(&interpreter, "r"), Some(Value::Nil));
}

#[test]
fn return_exits_the_function_early() {
    let interpreter = run(r#"
        var side = 0;
        fun f() { return 1; side = 99; }
        var r = f();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(1.0)));
    assert_eq!(global(&interpreter, "side"), Some(num(0.0)));
}

#[test]
fn return_from_inside_a_loop() {
    let interpreter = run(r#"
        fun first() {
            var i = 0;
            while (true) { i = i + 1; if (i == 3) return i; }
        }
        var r = first();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(3.0)));
}

#[test]
fn nested_calls() {
    let interpreter = run("fun double(x) { return x * 2; } var r = double(double(3));");
    assert_eq!(global(&interpreter, "r"), Some(num(12.0)));
}

#[test]
fn recursion_factorial() {
    let interpreter = run(r#"
        fun fact(n) { if (n <= 1) return 1; return n * fact(n - 1); }
        var r = fact(5);
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(120.0)));
}

#[test]
fn recursion_fibonacci() {
    let interpreter = run(r#"
        fun fib(n) { if (n < 2) return n; return fib(n - 1) + fib(n - 2); }
        var r = fib(10);
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(55.0)));
}

#[test]
fn functions_are_first_class_values() {
    let interpreter = run(r#"
        fun inc(n) { return n + 1; }
        fun apply(f, x) { return f(x); }
        var r = apply(inc, 1);
        var g = inc;
        var same = g == inc;
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(2.0)));
    assert_eq!(global(&interpreter, "same"), Some(boolean(true)));
}

#[test]
fn distinct_functions_are_not_equal() {
    let interpreter = run("fun f() {} fun g() {} var r = f == g;");
    assert_eq!(global(&interpreter, "r"), Some(boolean(false)));
}

#[test]
fn function_can_modify_a_global() {
    let interpreter = run("var g = 0; fun set(v) { g = v; } set(5);");
    assert_eq!(global(&interpreter, "g"), Some(num(5.0)));
}

#[test]
fn parameters_shadow_globals() {
    let interpreter = run("var a = 1; fun f(a) { return a; } var r = f(7);");
    assert_eq!(global(&interpreter, "r"), Some(num(7.0)));
    assert_eq!(global(&interpreter, "a"), Some(num(1.0)));
}

#[test]
fn wrong_argument_count_is_a_runtime_error() {
    assert!(fails("fun f(a) {} f();"));
    assert!(fails("fun f(a) {} f(1, 2);"));
    assert!(fails("fun f() {} f(1);"));
}

#[test]
fn calling_a_non_callable_is_a_runtime_error() {
    assert!(fails("var a = 1; a();"));
    assert!(fails("\"str\"();"));
    assert!(fails("nil();"));
}

#[test]
fn argument_errors_propagate() {
    assert!(fails("fun f(a) {} f(1 / 0);"));
    assert!(fails("fun f(a) {} f(missing);"));
}

/* =====================================================================
   Closures & static scoping
   ===================================================================== */

#[test]
fn closure_keeps_its_captured_variable_alive() {
    let interpreter = run(r#"
        fun makeCounter() {
            var i = 0;
            fun count() { i = i + 1; return i; }
            return count;
        }
        var c = makeCounter();
        c(); c();
        var r = c();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(3.0)));
}

#[test]
fn closures_from_separate_calls_have_independent_state() {
    let interpreter = run(r#"
        fun make() { var i = 0; fun count() { i = i + 1; return i; } return count; }
        var a = make();
        var b = make();
        a(); a();
        var ra = a();
        var rb = b();
    "#);
    assert_eq!(global(&interpreter, "ra"), Some(num(3.0)));
    assert_eq!(global(&interpreter, "rb"), Some(num(1.0)));
}

#[test]
fn closure_captures_parameters() {
    let interpreter = run(r#"
        fun adder(n) { fun add(x) { return x + n; } return add; }
        var add5 = adder(5);
        var r = add5(10);
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(15.0)));
}

#[test]
fn resolver_binds_variables_statically() {
    // Classic test: `showA` must keep seeing the *global* `a`, even after a
    // block-local `a` is declared next to it.
    let interpreter = run(r#"
        var a = "global";
        var result;
        {
            fun showA() { return a; }
            var a = "block";
            result = showA();
        }
    "#);
    assert_eq!(global(&interpreter, "result"), Some(text("global")));
}

/* =====================================================================
   Classes, instances, methods
   ===================================================================== */

#[test]
fn class_declaration_defines_a_callable_global() {
    let interpreter = run("class A {}");
    match global(&interpreter, "A") {
        Some(Value::Callable(c)) => assert_eq!(c.name(), "A"),
        other => panic!("expected class callable, got {other:?}"),
    }
}

#[test]
fn calling_a_class_creates_an_instance() {
    let interpreter = run("class A {} var a = A();");
    assert!(matches!(global(&interpreter, "a"), Some(Value::Instance(_))));
}

#[test]
fn instances_of_the_same_class_are_distinct() {
    let interpreter = run(r#"
        class A {}
        var a = A(); var b = A();
        var same = a == a;
        var different = a == b;
    "#);
    assert_eq!(global(&interpreter, "same"), Some(boolean(true)));
    assert_eq!(global(&interpreter, "different"), Some(boolean(false)));
}

#[test]
fn class_values_compare_by_identity() {
    let interpreter = run("class A {} class B {} var r1 = A == A; var r2 = A == B;");
    assert_eq!(global(&interpreter, "r1"), Some(boolean(true)));
    assert_eq!(global(&interpreter, "r2"), Some(boolean(false)));
}

#[test]
fn fields_can_be_set_and_read() {
    let interpreter = run("class A {} var a = A(); a.x = 5; var r = a.x;");
    assert_eq!(global(&interpreter, "r"), Some(num(5.0)));
}

#[test]
fn set_expression_evaluates_to_the_assigned_value() {
    let interpreter = run("class A {} var a = A(); var r = a.x = 9;");
    assert_eq!(global(&interpreter, "r"), Some(num(9.0)));
}

#[test]
fn reading_an_undefined_property_is_a_runtime_error() {
    assert!(fails("class A {} var a = A(); var r = a.missing;"));
}

#[test]
fn property_access_on_non_instances_is_a_runtime_error() {
    assert!(fails("var n = 1; var r = n.x;"));
    assert!(fails("var n = 1; n.x = 2;"));
    assert!(fails("var s = \"str\"; var r = s.length;"));
}

#[test]
fn methods_can_be_called() {
    let interpreter = run(r#"
        class A { hello() { return "hi"; } }
        var r = A().hello();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(text("hi")));
}

#[test]
fn methods_see_this() {
    let interpreter = run(r#"
        class A { get() { return this.v; } }
        var a = A();
        a.v = 9;
        var r = a.get();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(9.0)));
}

#[test]
fn bound_methods_remember_their_instance() {
    let interpreter = run(r#"
        class A { get() { return this.v; } }
        var a = A();
        a.v = 9;
        var m = a.get;
        a.v = 10;
        var r = m();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(10.0)));
}

#[test]
fn fields_shadow_methods() {
    let interpreter = run(r#"
        class A { m() { return 1; } }
        var a = A();
        a.m = 2;
        var r = a.m;
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(2.0)));
}

#[test]
fn initializer_receives_constructor_arguments() {
    let interpreter = run(r#"
        class Point {
            init(x, y) { this.x = x; this.y = y; }
            sum() { return this.x + this.y; }
        }
        var p = Point(1, 2);
        var r = p.sum();
        var px = p.x;
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(3.0)));
    assert_eq!(global(&interpreter, "px"), Some(num(1.0)));
}

#[test]
fn calling_init_directly_returns_the_instance() {
    let interpreter = run(r#"
        class A { init() { this.v = 1; } }
        var a = A();
        var b = a.init();
        var same = a == b;
    "#);
    assert_eq!(global(&interpreter, "same"), Some(boolean(true)));
}

#[test]
fn class_constructor_arity_follows_init() {
    assert!(fails("class A { init(x) {} } A();"));
    assert!(fails("class A { init(x) {} } A(1, 2);"));
    assert!(fails("class A {} A(1);"));
    assert!(!fails("class A { init(x) {} } A(1);"));
}

#[test]
fn method_chaining_with_this() {
    let interpreter = run(r#"
        class Counter {
            init() { this.n = 0; }
            inc() { this.n = this.n + 1; return this; }
        }
        var c = Counter();
        c.inc().inc().inc();
        var r = c.n;
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(3.0)));
}

#[test]
fn instances_are_shared_by_reference() {
    let interpreter = run(r#"
        class Box {}
        var a = Box();
        var b = a;
        b.v = 42;
        var r = a.v;
    "#);
    assert_eq!(global(&interpreter, "r"), Some(num(42.0)));
}

#[test]
fn instance_display_format() {
    let interpreter = run("class Point {} var p = Point();");
    let p = global(&interpreter, "p").unwrap();
    assert_eq!(p.to_string(), "Instance Point.class");
}

/* =====================================================================
   Inheritance & super
   ===================================================================== */

#[test]
fn subclass_inherits_methods() {
    let interpreter = run(r#"
        class A { hello() { return "hello"; } }
        class B < A {}
        var r = B().hello();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(text("hello")));
}

#[test]
fn subclass_overrides_methods() {
    let interpreter = run(r#"
        class A { name() { return "A"; } }
        class B < A { name() { return "B"; } }
        var r = B().name();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(text("B")));
}

#[test]
fn inherited_methods_dispatch_on_the_runtime_class() {
    let interpreter = run(r#"
        class A { name() { return "A"; } hello() { return "hello " + this.name(); } }
        class B < A { name() { return "B"; } }
        var r = B().hello();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(text("hello B")));
}

#[test]
fn super_calls_the_superclass_method() {
    let interpreter = run(r#"
        class A { m() { return "A"; } }
        class B < A { m() { return super.m() + "B"; } }
        var r = B().m();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(text("AB")));
}

#[test]
fn super_skips_the_subclass_override() {
    let interpreter = run(r#"
        class A { who() { return "A"; } }
        class B < A { who() { return "B"; } test() { return super.who(); } }
        var r = B().test();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(text("A")));
}

#[test]
fn super_init_chains_constructors() {
    let interpreter = run(r#"
        class A { init(x) { this.x = x; } }
        class B < A { init(x) { super.init(x); this.y = 2; } }
        var b = B(5);
        var rx = b.x;
        var ry = b.y;
    "#);
    assert_eq!(global(&interpreter, "rx"), Some(num(5.0)));
    assert_eq!(global(&interpreter, "ry"), Some(num(2.0)));
}

#[test]
fn three_level_inheritance() {
    let interpreter = run(r#"
        class A { a() { return "a"; } }
        class B < A { b() { return "b"; } }
        class C < B { c() { return "c"; } }
        var o = C();
        var r = o.a() + o.b() + o.c();
    "#);
    assert_eq!(global(&interpreter, "r"), Some(text("abc")));
}

#[test]
fn inheriting_from_a_non_class_is_a_runtime_error() {
    assert!(fails("var NotAClass = 1; class B < NotAClass {}"));
    assert!(fails("fun f() {} class B < f {}"));
}

#[test]
fn super_with_a_missing_method_is_a_runtime_error() {
    assert!(fails(r#"
        class A {}
        class B < A { m() { return super.missing(); } }
        B().m();
    "#));
}

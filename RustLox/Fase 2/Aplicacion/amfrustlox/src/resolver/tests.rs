//! Tests for `Resolver` (src/resolver.rs).
//!
//! The resolver reports problems as `Err(..)` and otherwise records variable
//! depths inside the `Interpreter`. These tests check which programs are
//! accepted/rejected; the resulting scoping *behaviour* (depths actually being
//! honoured at runtime) is covered in `interpreter/tests.rs`.
use super::*;
use crate::{parser::Parser, token::Scanner};
use std::{cell::RefCell, rc::Rc};

/* ---------- helpers ---------- */

fn resolve_src(src: &str) -> Result<Vec<Stmt>, Error> {
    let tokens = Scanner::new(src).scan_tokens().expect("scan should succeed");
    let statements = match Parser::new(tokens).parse() {
        Ok(s) => s,
        Err(_) => panic!("`{src}` should parse before being resolved"),
    };
    let mut interpreter = Interpreter::new(Rc::new(RefCell::new(Vec::<u8>::new())));
    Resolver::new(&mut interpreter).resolve(&statements)
}

fn accepts(src: &str) -> bool {
    resolve_src(src).is_ok()
}

fn rejects(src: &str) -> bool {
    resolve_src(src).is_err()
}

/* =====================================================================
   Basics
   ===================================================================== */

#[test]
fn empty_program_resolves() {
    match resolve_src("") {
        Ok(statements) => assert!(statements.is_empty()),
        Err(_) => panic!("empty program must resolve"),
    }
}

#[test]
fn resolve_returns_the_same_number_of_statements() {
    match resolve_src("var a = 1; print a; { var b = 2; }") {
        Ok(statements) => assert_eq!(statements.len(), 3),
        Err(_) => panic!("program should resolve"),
    }
}

#[test]
fn simple_expressions_and_prints_resolve() {
    assert!(accepts("print 1 + 2 * 3;"));
    assert!(accepts("print !true or false and nil;"));
    assert!(accepts("print (1 + 2) * -3;"));
    assert!(accepts("\"a\" + \"b\";"));
}

/* =====================================================================
   Variable declarations & scopes
   ===================================================================== */

#[test]
fn globals_may_be_redeclared() {
    assert!(accepts("var a = 1; var a = 2;"));
}

#[test]
fn undeclared_globals_are_not_a_resolver_error() {
    // Global lookups are checked at runtime, not by the resolver.
    assert!(accepts("print missing;"));
    assert!(accepts("missing = 1;"));
}

#[test]
fn local_variable_cannot_be_redeclared_in_the_same_scope() {
    assert!(rejects("{ var a = 1; var a = 2; }"));
}

#[test]
fn shadowing_in_a_nested_scope_is_allowed() {
    assert!(accepts("{ var a = 1; { var a = 2; } }"));
    assert!(accepts("var a = 1; { var a = 2; }"));
}

#[test]
fn same_name_in_sibling_scopes_is_allowed() {
    assert!(accepts("{ var a = 1; } { var a = 2; }"));
}

#[test]
fn local_cannot_read_itself_in_its_own_initializer() {
    assert!(rejects("{ var a = a; }"));
    assert!(rejects("{ var a = 1 + a; }"));
    assert!(rejects("{ var f = f(); }"));
}

#[test]
fn local_may_read_an_outer_variable_in_its_initializer() {
    assert!(accepts("var a = 1; { var b = a; }"));
}

#[test]
fn shadowing_variable_reading_itself_in_initializer_is_rejected() {
    // The inner `a` is already "declared" when its own initializer is resolved.
    assert!(rejects("{ var a = 1; { var a = a; } }"));
}

#[test]
fn uninitialized_local_resolves() {
    assert!(accepts("{ var a; print a; }"));
}

#[test]
fn assignment_to_locals_and_globals_resolves() {
    assert!(accepts("{ var a = 1; a = a + 1; }"));
    assert!(accepts("var a = 1; a = 2;"));
}

/* =====================================================================
   Control flow
   ===================================================================== */

#[test]
fn if_else_and_while_bodies_are_resolved() {
    assert!(accepts(
        "{ var a = 1; if (a) { var b = a; } else { var c = a; } while (a) { var d = a; } }"
    ));
}

#[test]
fn errors_inside_branches_are_propagated() {
    assert!(rejects("{ if (true) { var a = a; } }"));
    assert!(rejects("{ if (false) print 1; else { var a = a; } }"));
    assert!(rejects("{ while (true) { var a = a; } }"));
}

#[test]
fn error_in_condition_is_propagated() {
    assert!(rejects("{ var a = a; if (a) print 1; }"));
}

#[test]
fn desugared_for_loop_resolves() {
    assert!(accepts("for (var i = 0; i < 3; i = i + 1) print i;"));
}

/* =====================================================================
   Functions
   ===================================================================== */

#[test]
fn function_declaration_resolves() {
    assert!(accepts("fun f(a, b) { return a + b; }"));
}

#[test]
fn recursive_function_resolves() {
    assert!(accepts("fun f(n) { return f(n - 1); }"));
    assert!(accepts("{ fun f(n) { return f(n - 1); } }"));
}

#[test]
fn duplicate_parameters_are_rejected() {
    assert!(rejects("fun f(a, a) {}"));
}

#[test]
fn body_variable_cannot_redeclare_a_parameter() {
    assert!(rejects("fun f(a) { var a = 1; }"));
}

#[test]
fn body_variable_may_shadow_in_a_nested_block() {
    assert!(accepts("fun f(a) { { var a = 1; } }"));
}

#[test]
fn local_function_name_cannot_be_redeclared() {
    assert!(rejects("{ fun f() {} fun f() {} }"));
    assert!(rejects("{ var f = 1; fun f() {} }"));
}

#[test]
fn closures_capturing_locals_resolve() {
    assert!(accepts(
        "fun make() { var i = 0; fun inc() { i = i + 1; return i; } return inc; }"
    ));
}

#[test]
fn errors_inside_function_bodies_are_propagated() {
    assert!(rejects("fun f() { var a = a; }"));
    assert!(rejects("fun f() { { var a = 1; var a = 2; } }"));
}

#[test]
fn call_arguments_are_resolved() {
    assert!(rejects("{ var a = 1; var b = f(b); }"));
    assert!(accepts("fun f(x) {} { var a = 1; f(a); }"));
}

#[test]
fn return_statements() {
    assert!(accepts("fun f() { return; }"));
    assert!(accepts("fun f() { return 1; }"));
}

/* =====================================================================
   Classes
   ===================================================================== */

#[test]
fn simple_class_resolves() {
    assert!(accepts("class A {}"));
    assert!(accepts("class A { m() { return 1; } }"));
}

#[test]
fn this_is_valid_inside_methods() {
    assert!(accepts("class A { m() { return this; } }"));
    assert!(accepts("class A { init(x) { this.x = x; } }"));
}

#[test]
fn this_is_valid_in_a_function_nested_in_a_method() {
    assert!(accepts(
        "class A { m() { fun g() { return this; } return g; } }"
    ));
}

#[test]
fn this_outside_a_class_is_rejected() {
    assert!(rejects("print this;"));
    assert!(rejects("fun f() { return this; }"));
    assert!(rejects("{ var a = this; }"));
}

#[test]
fn initializer_may_return_without_a_value() {
    assert!(accepts("class A { init() { return; } }"));
}

#[test]
fn initializer_cannot_return_a_value() {
    assert!(rejects("class A { init() { return 1; } }"));
    assert!(rejects("class A { init() { return this; } }"));
}

#[test]
fn regular_methods_may_return_values() {
    assert!(accepts("class A { get() { return 1; } }"));
}

#[test]
fn class_cannot_inherit_from_itself() {
    assert!(rejects("class A < A {}"));
}

#[test]
fn class_may_inherit_from_another_class() {
    assert!(accepts("class A {} class B < A {}"));
}

#[test]
fn super_is_valid_in_a_subclass() {
    assert!(accepts(
        "class A { m() {} } class B < A { m() { return super.m(); } }"
    ));
    assert!(accepts("class A {} class B < A { m() { return super.m; } }"));
}

#[test]
fn super_outside_a_class_is_rejected() {
    assert!(rejects("print super.x;"));
    assert!(rejects("fun f() { return super.x; }"));
}

#[test]
fn super_in_a_class_without_superclass_is_rejected() {
    assert!(rejects("class A { m() { return super.m(); } }"));
}

#[test]
fn property_get_and_set_are_resolved() {
    assert!(accepts("{ var o = 1; o.x = o; var y = o.x; }"));
    assert!(rejects("{ var o = o.x; }"));
    assert!(accepts("{ var o = 1; var v = o; o.x = v; }"));
}

#[test]
fn class_in_a_block_resolves() {
    assert!(accepts("{ class A { m() { return this; } } var a = A(); }"));
}

#[test]
fn class_state_is_restored_after_a_class_declaration() {
    // `this` must be rejected again once the class body has been left.
    assert!(rejects("class A { m() { return this; } } print this;"));
}

//! Tests for `Parser`, `Expr`, `Stmt` and `Operator` (src/parser.rs).
use super::*;
use crate::token::Scanner;

/* ---------- helpers ---------- */

fn parser_for(src: &str) -> Parser {
    Parser::new(Scanner::new(src).scan_tokens().expect("scan should succeed"))
}

/// Parses a single expression (no trailing `;` needed).
fn parse_expr(src: &str) -> Expr {
    match parser_for(src).parse_expression() {
        Ok(e) => e,
        Err(_) => panic!("`{src}` should parse as an expression"),
    }
}

fn expr_fails(src: &str) -> bool {
    parser_for(src).parse_expression().is_err()
}

/// Parses a whole program.
fn parse_program(src: &str) -> Vec<Stmt> {
    match parser_for(src).parse() {
        Ok(s) => s,
        Err(_) => panic!("`{src}` should parse as a program"),
    }
}

fn program_fails(src: &str) -> bool {
    parser_for(src).parse().is_err()
}

fn num(v: &str) -> Expr {
    Expr::numeric(v)
}

/* =====================================================================
   Literals & grouping
   ===================================================================== */

#[test]
fn parses_literals() {
    assert_eq!(parse_expr("42"), Expr::numeric("42"));
    assert_eq!(parse_expr("3.5"), Expr::numeric("3.5"));
    assert_eq!(parse_expr("\"hi\""), Expr::str("hi"));
    assert_eq!(parse_expr("true"), Expr::bool(true));
    assert_eq!(parse_expr("false"), Expr::bool(false));
    assert_eq!(parse_expr("nil"), Expr::nil());
}

#[test]
fn parses_grouping() {
    assert_eq!(parse_expr("(1)"), Expr::grouping(num("1")));
    assert_eq!(
        parse_expr("((1))"),
        Expr::grouping(Expr::grouping(num("1")))
    );
}

#[test]
fn unclosed_grouping_is_an_error() {
    assert!(expr_fails("(1"));
    assert!(expr_fails("(1 + 2"));
}

#[test]
fn empty_input_is_not_an_expression() {
    assert!(expr_fails(""));
}

#[test]
fn stray_operator_is_not_an_expression() {
    assert!(expr_fails("*"));
    assert!(expr_fails("1 +"));
}

/* =====================================================================
   Unary
   ===================================================================== */

#[test]
fn parses_unary_operators() {
    assert_eq!(
        parse_expr("-1"),
        Expr::unary(Operator::Subtract, num("1"))
    );
    assert_eq!(
        parse_expr("!true"),
        Expr::unary(Operator::Negation, Expr::bool(true))
    );
}

#[test]
fn unary_operators_nest() {
    assert_eq!(
        parse_expr("!!true"),
        Expr::unary(
            Operator::Negation,
            Expr::unary(Operator::Negation, Expr::bool(true))
        )
    );
    assert_eq!(
        parse_expr("--1"),
        Expr::unary(Operator::Subtract, Expr::unary(Operator::Subtract, num("1")))
    );
}

#[test]
fn unary_binds_tighter_than_multiplication() {
    assert_eq!(
        parse_expr("-1 * 2"),
        Expr::binary(
            Expr::unary(Operator::Subtract, num("1")),
            Operator::Multiply,
            num("2")
        )
    );
}

#[test]
fn unary_binds_tighter_than_equality() {
    assert_eq!(
        parse_expr("!true == false"),
        Expr::binary(
            Expr::unary(Operator::Negation, Expr::bool(true)),
            Operator::Equal,
            Expr::bool(false)
        )
    );
}

/* =====================================================================
   Binary: precedence & associativity
   ===================================================================== */

#[test]
fn parses_every_binary_operator() {
    let cases = [
        ("1 + 2", Operator::Add),
        ("1 - 2", Operator::Subtract),
        ("1 * 2", Operator::Multiply),
        ("1 / 2", Operator::Divide),
        ("1 < 2", Operator::Less),
        ("1 <= 2", Operator::LessEqual),
        ("1 > 2", Operator::Greater),
        ("1 >= 2", Operator::GreaterEqual),
        ("1 == 2", Operator::Equal),
        ("1 != 2", Operator::Inequal),
    ];
    for (src, op) in cases {
        assert_eq!(parse_expr(src), Expr::binary(num("1"), op, num("2")), "`{src}`");
    }
}

#[test]
fn multiplication_binds_tighter_than_addition() {
    assert_eq!(
        parse_expr("1 + 2 * 3"),
        Expr::binary(
            num("1"),
            Operator::Add,
            Expr::binary(num("2"), Operator::Multiply, num("3"))
        )
    );
}

#[test]
fn division_binds_tighter_than_subtraction() {
    assert_eq!(
        parse_expr("1 - 4 / 2"),
        Expr::binary(
            num("1"),
            Operator::Subtract,
            Expr::binary(num("4"), Operator::Divide, num("2"))
        )
    );
}

#[test]
fn subtraction_is_left_associative() {
    assert_eq!(
        parse_expr("10 - 4 - 3"),
        Expr::binary(
            Expr::binary(num("10"), Operator::Subtract, num("4")),
            Operator::Subtract,
            num("3")
        )
    );
}

#[test]
fn division_is_left_associative() {
    assert_eq!(
        parse_expr("8 / 2 / 2"),
        Expr::binary(
            Expr::binary(num("8"), Operator::Divide, num("2")),
            Operator::Divide,
            num("2")
        )
    );
}

#[test]
fn grouping_overrides_precedence() {
    assert_eq!(
        parse_expr("(1 + 2) * 3"),
        Expr::binary(
            Expr::grouping(Expr::binary(num("1"), Operator::Add, num("2"))),
            Operator::Multiply,
            num("3")
        )
    );
}

#[test]
fn addition_binds_tighter_than_comparison() {
    assert_eq!(
        parse_expr("1 + 2 < 4"),
        Expr::binary(
            Expr::binary(num("1"), Operator::Add, num("2")),
            Operator::Less,
            num("4")
        )
    );
}

#[test]
fn comparison_binds_tighter_than_equality() {
    assert_eq!(
        parse_expr("1 < 2 == true"),
        Expr::binary(
            Expr::binary(num("1"), Operator::Less, num("2")),
            Operator::Equal,
            Expr::bool(true)
        )
    );
}

#[test]
fn and_binds_tighter_than_or() {
    assert_eq!(
        parse_expr("true or false and nil"),
        Expr::binary(
            Expr::bool(true),
            Operator::Or,
            Expr::binary(Expr::bool(false), Operator::And, Expr::nil())
        )
    );
}

#[test]
fn equality_binds_tighter_than_and() {
    assert_eq!(
        parse_expr("true and 1 == 1"),
        Expr::binary(
            Expr::bool(true),
            Operator::And,
            Expr::binary(num("1"), Operator::Equal, num("1"))
        )
    );
}

#[test]
fn or_is_left_associative() {
    assert_eq!(
        parse_expr("true or false or nil"),
        Expr::binary(
            Expr::binary(Expr::bool(true), Operator::Or, Expr::bool(false)),
            Operator::Or,
            Expr::nil()
        )
    );
}

/* =====================================================================
   Variables, assignment
   ===================================================================== */

#[test]
fn parses_variable() {
    match parse_expr("foo") {
        Expr::Variable { name } => {
            assert_eq!(name.lexeme, "foo");
            assert_eq!(name.token_type, TokenType::Identifier);
        }
        other => panic!("expected variable, got {other:?}"),
    }
}

#[test]
fn parses_assignment() {
    match parse_expr("a = 1") {
        Expr::Assign { name, value } => {
            assert_eq!(name.lexeme, "a");
            assert_eq!(*value, num("1"));
        }
        other => panic!("expected assignment, got {other:?}"),
    }
}

#[test]
fn assignment_is_right_associative() {
    match parse_expr("a = b = 1") {
        Expr::Assign { name, value } => {
            assert_eq!(name.lexeme, "a");
            match *value {
                Expr::Assign { name, value } => {
                    assert_eq!(name.lexeme, "b");
                    assert_eq!(*value, num("1"));
                }
                other => panic!("expected nested assignment, got {other:?}"),
            }
        }
        other => panic!("expected assignment, got {other:?}"),
    }
}

#[test]
fn assignment_has_lowest_precedence() {
    match parse_expr("a = 1 + 2") {
        Expr::Assign { value, .. } => assert_eq!(
            *value,
            Expr::binary(num("1"), Operator::Add, num("2"))
        ),
        other => panic!("expected assignment, got {other:?}"),
    }
}

#[test]
fn invalid_assignment_targets_are_rejected() {
    assert!(expr_fails("1 = 2"));
    assert!(expr_fails("a + b = 1"));
    assert!(expr_fails("(a) = 1"));
    assert!(expr_fails("\"s\" = 1"));
}

/* =====================================================================
   Calls, property access
   ===================================================================== */

#[test]
fn parses_call_without_arguments() {
    match parse_expr("f()") {
        Expr::Call {
            callee, arguments, ..
        } => {
            assert!(matches!(*callee, Expr::Variable { .. }));
            assert!(arguments.is_empty());
        }
        other => panic!("expected call, got {other:?}"),
    }
}

#[test]
fn parses_call_with_arguments() {
    match parse_expr("f(1, \"a\", true)") {
        Expr::Call { arguments, .. } => {
            assert_eq!(arguments, vec![num("1"), Expr::str("a"), Expr::bool(true)]);
        }
        other => panic!("expected call, got {other:?}"),
    }
}

#[test]
fn call_arguments_are_full_expressions() {
    match parse_expr("f(1 + 2, g(3))") {
        Expr::Call { arguments, .. } => {
            assert_eq!(arguments.len(), 2);
            assert_eq!(
                arguments[0],
                Expr::binary(num("1"), Operator::Add, num("2"))
            );
            assert!(matches!(arguments[1], Expr::Call { .. }));
        }
        other => panic!("expected call, got {other:?}"),
    }
}

#[test]
fn chained_calls_nest_on_the_callee() {
    match parse_expr("f(1)(2)") {
        Expr::Call {
            callee, arguments, ..
        } => {
            assert_eq!(arguments, vec![num("2")]);
            assert!(matches!(*callee, Expr::Call { .. }));
        }
        other => panic!("expected call, got {other:?}"),
    }
}

#[test]
fn malformed_calls_are_errors() {
    assert!(expr_fails("f(1"));
    assert!(expr_fails("f(1,"));
    assert!(expr_fails("f(,)"));
}

#[test]
fn parses_property_get() {
    match parse_expr("a.b") {
        Expr::Get { expr, name } => {
            assert!(matches!(*expr, Expr::Variable { .. }));
            assert_eq!(name.lexeme, "b");
        }
        other => panic!("expected get, got {other:?}"),
    }
}

#[test]
fn property_gets_chain_left_to_right() {
    match parse_expr("a.b.c") {
        Expr::Get { expr, name } => {
            assert_eq!(name.lexeme, "c");
            assert!(matches!(*expr, Expr::Get { .. }));
        }
        other => panic!("expected get, got {other:?}"),
    }
}

#[test]
fn parses_method_call_on_property() {
    match parse_expr("a.m(1)") {
        Expr::Call { callee, arguments, .. } => {
            assert!(matches!(*callee, Expr::Get { .. }));
            assert_eq!(arguments.len(), 1);
        }
        other => panic!("expected call, got {other:?}"),
    }
}

#[test]
fn parses_property_set() {
    match parse_expr("a.b = 1") {
        Expr::Set { expr, name, value } => {
            assert!(matches!(*expr, Expr::Variable { .. }));
            assert_eq!(name.lexeme, "b");
            assert_eq!(*value, num("1"));
        }
        other => panic!("expected set, got {other:?}"),
    }
}

#[test]
fn nested_property_set_targets_the_last_property() {
    match parse_expr("a.b.c = 1") {
        Expr::Set { expr, name, .. } => {
            assert_eq!(name.lexeme, "c");
            assert!(matches!(*expr, Expr::Get { .. }));
        }
        other => panic!("expected set, got {other:?}"),
    }
}

#[test]
fn dot_without_property_name_is_an_error() {
    assert!(expr_fails("a."));
    assert!(expr_fails("a.1"));
}

#[test]
fn parses_this() {
    match parse_expr("this") {
        Expr::This { keyword } => assert_eq!(keyword.lexeme, "this"),
        other => panic!("expected this, got {other:?}"),
    }
}

#[test]
fn parses_this_property_access() {
    match parse_expr("this.x") {
        Expr::Get { expr, name } => {
            assert!(matches!(*expr, Expr::This { .. }));
            assert_eq!(name.lexeme, "x");
        }
        other => panic!("expected get, got {other:?}"),
    }
}

#[test]
fn parses_super_method_access() {
    match parse_expr("super.m") {
        Expr::Super { keyword, method } => {
            assert_eq!(keyword.lexeme, "super");
            assert_eq!(method.lexeme, "m");
        }
        other => panic!("expected super, got {other:?}"),
    }
}

#[test]
fn bare_super_is_an_error() {
    assert!(expr_fails("super"));
    assert!(expr_fails("super."));
    assert!(expr_fails("super.1"));
}

/* =====================================================================
   Expr / Operator helpers
   ===================================================================== */

#[test]
fn operator_from_token_covers_all_operators() {
    let cases = [
        (TokenType::Minus, Operator::Subtract),
        (TokenType::Plus, Operator::Add),
        (TokenType::Slash, Operator::Divide),
        (TokenType::Star, Operator::Multiply),
        (TokenType::Bang, Operator::Negation),
        (TokenType::BangEqual, Operator::Inequal),
        (TokenType::EqualEqual, Operator::Equal),
        (TokenType::Greater, Operator::Greater),
        (TokenType::GreaterEqual, Operator::GreaterEqual),
        (TokenType::Less, Operator::Less),
        (TokenType::LessEqual, Operator::LessEqual),
        (TokenType::And, Operator::And),
        (TokenType::Or, Operator::Or),
    ];
    for (token_type, expected) in cases {
        let token = Token::new(token_type, "?", 1, Literal::None, 0);
        assert_eq!(Operator::from(&token), expected);
    }
}

#[test]
#[should_panic(expected = "Invalid operator")]
fn operator_from_non_operator_token_panics() {
    let token = Token::new(TokenType::Semicolon, ";", 1, Literal::None, 0);
    let _ = Operator::from(&token);
}

#[test]
fn expr_constructors_build_expected_variants() {
    assert_eq!(
        Expr::str("x"),
        Expr::String {
            value: "x".to_string()
        }
    );
    assert_eq!(Expr::bool(true), Expr::Bool { value: true });
    assert_eq!(
        Expr::numeric("1"),
        Expr::Numeric {
            value: "1".to_string()
        }
    );
    assert_eq!(Expr::nil(), Expr::Nil);
    assert_eq!(
        Expr::grouping(Expr::nil()),
        Expr::Grouping {
            expr: Box::new(Expr::Nil)
        }
    );
}

/* =====================================================================
   Statements
   ===================================================================== */

#[test]
fn empty_program_has_no_statements() {
    assert!(parse_program("").is_empty());
    assert!(parse_program("   // just a comment\n").is_empty());
}

#[test]
fn parses_print_statement() {
    let stmts = parse_program("print 1 + 2;");
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Stmt::Print { expr } => {
            assert_eq!(*expr, Expr::binary(num("1"), Operator::Add, num("2")))
        }
        other => panic!("expected print, got {other:?}"),
    }
}

#[test]
fn parses_expression_statement() {
    let stmts = parse_program("1 + 2;");
    assert!(matches!(stmts[0], Stmt::Expression { .. }));
}

#[test]
fn statements_require_semicolons() {
    assert!(program_fails("print 1"));
    assert!(program_fails("1 + 2"));
    assert!(program_fails("var a = 1"));
    assert!(program_fails("return 1"));
}

#[test]
fn parses_multiple_statements_in_order() {
    let stmts = parse_program("var a = 1; print a; a = 2;");
    assert_eq!(stmts.len(), 3);
    assert!(matches!(stmts[0], Stmt::Var { .. }));
    assert!(matches!(stmts[1], Stmt::Print { .. }));
    assert!(matches!(stmts[2], Stmt::Expression { .. }));
}

#[test]
fn parses_var_declaration_with_initializer() {
    match &parse_program("var a = 1;")[0] {
        Stmt::Var { name, initializar } => {
            assert_eq!(name.lexeme, "a");
            assert_eq!(initializar, &Some(num("1")));
        }
        other => panic!("expected var, got {other:?}"),
    }
}

#[test]
fn parses_var_declaration_without_initializer() {
    match &parse_program("var a;")[0] {
        Stmt::Var { name, initializar } => {
            assert_eq!(name.lexeme, "a");
            assert!(initializar.is_none());
        }
        other => panic!("expected var, got {other:?}"),
    }
}

#[test]
fn malformed_var_declarations_are_errors() {
    assert!(program_fails("var = 1;"));
    assert!(program_fails("var 1 = 1;"));
    assert!(program_fails("var a = ;"));
}

#[test]
fn parses_block() {
    match &parse_program("{ var a = 1; print a; }")[0] {
        Stmt::Block { statements } => assert_eq!(statements.len(), 2),
        other => panic!("expected block, got {other:?}"),
    }
}

#[test]
fn parses_empty_and_nested_blocks() {
    match &parse_program("{ { } }")[0] {
        Stmt::Block { statements } => {
            assert_eq!(statements.len(), 1);
            match &statements[0] {
                Stmt::Block { statements } => assert!(statements.is_empty()),
                other => panic!("expected inner block, got {other:?}"),
            }
        }
        other => panic!("expected block, got {other:?}"),
    }
}

#[test]
fn unclosed_block_is_an_error() {
    assert!(program_fails("{ print 1;"));
    assert!(program_fails("{"));
}

#[test]
fn parses_if_without_else() {
    match &parse_program("if (true) print 1;")[0] {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
        } => {
            assert_eq!(*condition, Expr::bool(true));
            assert!(matches!(**then_branch, Stmt::Print { .. }));
            assert!(else_branch.is_none());
        }
        other => panic!("expected if, got {other:?}"),
    }
}

#[test]
fn parses_if_with_else() {
    match &parse_program("if (false) print 1; else print 2;")[0] {
        Stmt::If { else_branch, .. } => {
            let else_branch = else_branch.as_ref().expect("else branch");
            assert!(matches!(**else_branch, Stmt::Print { .. }));
        }
        other => panic!("expected if, got {other:?}"),
    }
}

#[test]
fn dangling_else_binds_to_nearest_if() {
    match &parse_program("if (true) if (false) print 1; else print 2;")[0] {
        Stmt::If {
            then_branch,
            else_branch,
            ..
        } => {
            assert!(else_branch.is_none(), "outer if must not own the else");
            match &**then_branch {
                Stmt::If { else_branch, .. } => assert!(else_branch.is_some()),
                other => panic!("expected inner if, got {other:?}"),
            }
        }
        other => panic!("expected if, got {other:?}"),
    }
}

#[test]
fn malformed_if_is_an_error() {
    assert!(program_fails("if true) print 1;"));
    assert!(program_fails("if (true print 1;"));
    assert!(program_fails("if (true)"));
}

#[test]
fn parses_while() {
    match &parse_program("while (true) print 1;")[0] {
        Stmt::While { contion, body } => {
            assert_eq!(*contion, Expr::bool(true));
            assert!(matches!(**body, Stmt::Print { .. }));
        }
        other => panic!("expected while, got {other:?}"),
    }
}

#[test]
fn malformed_while_is_an_error() {
    assert!(program_fails("while true) print 1;"));
    assert!(program_fails("while (true print 1;"));
}

#[test]
fn for_loop_desugars_to_block_with_while() {
    let stmts = parse_program("for (var i = 0; i < 3; i = i + 1) print i;");
    match &stmts[0] {
        Stmt::Block { statements } => {
            assert_eq!(statements.len(), 2);
            assert!(matches!(statements[0], Stmt::Var { .. }));
            match &statements[1] {
                Stmt::While { body, .. } => match &**body {
                    Stmt::Block { statements } => {
                        assert_eq!(statements.len(), 2);
                        assert!(matches!(statements[0], Stmt::Print { .. }));
                        assert!(matches!(statements[1], Stmt::Expression { .. }));
                    }
                    other => panic!("expected loop body block, got {other:?}"),
                },
                other => panic!("expected while, got {other:?}"),
            }
        }
        other => panic!("expected block, got {other:?}"),
    }
}

#[test]
fn for_loop_accepts_expression_initializer() {
    assert!(!program_fails("for (i = 0; i < 3; i = i + 1) print i;"));
}

#[test]
fn malformed_for_is_an_error() {
    assert!(program_fails("for var i = 0; i < 3; i = i + 1) print i;"));
    assert!(program_fails("for (var i = 0; i < 3 i = i + 1) print i;"));
}

#[test]
fn for_loop_without_increment_parses() {
    assert!(!program_fails("for (var i = 0; i < 3;) print i;"));
}

#[test]
fn for_loop_without_initializer_parses() {
    assert!(!program_fails("for (; true; i = i + 1) print i;"));
}

#[test]
fn parses_return_with_value() {
    match &parse_program("return 1;")[0] {
        Stmt::Return { keyword, value } => {
            assert_eq!(keyword.lexeme, "return");
            assert_eq!(*value, num("1"));
        }
        other => panic!("expected return, got {other:?}"),
    }
}

#[test]
fn parses_return_without_value() {
    match &parse_program("return;")[0] {
        Stmt::Return { value, .. } => assert_eq!(*value, Expr::Nil),
        other => panic!("expected return, got {other:?}"),
    }
}

/* =====================================================================
   Functions & classes
   ===================================================================== */

#[test]
fn parses_function_declaration() {
    match &parse_program("fun add(a, b) { return a + b; }")[0] {
        Stmt::Function { name, params, body } => {
            assert_eq!(name.lexeme, "add");
            let names: Vec<&str> = params.iter().map(|p| p.lexeme.as_str()).collect();
            assert_eq!(names, vec!["a", "b"]);
            assert_eq!(body.len(), 1);
            assert!(matches!(body[0], Stmt::Return { .. }));
        }
        other => panic!("expected function, got {other:?}"),
    }
}

#[test]
fn parses_function_without_params_or_body() {
    match &parse_program("fun f() {}")[0] {
        Stmt::Function { params, body, .. } => {
            assert!(params.is_empty());
            assert!(body.is_empty());
        }
        other => panic!("expected function, got {other:?}"),
    }
}

#[test]
fn malformed_functions_are_errors() {
    assert!(program_fails("fun () {}"));
    assert!(program_fails("fun f( {}"));
    assert!(program_fails("fun f(a,) {}"));
    assert!(program_fails("fun f(1) {}"));
    assert!(program_fails("fun f() print 1;"));
    assert!(program_fails("fun f() { print 1;"));
}

#[test]
fn parses_empty_class() {
    match &parse_program("class A {}")[0] {
        Stmt::Class {
            name,
            methods,
            superclass,
        } => {
            assert_eq!(name.lexeme, "A");
            assert!(methods.is_empty());
            assert!(superclass.is_none());
        }
        other => panic!("expected class, got {other:?}"),
    }
}

#[test]
fn parses_class_with_methods() {
    match &parse_program("class A { init(x) { this.x = x; } get() { return this.x; } }")[0] {
        Stmt::Class { methods, .. } => {
            assert_eq!(methods.len(), 2);
            let names: Vec<String> = methods
                .iter()
                .map(|m| match m {
                    Stmt::Function { name, .. } => name.lexeme.clone(),
                    other => panic!("method should be a function, got {other:?}"),
                })
                .collect();
            assert_eq!(names, vec!["init", "get"]);
        }
        other => panic!("expected class, got {other:?}"),
    }
}

#[test]
fn parses_class_with_superclass() {
    match &parse_program("class B < A {}")[0] {
        Stmt::Class {
            name, superclass, ..
        } => {
            assert_eq!(name.lexeme, "B");
            match superclass {
                Some(Expr::Variable { name }) => assert_eq!(name.lexeme, "A"),
                other => panic!("expected superclass variable, got {other:?}"),
            }
        }
        other => panic!("expected class, got {other:?}"),
    }
}

#[test]
fn malformed_classes_are_errors() {
    assert!(program_fails("class { }"));
    assert!(program_fails("class A < { }"));
    assert!(program_fails("class A"));
    assert!(program_fails("class A { m() }"));
    assert!(program_fails("class A { m() {} "));
}

#[test]
fn for_loop_without_initializer_has_no_wrapping_block() {
    let stmts = parse_program("for (; true; i = i + 1) print i;");
    assert!(matches!(stmts[0], Stmt::While { .. }));
}

#[test]
fn for_loop_without_increment_uses_the_body_directly() {
    match &parse_program("for (var i = 0; i < 3;) print i;")[0] {
        Stmt::Block { statements } => match &statements[1] {
            Stmt::While { body, .. } => assert!(matches!(**body, Stmt::Print { .. })),
            other => panic!("expected while, got {other:?}"),
        },
        other => panic!("expected block, got {other:?}"),
    }
}

#[test]
fn for_loop_without_condition_loops_forever_by_default() {
    match &parse_program("for (;;) print 1;")[0] {
        Stmt::While { contion, .. } => assert_eq!(*contion, Expr::bool(true)),
        other => panic!("expected while, got {other:?}"),
    }
}

/* =====================================================================
   Diagnostics (line, location and message of syntax errors)
   ===================================================================== */

use crate::error::Diagnostic;

fn syntax_error(src: &str) -> Diagnostic {
    match parser_for(src).parse() {
        Err(Error::Syntax(d)) => d,
        Err(other) => panic!("expected a syntax error for `{src}`, got {other:?}"),
        Ok(_) => panic!("`{src}` should not parse"),
    }
}

fn assert_syntax_error(src: &str, line: usize, location: &str, message: &str) {
    let d = syntax_error(src);
    assert_eq!(
        (d.line, d.location.as_str(), d.message.as_str()),
        (line, location, message),
        "source: {src:?}"
    );
}

#[test]
fn diagnostic_display_is_lox_style() {
    assert_eq!(
        syntax_error("print 1").to_string(),
        "[line 1] Error at end: Expect ';' after value."
    );
}

#[test]
fn errors_at_end_of_input_say_at_end() {
    assert_syntax_error("print 1", 1, " at end", "Expect ';' after value.");
    assert_syntax_error("1 + 2", 1, " at end", "Expect ';' after expression.");
    assert_syntax_error("var a = 1", 1, " at end", "Expect ';' after variable declaration.");
    assert_syntax_error("return 1", 1, " at end", "Expect ';' after return value.");
    assert_syntax_error("{ print 1;", 1, " at end", "Expect '}' after block.");
}

#[test]
fn errors_name_the_offending_token() {
    assert_syntax_error("print 1 2;", 1, " at '2'", "Expect ';' after value.");
    assert_syntax_error("var = 1;", 1, " at '='", "Expect variable name.");
}

#[test]
fn missing_expressions() {
    assert_syntax_error("print ;", 1, " at ';'", "Expect expression.");
    assert_syntax_error("var a = ;", 1, " at ';'", "Expect expression.");
    assert_syntax_error("*", 1, " at '*'", "Expect expression.");
    assert_syntax_error("1 + ;", 1, " at ';'", "Expect expression.");
}

#[test]
fn grouping_and_call_errors() {
    assert_syntax_error("(1", 1, " at end", "Expect ')' after expression.");
    assert_syntax_error("f(1", 1, " at end", "Expect ')' after arguments.");
    assert_syntax_error("a.", 1, " at end", "Expect property name after '.'.");
    assert_syntax_error("super", 1, " at end", "Expect '.' after 'super'.");
    assert_syntax_error("super.", 1, " at end", "Expect superclass method name.");
}

#[test]
fn invalid_assignment_target_points_at_the_equals_sign() {
    assert_syntax_error("1 = 2;", 1, " at '='", "Invalid assignment target.");
    assert_syntax_error("a + b = 1;", 1, " at '='", "Invalid assignment target.");
    assert_syntax_error("(a) = 1;", 1, " at '='", "Invalid assignment target.");
}

#[test]
fn control_flow_errors() {
    assert_syntax_error("if true) print 1;", 1, " at 'true'", "Expect '(' after 'if'.");
    assert_syntax_error("if (true print 1;", 1, " at 'print'", "Expect ')' after if condition.");
    assert_syntax_error("while true) {}", 1, " at 'true'", "Expect '(' after 'while'.");
    assert_syntax_error("while (true print 1;", 1, " at 'print'", "Expect ')' after condition.");
    assert_syntax_error("for var i;", 1, " at 'var'", "Expect '(' after 'for'.");
    assert_syntax_error(
        "for (var i = 0; i < 3 i = i + 1) {}",
        1,
        " at 'i'",
        "Expect ';' after loop condition.",
    );
    assert_syntax_error(
        "for (var i = 0; i < 3; i = i + 1 print i;",
        1,
        " at 'print'",
        "Expect ')' after for clauses.",
    );
}

#[test]
fn function_errors_mention_function() {
    assert_syntax_error("fun () {}", 1, " at '('", "Expect function name.");
    assert_syntax_error("fun f {}", 1, " at '{'", "Expect '(' after function name.");
    assert_syntax_error("fun f(1) {}", 1, " at '1'", "Expect parameter name.");
    assert_syntax_error("fun f(a b) {}", 1, " at 'b'", "Expect ')' after parameters.");
    assert_syntax_error("fun f(a) print 1;", 1, " at 'print'", "Expect '{' before function body.");
}

#[test]
fn method_errors_mention_method() {
    assert_syntax_error("class A { 1() {} }", 1, " at '1'", "Expect method name.");
    assert_syntax_error("class A { m {} }", 1, " at '{'", "Expect '(' after method name.");
    assert_syntax_error("class A { m() print 1; }", 1, " at 'print'", "Expect '{' before method body.");
}

#[test]
fn class_errors() {
    assert_syntax_error("class { }", 1, " at '{'", "Expect class name.");
    assert_syntax_error("class A < { }", 1, " at '{'", "Expect superclass name.");
    assert_syntax_error("class A", 1, " at end", "Expect '{' before class body.");
    assert_syntax_error("class A {", 1, " at end", "Expect '}' after class body.");
}

#[test]
fn error_line_numbers_follow_the_source() {
    assert_syntax_error("var a = 1;\nvar b = ;", 2, " at ';'", "Expect expression.");
    assert_syntax_error(
        "print 1\nprint 2;",
        2,
        " at 'print'",
        "Expect ';' after value.",
    );
    assert_syntax_error("\n\n\nvar = 1;", 4, " at '='", "Expect variable name.");
}

#[test]
fn only_the_first_error_is_reported() {
    // Both statements are broken; the parser stops at the first one.
    assert_syntax_error("print ;\nprint ;", 1, " at ';'", "Expect expression.");
}

#[test]
fn syntax_errors_are_never_reported_as_other_variants() {
    assert!(matches!(
        parser_for("print 1").parse(),
        Err(Error::Syntax(_))
    ));
    assert!(matches!(
        parser_for("1 = 2;").parse(),
        Err(Error::Syntax(_))
    ));
}

/* ---------- 255 argument / parameter limit ---------- */

fn call_with_args(n: usize) -> String {
    format!("f({});", vec!["1"; n].join(", "))
}

fn function_with_params(n: usize) -> String {
    let params: Vec<String> = (0..n).map(|i| format!("p{i}")).collect();
    format!("fun f({}) {{}}", params.join(", "))
}

#[test]
fn up_to_255_arguments_are_accepted() {
    assert!(!program_fails(&call_with_args(0)));
    assert!(!program_fails(&call_with_args(254)));
    assert!(!program_fails(&call_with_args(255)));
}

#[test]
fn more_than_255_arguments_are_rejected_with_a_message() {
    let d = syntax_error(&call_with_args(256));
    assert_eq!(d.message, "Can't have more than 255 arguments.");
    assert_eq!(d.line, 1);
}

#[test]
fn up_to_255_parameters_are_accepted() {
    assert!(!program_fails(&function_with_params(254)));
    assert!(!program_fails(&function_with_params(255)));
}

#[test]
fn more_than_255_parameters_are_rejected_with_a_message() {
    let d = syntax_error(&function_with_params(256));
    assert_eq!(d.message, "Can't have more than 255 parameters.");
}

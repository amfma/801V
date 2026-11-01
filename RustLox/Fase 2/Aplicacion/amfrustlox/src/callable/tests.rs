//! Tests for `Native`, `Function`, `Class`, `Instance` and `Method`
//! (src/callable.rs), exercised directly without going through the
//! interpreter's `Expr::Call` path.
use super::*;
use crate::{parser::Parser, resolver::Resolver, token::Scanner};

/* ---------- helpers ---------- */

fn new_interpreter() -> Interpreter {
    Interpreter::new(Rc::new(RefCell::new(Vec::<u8>::new())))
}

/// Parses + resolves a single `fun ...` declaration and builds a `Function`
/// whose closure is a fresh global environment.
fn function(interpreter: &mut Interpreter, src: &str) -> Rc<Function> {
    let tokens = Scanner::new(src).scan_tokens().expect("scan should succeed");
    let statements = match Parser::new(tokens).parse() {
        Ok(s) => s,
        Err(_) => panic!("parse error in: {src}"),
    };
    let statements = match Resolver::new(interpreter).resolve(&statements) {
        Ok(s) => s,
        Err(_) => panic!("resolver error in: {src}"),
    };
    match &statements[0] {
        Stmt::Function { name, params, body } => Rc::new(Function::new(
            Rc::new(RefCell::new(Enviroment::new(None))),
            name,
            params,
            body,
        )),
        other => panic!("expected a function declaration, got {other:?}"),
    }
}

fn expect_ok(result: Result<Value, Error>) -> Value {
    match result {
        Ok(value) => value,
        Err(_) => panic!("call should have succeeded"),
    }
}

fn empty_class(name: &str) -> Rc<Class> {
    Class::new(name.to_string(), HashMap::new(), None)
}

fn class_with(name: &str, methods: Vec<(&str, Rc<Function>)>, superclass: Option<Rc<Class>>) -> Rc<Class> {
    let mut map = HashMap::new();
    for (method_name, function) in methods {
        map.insert(method_name.to_string(), function);
    }
    Class::new(name.to_string(), map, superclass)
}

/* =====================================================================
   Native
   ===================================================================== */

#[test]
fn native_exposes_name_arity_and_type() {
    let func: BoxedFunction = Box::new(|_| Ok(Value::Nil));
    let native = Rc::new(Native::new(func, "noop".to_string(), 2));
    assert_eq!(native.name(), "noop");
    assert_eq!(native.arity(), 2);
    assert_eq!(native.ctype(), CallableType::Function);
}

#[test]
fn native_call_forwards_arguments() {
    let mut interpreter = new_interpreter();
    let func: BoxedFunction =
        Box::new(|args: &[Value]| Ok(Value::Number(args.len() as f64)));
    let native = Rc::new(Native::new(func, "count".to_string(), 3));
    let args = vec![Value::Nil, Value::Boolean(true), Value::Number(1.0)];
    let result = expect_ok(native.call(&mut interpreter, &args));
    assert_eq!(result, Value::Number(3.0));
}

#[test]
fn native_call_can_return_an_error() {
    let mut interpreter = new_interpreter();
    let func: BoxedFunction = Box::new(|_| Err(Error::RuntimeError));
    let native = Rc::new(Native::new(func, "boom".to_string(), 0));
    assert!(native.call(&mut interpreter, &vec![]).is_err());
}

#[test]
fn native_is_not_a_class() {
    let func: BoxedFunction = Box::new(|_| Ok(Value::Nil));
    let native = Rc::new(Native::new(func, "noop".to_string(), 0));
    assert!(native.as_class().is_none());
}

/* =====================================================================
   Function
   ===================================================================== */

#[test]
fn function_exposes_name_arity_and_type() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun add(a, b) { return a + b; }");
    assert_eq!(f.name(), "add");
    assert_eq!(f.arity(), 2);
    assert_eq!(f.ctype(), CallableType::Function);
}

#[test]
fn function_with_no_parameters_has_arity_zero() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun f() {}");
    assert_eq!(f.arity(), 0);
}

#[test]
fn function_binds_arguments_to_parameters() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun add(a, b) { return a + b; }");
    let args = vec![Value::Number(1.0), Value::Number(2.0)];
    let result = expect_ok(f.call(&mut interpreter, &args));
    assert_eq!(result, Value::Number(3.0));
}

#[test]
fn function_argument_order_matters() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun sub(a, b) { return a - b; }");
    let args = vec![Value::Number(10.0), Value::Number(4.0)];
    assert_eq!(expect_ok(f.call(&mut interpreter, &args)), Value::Number(6.0));
}

#[test]
fn function_without_return_yields_nil() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun f() { var a = 1; }");
    assert_eq!(expect_ok(f.call(&mut interpreter, &vec![])), Value::Nil);
}

#[test]
fn function_bare_return_yields_nil() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun f() { return; }");
    assert_eq!(expect_ok(f.call(&mut interpreter, &vec![])), Value::Nil);
}

#[test]
fn function_returns_at_the_first_return_statement() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun f() { return 1; return 2; }");
    assert_eq!(expect_ok(f.call(&mut interpreter, &vec![])), Value::Number(1.0));
}

#[test]
fn function_propagates_runtime_errors() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun f() { return 1 / 0; }");
    assert!(f.call(&mut interpreter, &vec![]).is_err());
}

#[test]
fn function_calls_do_not_leak_parameters_between_invocations() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun id(x) { return x; }");
    let first = expect_ok(Rc::clone(&f).call(&mut interpreter, &vec![Value::Number(1.0)]));
    let second = expect_ok(Rc::clone(&f).call(&mut interpreter, &vec![Value::Number(2.0)]));
    assert_eq!(first, Value::Number(1.0));
    assert_eq!(second, Value::Number(2.0));
}

#[test]
fn function_is_not_a_class() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun f() {}");
    assert!(f.as_class().is_none());
}

#[test]
fn callable_debug_format_for_a_function() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun add(a, b) {}");
    let callable: Rc<dyn Callable> = f;
    assert_eq!(format!("{:?}", callable), "<Function add>");
}

/* =====================================================================
   Class
   ===================================================================== */

#[test]
fn class_exposes_name_and_type() {
    let class = empty_class("A");
    assert_eq!(class.name(), "A");
    assert_eq!(class.ctype(), CallableType::Class);
}

#[test]
fn class_without_init_has_arity_zero() {
    assert_eq!(empty_class("A").arity(), 0);
}

#[test]
fn class_arity_follows_init() {
    let mut interpreter = new_interpreter();
    let init = function(&mut interpreter, "fun init(a, b) {}");
    let class = class_with("A", vec![("init", init)], None);
    assert_eq!(class.arity(), 2);
}

#[test]
fn subclass_inherits_init_arity() {
    let mut interpreter = new_interpreter();
    let init = function(&mut interpreter, "fun init(a, b, c) {}");
    let parent = class_with("A", vec![("init", init)], None);
    let child = class_with("B", vec![], Some(parent));
    assert_eq!(child.arity(), 3);
}

#[test]
fn subclass_init_overrides_parent_arity() {
    let mut interpreter = new_interpreter();
    let parent_init = function(&mut interpreter, "fun init(a, b, c) {}");
    let child_init = function(&mut interpreter, "fun init(a) {}");
    let parent = class_with("A", vec![("init", parent_init)], None);
    let child = class_with("B", vec![("init", child_init)], Some(parent));
    assert_eq!(child.arity(), 1);
}

#[test]
fn calling_a_class_creates_an_instance_of_it() {
    let mut interpreter = new_interpreter();
    let class = empty_class("A");
    match expect_ok(Rc::clone(&class).call(&mut interpreter, &vec![])) {
        Value::Instance(instance) => {
            assert!(Rc::ptr_eq(&instance.borrow().class, &class));
        }
        other => panic!("expected an instance, got {other:?}"),
    }
}

#[test]
fn calling_a_class_runs_its_initializer() {
    let mut interpreter = new_interpreter();
    let init = function(&mut interpreter, "fun init(a) { }");
    let class = class_with("A", vec![("init", init)], None);
    let result = Rc::clone(&class).call(&mut interpreter, &vec![Value::Number(1.0)]);
    assert!(matches!(result, Ok(Value::Instance(_))));
}

#[test]
fn class_initializer_errors_propagate() {
    let mut interpreter = new_interpreter();
    let init = function(&mut interpreter, "fun init() { return 1 / 0; }");
    let class = class_with("A", vec![("init", init)], None);
    assert!(Rc::clone(&class).call(&mut interpreter, &vec![]).is_err());
}

#[test]
fn each_class_call_creates_a_new_instance() {
    let mut interpreter = new_interpreter();
    let class = empty_class("A");
    let a = expect_ok(Rc::clone(&class).call(&mut interpreter, &vec![]));
    let b = expect_ok(Rc::clone(&class).call(&mut interpreter, &vec![]));
    assert_ne!(a, b);
}

#[test]
fn class_as_class_returns_itself() {
    let class = empty_class("A");
    let same = Rc::clone(&class).as_class().expect("a class is a class");
    assert!(Rc::ptr_eq(&same, &class));
}

#[test]
fn callable_debug_format_for_a_class() {
    let callable: Rc<dyn Callable> = empty_class("A");
    assert_eq!(format!("{:?}", callable), "<Class A>");
}

#[test]
fn find_method_returns_own_methods() {
    let mut interpreter = new_interpreter();
    let m = function(&mut interpreter, "fun m() {}");
    let class = class_with("A", vec![("m", Rc::clone(&m))], None);
    let found = class.find_method("m".to_string()).expect("method exists");
    assert!(Rc::ptr_eq(&found, &m));
}

#[test]
fn find_method_returns_none_for_unknown_names() {
    let class = empty_class("A");
    assert!(class.find_method("nope".to_string()).is_none());
}

#[test]
fn find_method_walks_up_the_superclass_chain() {
    let mut interpreter = new_interpreter();
    let m = function(&mut interpreter, "fun m() {}");
    let a = class_with("A", vec![("m", Rc::clone(&m))], None);
    let b = class_with("B", vec![], Some(a));
    let c = class_with("C", vec![], Some(b));
    let found = c.find_method("m".to_string()).expect("inherited method");
    assert!(Rc::ptr_eq(&found, &m));
}

#[test]
fn find_method_prefers_the_subclass_override() {
    let mut interpreter = new_interpreter();
    let parent_m = function(&mut interpreter, "fun m() {}");
    let child_m = function(&mut interpreter, "fun m() {}");
    let parent = class_with("A", vec![("m", Rc::clone(&parent_m))], None);
    let child = class_with("B", vec![("m", Rc::clone(&child_m))], Some(parent));
    let found = child.find_method("m".to_string()).unwrap();
    assert!(Rc::ptr_eq(&found, &child_m));
    assert!(!Rc::ptr_eq(&found, &parent_m));
}

/* =====================================================================
   Instance
   ===================================================================== */

#[test]
fn new_instance_has_no_fields() {
    let instance = Instance::new(empty_class("A"));
    assert!(Instance::get(&instance, "x".to_string()).is_none());
}

#[test]
fn instance_set_returns_the_value_and_stores_it() {
    let instance = Instance::new(empty_class("A"));
    let returned = instance.borrow_mut().set("x".to_string(), Value::Number(1.0));
    assert_eq!(returned, Value::Number(1.0));
    assert_eq!(
        Instance::get(&instance, "x".to_string()),
        Some(Value::Number(1.0))
    );
}

#[test]
fn instance_set_overwrites_existing_fields() {
    let instance = Instance::new(empty_class("A"));
    instance.borrow_mut().set("x".to_string(), Value::Number(1.0));
    instance.borrow_mut().set("x".to_string(), Value::Number(2.0));
    assert_eq!(
        Instance::get(&instance, "x".to_string()),
        Some(Value::Number(2.0))
    );
}

#[test]
fn instance_fields_are_independent_between_instances() {
    let class = empty_class("A");
    let a = Instance::new(Rc::clone(&class));
    let b = Instance::new(Rc::clone(&class));
    a.borrow_mut().set("x".to_string(), Value::Number(1.0));
    assert!(Instance::get(&b, "x".to_string()).is_none());
}

#[test]
fn instance_get_returns_a_bound_method() {
    let mut interpreter = new_interpreter();
    let m = function(&mut interpreter, "fun m(a) { return a; }");
    let class = class_with("A", vec![("m", m)], None);
    let instance = Instance::new(class);
    match Instance::get(&instance, "m".to_string()) {
        Some(Value::Callable(callable)) => {
            assert_eq!(callable.name(), "m");
            assert_eq!(callable.arity(), 1);
        }
        other => panic!("expected a bound method, got {other:?}"),
    }
}

#[test]
fn instance_get_finds_inherited_methods() {
    let mut interpreter = new_interpreter();
    let m = function(&mut interpreter, "fun m() {}");
    let parent = class_with("A", vec![("m", m)], None);
    let child = class_with("B", vec![], Some(parent));
    let instance = Instance::new(child);
    assert!(matches!(
        Instance::get(&instance, "m".to_string()),
        Some(Value::Callable(_))
    ));
}

#[test]
fn instance_fields_shadow_methods() {
    let mut interpreter = new_interpreter();
    let m = function(&mut interpreter, "fun m() {}");
    let class = class_with("A", vec![("m", m)], None);
    let instance = Instance::new(class);
    instance.borrow_mut().set("m".to_string(), Value::Number(5.0));
    assert_eq!(
        Instance::get(&instance, "m".to_string()),
        Some(Value::Number(5.0))
    );
}

#[test]
fn instance_display_format() {
    let instance = Instance::new(empty_class("Point"));
    assert_eq!(format!("{}", instance.borrow()), "Instance Point.class");
}

/* =====================================================================
   Method
   ===================================================================== */

#[test]
fn bind_defines_this_in_a_fresh_closure() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun m() {}");
    let instance = Instance::new(empty_class("A"));
    let method = Method::bind(f, Rc::clone(&instance), false);

    let this = method.closure.borrow().get_at(0, "this".to_string());
    match this {
        Some(Value::Instance(found)) => assert!(Rc::ptr_eq(&found, &instance)),
        other => panic!("`this` should be the bound instance, got {other:?}"),
    }
}

#[test]
fn bind_encloses_the_functions_original_closure() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun m() {}");
    f.closure
        .borrow_mut()
        .define("captured".to_string(), Value::Number(7.0));
    let method = Method::bind(Rc::clone(&f), Instance::new(empty_class("A")), false);

    assert_eq!(
        method.closure.borrow().get("captured".to_string()),
        Some(Value::Number(7.0))
    );
    // ...but `this` lives only in the method's own scope, not in the original closure.
    assert!(f.closure.borrow().get("this".to_string()).is_none());
}

#[test]
fn method_delegates_name_and_arity_to_the_function() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun m(a, b) {}");
    let method = Rc::new(Method::bind(f, Instance::new(empty_class("A")), false));
    assert_eq!(method.name(), "m");
    assert_eq!(method.arity(), 2);
    assert_eq!(method.ctype(), CallableType::Function);
    assert!(Rc::clone(&method).as_class().is_none());
}

#[test]
fn method_call_returns_the_function_result() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun m(a) { return a + 1; }");
    let method = Rc::new(Method::bind(f, Instance::new(empty_class("A")), false));
    let result = expect_ok(method.call(&mut interpreter, &vec![Value::Number(1.0)]));
    assert_eq!(result, Value::Number(2.0));
}

#[test]
fn non_initializer_method_without_return_yields_nil() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun m() {}");
    let method = Rc::new(Method::bind(f, Instance::new(empty_class("A")), false));
    assert_eq!(expect_ok(method.call(&mut interpreter, &vec![])), Value::Nil);
}

#[test]
fn initializer_method_returns_the_instance() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun init() {}");
    let instance = Instance::new(empty_class("A"));
    let method = Rc::new(Method::bind(f, Rc::clone(&instance), true));
    match expect_ok(method.call(&mut interpreter, &vec![])) {
        Value::Instance(returned) => assert!(Rc::ptr_eq(&returned, &instance)),
        other => panic!("init should return the instance, got {other:?}"),
    }
}

#[test]
fn method_errors_propagate() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun m() { return 1 / 0; }");
    let method = Rc::new(Method::bind(f, Instance::new(empty_class("A")), false));
    assert!(method.call(&mut interpreter, &vec![]).is_err());
}

#[test]
fn initializer_errors_propagate_instead_of_returning_this() {
    let mut interpreter = new_interpreter();
    let f = function(&mut interpreter, "fun init() { return 1 / 0; }");
    let method = Rc::new(Method::bind(f, Instance::new(empty_class("A")), true));
    assert!(method.call(&mut interpreter, &vec![]).is_err());
}

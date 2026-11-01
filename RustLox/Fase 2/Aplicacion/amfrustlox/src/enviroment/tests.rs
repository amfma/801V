//! Tests for `Enviroment` (src/enviroment.rs).
use super::*;

/* ---------- helpers ---------- */

type Env = Rc<RefCell<Enviroment>>;

fn global() -> Env {
    Rc::new(RefCell::new(Enviroment::new(None)))
}

fn child_of(parent: &Env) -> Env {
    Rc::new(RefCell::new(Enviroment::new(Some(Rc::clone(parent)))))
}

fn num(n: f64) -> Value {
    Value::Number(n)
}

fn key(name: &str) -> String {
    name.to_string()
}

fn define(env: &Env, name: &str, value: Value) {
    env.borrow_mut().define(key(name), value);
}

fn get(env: &Env, name: &str) -> Option<Value> {
    env.borrow().get(key(name))
}

fn get_at(env: &Env, distance: usize, name: &str) -> Option<Value> {
    env.borrow().get_at(distance, key(name))
}

/* =====================================================================
   new / define / get
   ===================================================================== */

#[test]
fn new_environment_is_empty() {
    let env = global();
    assert_eq!(get(&env, "a"), None);
}

#[test]
fn define_then_get() {
    let env = global();
    define(&env, "a", num(1.0));
    assert_eq!(get(&env, "a"), Some(num(1.0)));
}

#[test]
fn define_overwrites_an_existing_binding() {
    let env = global();
    define(&env, "a", num(1.0));
    define(&env, "a", num(2.0));
    assert_eq!(get(&env, "a"), Some(num(2.0)));
}

#[test]
fn define_can_change_the_type_of_a_binding() {
    let env = global();
    define(&env, "a", num(1.0));
    define(&env, "a", Value::String("now a string".to_string()));
    assert_eq!(get(&env, "a"), Some(Value::String("now a string".to_string())));
}

#[test]
fn nil_value_is_distinct_from_an_undefined_name() {
    let env = global();
    define(&env, "a", Value::Nil);
    assert_eq!(get(&env, "a"), Some(Value::Nil));
    assert_eq!(get(&env, "b"), None);
}

#[test]
fn names_are_case_sensitive() {
    let env = global();
    define(&env, "a", num(1.0));
    assert_eq!(get(&env, "A"), None);
}

#[test]
fn multiple_bindings_are_independent() {
    let env = global();
    define(&env, "a", num(1.0));
    define(&env, "b", num(2.0));
    assert_eq!(get(&env, "a"), Some(num(1.0)));
    assert_eq!(get(&env, "b"), Some(num(2.0)));
}

/* =====================================================================
   get through enclosing scopes
   ===================================================================== */

#[test]
fn get_finds_values_in_the_enclosing_scope() {
    let parent = global();
    define(&parent, "a", num(1.0));
    let child = child_of(&parent);
    assert_eq!(get(&child, "a"), Some(num(1.0)));
}

#[test]
fn get_walks_multiple_levels() {
    let a = global();
    define(&a, "x", num(1.0));
    let b = child_of(&a);
    let c = child_of(&b);
    let d = child_of(&c);
    assert_eq!(get(&d, "x"), Some(num(1.0)));
}

#[test]
fn inner_binding_shadows_the_outer_one() {
    let parent = global();
    define(&parent, "a", num(1.0));
    let child = child_of(&parent);
    define(&child, "a", num(2.0));
    assert_eq!(get(&child, "a"), Some(num(2.0)));
    assert_eq!(get(&parent, "a"), Some(num(1.0)));
}

#[test]
fn nearest_enclosing_binding_wins() {
    let a = global();
    define(&a, "x", num(1.0));
    let b = child_of(&a);
    define(&b, "x", num(2.0));
    let c = child_of(&b);
    assert_eq!(get(&c, "x"), Some(num(2.0)));
}

#[test]
fn get_does_not_look_into_child_scopes() {
    let parent = global();
    let child = child_of(&parent);
    define(&child, "a", num(1.0));
    assert_eq!(get(&parent, "a"), None);
}

#[test]
fn get_of_an_unknown_name_returns_none_through_the_whole_chain() {
    let a = global();
    let b = child_of(&a);
    let c = child_of(&b);
    assert_eq!(get(&c, "missing"), None);
}

#[test]
fn child_sees_later_definitions_in_its_parent() {
    let parent = global();
    let child = child_of(&parent);
    assert_eq!(get(&child, "a"), None);
    define(&parent, "a", num(1.0));
    assert_eq!(get(&child, "a"), Some(num(1.0)));
}

#[test]
fn sibling_scopes_are_independent() {
    let parent = global();
    let left = child_of(&parent);
    let right = child_of(&parent);
    define(&left, "a", num(1.0));
    assert_eq!(get(&right, "a"), None);
}

/* =====================================================================
   get_at
   ===================================================================== */

#[test]
fn get_at_zero_reads_the_current_scope() {
    let env = global();
    define(&env, "a", num(1.0));
    assert_eq!(get_at(&env, 0, "a"), Some(num(1.0)));
}

#[test]
fn get_at_zero_does_not_fall_back_to_the_parent() {
    let parent = global();
    define(&parent, "a", num(1.0));
    let child = child_of(&parent);
    assert_eq!(get_at(&child, 0, "a"), None);
}

#[test]
fn get_at_one_reads_exactly_the_parent_scope() {
    let parent = global();
    define(&parent, "a", num(1.0));
    let child = child_of(&parent);
    assert_eq!(get_at(&child, 1, "a"), Some(num(1.0)));
}

#[test]
fn get_at_one_does_not_see_the_current_scope() {
    let parent = global();
    let child = child_of(&parent);
    define(&child, "a", num(1.0));
    assert_eq!(get_at(&child, 1, "a"), None);
}

#[test]
fn get_at_skips_shadowing_bindings() {
    let parent = global();
    define(&parent, "a", num(1.0));
    let child = child_of(&parent);
    define(&child, "a", num(2.0));
    assert_eq!(get_at(&child, 0, "a"), Some(num(2.0)));
    assert_eq!(get_at(&child, 1, "a"), Some(num(1.0)));
}

#[test]
fn get_at_walks_the_requested_number_of_hops() {
    let a = global();
    define(&a, "x", num(10.0));
    let b = child_of(&a);
    define(&b, "x", num(20.0));
    let c = child_of(&b);
    define(&c, "x", num(30.0));
    let d = child_of(&c);

    assert_eq!(get_at(&d, 0, "x"), None);
    assert_eq!(get_at(&d, 1, "x"), Some(num(30.0)));
    assert_eq!(get_at(&d, 2, "x"), Some(num(20.0)));
    assert_eq!(get_at(&d, 3, "x"), Some(num(10.0)));
}

#[test]
fn get_at_beyond_the_outermost_scope_returns_none() {
    let parent = global();
    define(&parent, "a", num(1.0));
    let child = child_of(&parent);
    assert_eq!(get_at(&child, 2, "a"), None);
    assert_eq!(get_at(&child, 100, "a"), None);
}

#[test]
fn get_at_of_an_unknown_name_returns_none() {
    let env = global();
    assert_eq!(get_at(&env, 0, "nope"), None);
}

/* =====================================================================
   assign
   ===================================================================== */

#[test]
fn assign_updates_an_existing_binding() {
    let env = global();
    define(&env, "a", num(1.0));
    assert!(env.borrow_mut().assign(key("a"), num(2.0)).is_ok());
    assert_eq!(get(&env, "a"), Some(num(2.0)));
}

#[test]
fn assign_to_an_undefined_name_is_an_error() {
    let env = global();
    let result = env.borrow_mut().assign(key("a"), num(1.0));
    assert!(matches!(result, Err(Error::UndefinedVariable)));
}

#[test]
fn failed_assign_does_not_create_the_binding() {
    let env = global();
    let _ = env.borrow_mut().assign(key("a"), num(1.0));
    assert_eq!(get(&env, "a"), None);
}

#[test]
fn assign_updates_the_enclosing_scope_when_not_defined_locally() {
    let parent = global();
    define(&parent, "a", num(1.0));
    let child = child_of(&parent);
    assert!(child.borrow_mut().assign(key("a"), num(2.0)).is_ok());
    assert_eq!(get(&parent, "a"), Some(num(2.0)));
    assert_eq!(get_at(&child, 0, "a"), None, "no local copy may be created");
}

#[test]
fn assign_prefers_the_nearest_binding() {
    let parent = global();
    define(&parent, "a", num(1.0));
    let child = child_of(&parent);
    define(&child, "a", num(2.0));
    assert!(child.borrow_mut().assign(key("a"), num(3.0)).is_ok());
    assert_eq!(get(&child, "a"), Some(num(3.0)));
    assert_eq!(get(&parent, "a"), Some(num(1.0)));
}

#[test]
fn assign_reaches_through_several_levels() {
    let a = global();
    define(&a, "x", num(1.0));
    let b = child_of(&a);
    let c = child_of(&b);
    assert!(c.borrow_mut().assign(key("x"), num(9.0)).is_ok());
    assert_eq!(get(&a, "x"), Some(num(9.0)));
}

#[test]
fn assign_of_unknown_name_fails_through_the_whole_chain() {
    let a = global();
    let b = child_of(&a);
    let c = child_of(&b);
    assert!(matches!(
        c.borrow_mut().assign(key("x"), num(1.0)),
        Err(Error::UndefinedVariable)
    ));
}

#[test]
fn assign_can_change_the_type_of_a_binding() {
    let env = global();
    define(&env, "a", num(1.0));
    assert!(env.borrow_mut().assign(key("a"), Value::Nil).is_ok());
    assert_eq!(get(&env, "a"), Some(Value::Nil));
}

/* =====================================================================
   assign_at
   ===================================================================== */

#[test]
fn assign_at_zero_updates_the_current_scope() {
    let env = global();
    define(&env, "a", num(1.0));
    env.borrow_mut().assign_at(key("a"), 0, num(2.0));
    assert_eq!(get(&env, "a"), Some(num(2.0)));
}

#[test]
fn assign_at_zero_does_not_define_a_missing_name() {
    let env = global();
    env.borrow_mut().assign_at(key("a"), 0, num(2.0));
    assert_eq!(get(&env, "a"), None);
}

#[test]
fn assign_at_one_updates_only_the_parent() {
    let parent = global();
    define(&parent, "a", num(1.0));
    let child = child_of(&parent);
    define(&child, "a", num(2.0));

    child.borrow_mut().assign_at(key("a"), 1, num(99.0));

    assert_eq!(get_at(&child, 0, "a"), Some(num(2.0)));
    assert_eq!(get_at(&child, 1, "a"), Some(num(99.0)));
}

#[test]
fn assign_at_walks_the_requested_number_of_hops() {
    let a = global();
    define(&a, "x", num(10.0));
    let b = child_of(&a);
    define(&b, "x", num(20.0));
    let c = child_of(&b);

    c.borrow_mut().assign_at(key("x"), 2, num(11.0));
    assert_eq!(get_at(&c, 2, "x"), Some(num(11.0)));
    assert_eq!(get_at(&c, 1, "x"), Some(num(20.0)));

    c.borrow_mut().assign_at(key("x"), 1, num(21.0));
    assert_eq!(get_at(&c, 1, "x"), Some(num(21.0)));
    assert_eq!(get_at(&c, 2, "x"), Some(num(11.0)));
}

#[test]
fn assign_at_does_not_fall_back_to_other_scopes() {
    // `a` lives in the parent, but we ask for distance 0: nothing may change.
    let parent = global();
    define(&parent, "a", num(1.0));
    let child = child_of(&parent);
    child.borrow_mut().assign_at(key("a"), 0, num(2.0));
    assert_eq!(get(&parent, "a"), Some(num(1.0)));
    assert_eq!(get_at(&child, 0, "a"), None);
}

#[test]
fn assign_at_beyond_the_outermost_scope_is_a_silent_noop() {
    let parent = global();
    define(&parent, "a", num(1.0));
    let child = child_of(&parent);
    child.borrow_mut().assign_at(key("a"), 5, num(2.0));
    assert_eq!(get(&parent, "a"), Some(num(1.0)));
}

#[test]
fn assign_at_of_unknown_name_at_the_target_scope_is_a_noop() {
    let parent = global();
    let child = child_of(&parent);
    child.borrow_mut().assign_at(key("ghost"), 1, num(2.0));
    assert_eq!(get(&parent, "ghost"), None);
    assert_eq!(get(&child, "ghost"), None);
}

/* =====================================================================
   Clone & sharing semantics
   ===================================================================== */

#[test]
fn clone_copies_local_values() {
    let mut original = Enviroment::new(None);
    original.define(key("a"), num(1.0));
    let mut copy = original.clone();

    copy.define(key("a"), num(2.0));
    copy.define(key("b"), num(3.0));

    assert_eq!(original.get(key("a")), Some(num(1.0)));
    assert_eq!(original.get(key("b")), None);
    assert_eq!(copy.get(key("a")), Some(num(2.0)));
}

#[test]
fn clone_shares_the_enclosing_scope() {
    let parent = global();
    define(&parent, "a", num(1.0));
    let original = Enviroment::new(Some(Rc::clone(&parent)));
    let copy = original.clone();

    parent.borrow_mut().define(key("a"), num(2.0));

    assert_eq!(original.get(key("a")), Some(num(2.0)));
    assert_eq!(copy.get(key("a")), Some(num(2.0)));
}

#[test]
fn closures_keep_their_defining_scope_alive() {
    // Mimics a closure: the child outlives every other handle on the parent.
    let child = {
        let parent = global();
        define(&parent, "captured", num(42.0));
        child_of(&parent)
    };
    assert_eq!(get(&child, "captured"), Some(num(42.0)));
}

#[test]
fn two_children_share_updates_made_through_a_common_parent() {
    let parent = global();
    define(&parent, "counter", num(0.0));
    let first = child_of(&parent);
    let second = child_of(&parent);

    assert!(first.borrow_mut().assign(key("counter"), num(1.0)).is_ok());
    assert_eq!(get(&second, "counter"), Some(num(1.0)));
}

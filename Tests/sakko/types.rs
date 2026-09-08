//! Unit coverage for the `Ty` lattice in isolation.

use sakko::typecheck::Ty;

#[test]
fn union_dedups_and_flattens() {
    assert_eq!(
        Ty::union(Ty::Number, Ty::union(Ty::Null, Ty::Number)),
        Ty::Union(vec![Ty::Number, Ty::Null])
    );
}

#[test]
fn null_assigns_nowhere_but_itself_and_any() {
    assert!(Ty::Null.assigns_to(&Ty::Null));
    assert!(!Ty::Null.assigns_to(&Ty::Str));
    assert!(!Ty::Number.assigns_to(&Ty::Union(vec![Ty::Str, Ty::Bool])));
    assert!(Ty::Str.assigns_to(&Ty::Union(vec![Ty::Str, Ty::Null])));
    assert!(!Ty::Union(vec![Ty::Str, Ty::Null]).assigns_to(&Ty::Str));
    assert!(!Ty::Union(vec![Ty::Str, Ty::Null]).assigns_to(&Ty::Number));
}

#[test]
fn arrays_are_invariant_except_widening() {
    assert!(Ty::array_of(Ty::Number).assigns_to(&Ty::Array(None)));
    assert!(!Ty::Array(None).assigns_to(&Ty::array_of(Ty::Number)));
    assert!(Ty::array_of(Ty::Number).assigns_to(&Ty::array_of(Ty::Number)));
    assert!(!Ty::array_of(Ty::Str).assigns_to(&Ty::array_of(Ty::Number)));
}

#[test]
fn without_nullish_strips_only_null_and_undefined() {
    let t = Ty::Union(vec![Ty::Str, Ty::Null]);
    assert_eq!(t.without_nullish(), Some(Ty::Str));
    assert_eq!(Ty::Null.without_nullish(), None);
    assert_eq!(Ty::Number.without_nullish(), Some(Ty::Number));
}

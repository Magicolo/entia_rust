use super::*;
use crate::generator::Constant;

#[test]
fn has_sample() {
    char::generator().sample(1).next().unwrap();
}

#[test]
fn sample_has_count() {
    for i in 0..COUNT {
        assert_eq!(char::generator().sample(i).len(), i);
    }
}

#[test]
#[should_panic]
fn empty_range() {
    let value = char::generator().sample(1).next().unwrap();
    (value..value).sample(1).next().unwrap();
}

#[test]
fn is_constant() {
    for value in char::generator().sample(COUNT) {
        assert_eq!(Constant(value).sample(1).next().unwrap(), value);
    }
}

#[test]
fn is_ascii() {
    assert!(ascii().sample(COUNT).all(|value| value.is_ascii()))
}

#[test]
fn is_digit() {
    assert!(digit().sample(COUNT).all(|value| value.is_ascii_digit()))
}

#[test]
fn is_alphabetic() {
    assert!(alphabet()
        .sample(COUNT)
        .all(|value| value.is_ascii_alphabetic()))
}

#[test]
fn full_does_not_panic() {
    <char>::generator().sample(COUNT).for_each(|_| {});
}

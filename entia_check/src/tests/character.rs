use super::*;

#[test]
fn has_sample() {
    char::generator().sample(1).next().unwrap();
}

#[test]
#[should_panic]
fn empty_range() {
    let value = char::generator().sample(1).next().unwrap();
    (value..value).sample(1).next().unwrap();
}

#[test]
fn is_ascii() {
    assert!(ascii().sample(1000).all(|value| value.is_ascii()))
}

#[test]
fn is_digit() {
    assert!(digit().sample(1000).all(|value| value.is_ascii_digit()))
}

#[test]
fn is_alphabetic() {
    assert!(alphabet()
        .sample(100)
        .all(|value| value.is_ascii_alphabetic()))
}

#[test]
fn full_does_not_panic() {
    <char>::generator().sample(1000).for_each(|_| {});
}

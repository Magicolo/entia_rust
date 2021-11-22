use super::*;

#[test]
fn is_ascii() {
    assert!(string(ascii()).sample(COUNT).all(|value| value.is_ascii()))
}

#[test]
fn is_digit() {
    assert!(string(digit())
        .sample(COUNT)
        .all(|value| value.chars().all(|value| value.is_ascii_digit())))
}

#[test]
fn is_alphabetic() {
    assert!(string(alphabet())
        .sample(COUNT)
        .all(|value| value.chars().all(|value| value.is_ascii_alphabetic())))
}

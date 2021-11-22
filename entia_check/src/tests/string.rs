use super::*;

#[test]
fn is_ascii() {
    assert!(string(ascii()).sample(1000).all(|value| value.is_ascii()))
}

#[test]
fn is_digit() {
    assert!(string(digit())
        .sample(1000)
        .all(|value| value.chars().all(|value| value.is_ascii_digit())))
}

#[test]
fn is_alphabetic() {
    assert!(string(alphabet())
        .sample(1000)
        .all(|value| value.chars().all(|value| value.is_ascii_alphabetic())))
}

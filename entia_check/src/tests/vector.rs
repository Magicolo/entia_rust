use super::*;

#[test]
fn is_ascii() {
    assert!(vector(ascii())
        .sample(COUNT)
        .all(|value| value.iter().all(|value| value.is_ascii())))
}

#[test]
fn is_digit() {
    assert!(vector(digit())
        .sample(COUNT)
        .all(|value| value.iter().all(|value| value.is_ascii_digit())))
}

#[test]
fn is_alphabetic() {
    assert!(vector(alphabet())
        .sample(COUNT)
        .all(|value| value.iter().all(|value| value.is_ascii_alphabetic())))
}

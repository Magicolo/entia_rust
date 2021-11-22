use super::*;

#[test]
fn is_ascii() {
    assert!(vector(ascii())
        .sample(1000)
        .all(|value| value.iter().all(|value| value.is_ascii())))
}

#[test]
fn is_digit() {
    assert!(vector(digit())
        .sample(1000)
        .all(|value| value.iter().all(|value| value.is_ascii_digit())))
}

#[test]
fn is_alphabetic() {
    assert!(vector(alphabet())
        .sample(1000)
        .all(|value| value.iter().all(|value| value.is_ascii_alphabetic())))
}

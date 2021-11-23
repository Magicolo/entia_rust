use super::*;
use crate::generator::Constant;

#[test]
fn has_constant_count() {
    for i in 0..COUNT {
        assert!(
            vector_with(u8::generator(), Constant(i))
                .sample(1)
                .next()
                .unwrap()
                .len()
                == i
        )
    }
}

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

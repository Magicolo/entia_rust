pub mod character;
pub mod floating;
pub mod integer;
pub mod string;
pub mod vector;

use super::*;

#[test]
fn sample_has_count() {
    for i in 0..100 {
        assert_eq!(<u8>::generator().sample(i).len(), i);
    }
}

#[test]
fn boolean_samples_true_and_false() {
    assert!(<bool>::generator().sample(100).any(|value| value));
    assert!(<bool>::generator().sample(100).any(|value| !value));
}

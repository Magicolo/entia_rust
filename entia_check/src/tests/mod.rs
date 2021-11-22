pub mod character;
pub mod floating;
pub mod integer;
pub mod string;
pub mod vector;

use super::*;

pub const COUNT: usize = 1000;

#[test]
fn boolean_samples_true_and_false() {
    assert!(<bool>::generator().sample(COUNT).any(|value| value));
    assert!(<bool>::generator().sample(COUNT).any(|value| !value));
}

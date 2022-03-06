pub mod character;
pub mod number;

use super::*;
use spectral::{boolean::*, numeric::*, *};

pub const COUNT: usize = 1024;

#[test]
fn boolean_samples_true_and_false() {
    assert_that(&<bool>::generator().sample(COUNT).any(|value| value)).is_true();
    assert_that(&<bool>::generator().sample(COUNT).any(|value| !value)).is_true();
}

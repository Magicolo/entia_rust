pub mod character;
pub mod number;

use super::*;
use spectral::{boolean::*, numeric::*, result::*, *};

pub const COUNT: usize = 256;

#[test]
fn boolean_samples_true_and_false() {
    assert_that(&<bool>::generator().sample(COUNT).any(|value| value)).is_true();
    assert_that(&<bool>::generator().sample(COUNT).any(|value| !value)).is_true();
}

pub mod all;
pub mod any;
pub mod generator;
pub mod primitive;

use self::{
    any::Any,
    generator::{Count, With},
};
pub use generator::{Generator, IntoGenerator};

pub fn alphabet() -> impl Generator<Item = char> {
    Any(('a'..='z', 'A'..='Z'))
}

pub fn digit() -> impl Generator<Item = char> {
    '0'..='9'
}

pub fn ascii() -> impl Generator<Item = char> {
    Any((
        alphabet(),
        digit(),
        Generator::map(0..=0x7Fu8, |value| value as char),
    ))
}

pub fn string(mut item: impl Generator<Item = char>) -> impl Generator<Item = String> {
    With::new(move |state| {
        Iterator::map(0..Count.generate(state), |_| item.generate(state)).collect()
    })
}

pub fn vector<G: Generator>(mut item: G) -> impl Generator<Item = Vec<G::Item>> {
    With::new(move |state| {
        Iterator::map(0..Count.generate(state), |_| item.generate(state)).collect()
    })
}

#[cfg(test)]
mod test {
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

    #[test]
    fn character_is_ascii() {
        assert!(ascii().sample(100).all(|value| value.is_ascii()))
    }

    #[test]
    fn character_is_digit() {
        assert!(digit().sample(100).all(|value| value.is_ascii_digit()))
    }

    #[test]
    fn character_is_alphabetic() {
        assert!(alphabet()
            .sample(100)
            .all(|value| value.is_ascii_alphabetic()))
    }

    #[test]
    fn string_is_ascii() {
        assert!(string(ascii()).sample(100).all(|value| value.is_ascii()))
    }

    #[test]
    fn string_is_digit() {
        assert!(string(digit())
            .sample(100)
            .all(|value| value.chars().all(|value| value.is_ascii_digit())))
    }

    #[test]
    fn string_is_alphabetic() {
        assert!(string(alphabet())
            .sample(100)
            .all(|value| value.chars().all(|value| value.is_ascii_alphabetic())))
    }

    #[test]
    fn vector_is_ascii() {
        assert!(vector(ascii())
            .sample(100)
            .all(|value| value.iter().all(|value| value.is_ascii())))
    }

    #[test]
    fn vector_is_digit() {
        assert!(vector(digit())
            .sample(100)
            .all(|value| value.iter().all(|value| value.is_ascii_digit())))
    }

    #[test]
    fn vector_is_alphabetic() {
        assert!(vector(alphabet())
            .sample(100)
            .all(|value| value.iter().all(|value| value.is_ascii_alphabetic())))
    }

    mod range {
        use super::*;

        macro_rules! integer_range {
            ($t:ident) => {
                mod $t {
                    use super::*;

                    #[test]
                    fn is_in_range() {
                        for pair in <($t, $t)>::generator().sample(100) {
                            let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                            if low == high { continue; }
                            assert!((low..high).sample(100).all(|value| value >= low && value < high));
                        }
                    }

                    #[test]
                    fn is_in_range_inclusive() {
                        for pair in <($t, $t)>::generator().sample(100) {
                            let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                            assert!((low..=high).sample(100).all(|value| value >= low && value <= high));
                        }
                    }

                    #[test]
                    fn is_in_range_from() {
                        for low in <$t>::generator().sample(100) {
                            assert!((low..).sample(100).all(|value| value >= low));
                        }
                    }

                    #[test]
                    fn is_in_range_to() {
                        for high in <$t>::generator().sample(100) {
                            if $t::MIN == high { continue; }
                            assert!((..high).sample(100).all(|value| value < high));
                        }
                    }

                    #[test]
                    fn is_in_range_to_inclusive() {
                        for high in <$t>::generator().sample(100) {
                            assert!((..=high).sample(100).all(|value| value <= high));
                        }
                    }
                }
            };
            ($($t:ident),*) => { $(integer_range!($t);)* };
        }

        macro_rules! floating_range {
            ($t:ident) => {
                mod $t {
                    use super::*;

                    #[test]
                    fn is_in_range() {
                        for pair in <($t, $t)>::generator().sample(100) {
                            let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                            if low == high { continue; }
                            assert!((low..high).sample(100).all(|value| value >= low && value < high));
                        }
                    }

                    #[test]
                    fn is_in_range_from() {
                        for low in <$t>::generator().sample(100) {
                            assert!((low..).sample(100).all(|value| value >= low));
                        }
                    }

                    #[test]
                    fn is_in_range_to() {
                        for high in <$t>::generator().sample(100) {
                            assert!((..high).sample(100).all(|value| value < high || $t::MIN == high));
                        }
                    }
                }
            };
            ($($t:ident),*) => { $(floating_range!($t);)* };
        }

        integer_range!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
        floating_range!(f32, f64);
    }
}

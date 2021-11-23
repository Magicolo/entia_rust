use super::*;
use generator::Constant;

mod range {
    use super::*;

    macro_rules! tests {
        ($t:ident) => {
            mod $t {
                use super::*;

                #[test]
                fn has_sample() {
                    <$t>::generator().sample(1).next().unwrap();
                }

                #[test]
                fn sample_has_count() {
                    for i in 0..COUNT {
                        assert_eq!(<$t>::generator().sample(i).len(), i);
                    }
                }

                #[test]
                #[should_panic]
                fn empty_range() {
                    let value = <$t>::generator().sample(1).next().unwrap();
                    (value..value).sample(1).next().unwrap();
                }

                #[test]
                fn is_constant() {
                    for value in $t::generator().sample(COUNT) {
                        assert_eq!(Constant(value).sample(1).next().unwrap(), value);
                    }
                }

                #[test]
                fn is_in_range() {
                    for pair in <($t, $t)>::generator().sample(COUNT) {
                        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                        if low == high { continue; }
                        assert!((low..high).sample(COUNT).all(|value| value >= low && value < high));
                    }
                }

                #[test]
                fn is_in_range_inclusive() {
                    for pair in <($t, $t)>::generator().sample(COUNT) {
                        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                        assert!((low..=high).sample(COUNT).all(|value| value >= low && value <= high));
                    }
                }

                #[test]
                fn is_in_range_from() {
                    for low in <$t>::generator().sample(COUNT) {
                        assert!((low..).sample(COUNT).all(|value| value >= low));
                    }
                }

                #[test]
                fn is_in_range_to() {
                    for high in <$t>::generator().sample(COUNT) {
                        if $t::MIN == high { continue; }
                        assert!((..high).sample(COUNT).all(|value| value < high));
                    }
                }

                #[test]
                fn is_in_range_to_inclusive() {
                    for high in <$t>::generator().sample(COUNT) {
                        assert!((..=high).sample(COUNT).all(|value| value <= high));
                    }
                }
            }
        };
        ($($t:ident),*) => { $(tests!($t);)* };
    }

    tests!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
}

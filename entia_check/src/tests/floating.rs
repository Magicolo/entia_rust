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
                fn is_constant() {
                    for value in $t::generator().sample(COUNT) {
                        if value.is_nan() { continue; }
                        assert_eq!(Constant(value).sample(1).next().unwrap(), value);
                    }
                }

                #[test]
                fn is_in_range() {
                    for pair in <($t, $t)>::generator().sample(COUNT) {
                        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                        if low == high { continue; }
                        assert!((low..high)
                            .sample(COUNT)
                            .filter(|value| value.is_finite())
                            .all(|value| value >= low && value < high));
                    }
                }

                #[test]
                fn is_in_range_from() {
                    for low in <$t>::generator().sample(COUNT) {
                        assert!((low..)
                            .sample(COUNT)
                            .filter(|value| value.is_finite())
                            .all(|value| value >= low));
                    }
                }

                #[test]
                fn is_in_range_to() {
                    for high in <$t>::generator().sample(COUNT) {
                        assert!((..high)
                            .sample(COUNT)
                            .filter(|value| value.is_finite())
                            .all(|value| value < high || $t::MIN == high));
                    }
                }
            }
        };
        ($($t:ident),*) => { $(tests!($t);)* };
    }

    tests!(f32, f64);
}

use super::*;

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
                #[should_panic]
                fn empty_range() {
                    let value = <$t>::generator().sample(1).next().unwrap();
                    (value..value).sample(1).next().unwrap();
                }

                #[test]
                fn is_in_range() {
                    for pair in <($t, $t)>::generator().sample(1000) {
                        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                        if low == high { continue; }
                        assert!((low..high)
                            .sample(1000)
                            .filter(|value| value.is_finite())
                            .all(|value| value >= low && value < high));
                    }
                }

                #[test]
                fn is_in_range_from() {
                    for low in <$t>::generator().sample(1000) {
                        assert!((low..)
                            .sample(1000)
                            .filter(|value| value.is_finite())
                            .all(|value| value >= low));
                    }
                }

                #[test]
                fn is_in_range_to() {
                    for high in <$t>::generator().sample(1000) {
                        assert!((..high)
                            .sample(1000)
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

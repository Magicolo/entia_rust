use super::*;

mod range {
    use super::*;

    macro_rules! tests {
        ($t:ident, ($c:expr) $(,$f:ident)?) => {
            mod $t {
                use super::*;

                #[test]
                fn has_sample() {
                    <$t>::generator().sample(1).next().unwrap();
                }

                #[test]
                fn sample_has_count() {
                    for i in 0..COUNT {
                        assert_that(&<$t>::generator().sample(i).len()).is_equal_to(i);
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
                    for value in $t::generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                        assert_that(&clone(value).sample(1).next().unwrap()).is_equal_to(value);
                    }
                }

                #[test]
                fn is_in_range() {
                    for pair in <($t, $t)>::generator().sample(COUNT) $(.filter(|(low, high)| low.$f() && high.$f()))? {
                        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                        if $c(low, high) { continue; }
                        for value in (low..high).sample(COUNT) $(.filter(|value| value.$f()))? {
                            assert_that(&value).is_greater_than_or_equal_to(low);
                            assert_that(&value).is_less_than(high);
                        }
                    }
                }

                #[test]
                fn is_in_range_inclusive() {
                    for pair in <($t, $t)>::generator().sample(COUNT) $(.filter(|(low, high)| low.$f() && high.$f()))? {
                        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                        for value in (low..=high).sample(COUNT) $(.filter(|value| value.$f()))? {
                            assert_that(&value).is_greater_than_or_equal_to(low);
                            assert_that(&value).is_less_than_or_equal_to(high);
                        }
                    }
                }

                #[test]
                fn is_in_range_from() {
                    for low in <$t>::generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                        for value in (low..).sample(COUNT) $(.filter(|value| value.$f()))? {
                            assert_that(&value).is_greater_than_or_equal_to(low);
                        }
                    }
                }

                #[test]
                fn is_in_range_to() {
                    for high in <$t>::generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                        if $c($t::MIN, high) { continue; }
                        for value in (..high).sample(COUNT) $(.filter(|value| value.$f()))? {
                            assert_that(&value).is_less_than(high);
                        }
                    }
                }

                #[test]
                fn is_in_range_to_inclusive() {
                    for high in <$t>::generator().sample(COUNT) {
                        for value in (..=high).sample(COUNT) $(.filter(|value| value.$f()))? {
                            assert_that(&value).is_less_than_or_equal_to(high);
                        }
                    }
                }
            }
        };
        ($($t:ident),*, ($c:expr)) => { $(tests!($t, ($c));)* };
    }

    tests!(
        u8,
        u16,
        u32,
        u64,
        u128,
        i8,
        i16,
        i32,
        i64,
        i128,
        (|low, high| low == high)
    );
    tests!(
        f32,
        (|low: f32, high: f32| high - low < f32::EPSILON),
        is_finite
    );
    tests!(
        f64,
        (|low: f64, high: f64| high - low < f64::EPSILON),
        is_finite
    );
}

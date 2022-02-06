use super::*;

mod range {
    use super::*;

    macro_rules! tests {
        ($t:ident, [$c:expr], [$($f:ident)?], [$($p:ident)?]) => {
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
                    (value..value).generator().sample(1).next().unwrap();
                }

                #[test]
                fn is_constant() {
                    for value in $t::generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                        assert_that(&[value].sample(1).next().unwrap()).is_equal_to(value);
                    }
                }

                #[test]
                fn is_in_range() {
                    for pair in <($t, $t)>::generator().sample(COUNT) $(.filter(|(low, high)| low.$f() && high.$f()))? {
                        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                        if $c(low, high) { continue; }
                        for value in (low..high).generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                            assert_that(&value).is_greater_than_or_equal_to(low);
                            assert_that(&value).is_less_than(high);
                        }
                    }
                }

                #[test]
                fn is_in_range_inclusive() {
                    for pair in <($t, $t)>::generator().sample(COUNT) $(.filter(|(low, high)| low.$f() && high.$f()))? {
                        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                        for value in (low..=high).generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                            assert_that(&value).is_greater_than_or_equal_to(low);
                            assert_that(&value).is_less_than_or_equal_to(high);
                        }
                    }
                }

                #[test]
                fn is_in_range_from() {
                    for low in <$t>::generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                        for value in (low..).generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                            assert_that(&value).is_greater_than_or_equal_to(low);
                        }
                    }
                }

                #[test]
                fn is_in_range_to() {
                    for high in <$t>::generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                        if $c($t::MIN, high) { continue; }
                        for value in (..high).generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                            assert_that(&value).is_less_than(high);
                        }
                    }
                }

                #[test]
                fn is_in_range_to_inclusive() {
                    for high in <$t>::generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                        for value in (..=high).generator().sample(COUNT) $(.filter(|value| value.$f()))? {
                            assert_that(&value).is_less_than_or_equal_to(high);
                        }
                    }
                }

                #[test]
                fn is_positive() {
                    for value in positive::<$t>().sample(COUNT) {
                        assert_that(&value).is_greater_than_or_equal_to(0 as $t);
                    }
                }

                $(#[cfg($p)])?
                #[test]
                fn is_negative() {
                    for value in negative::<$t>().sample(COUNT) {
                        assert_that(&value).is_less_than(0 as $t);
                    }
                }
            }
        };
    }

    macro_rules! tests_signed {
        ($($t:ident),*) => { $(tests!($t, [|low, high| low == high], [], []);)* };
    }

    macro_rules! tests_unsigned {
        ($($t:ident),*) => { $(tests!($t, [|low, high| low == high], [], [POSITIVE]);)* };
    }

    macro_rules! tests_floating {
        ($($t:ident),*) => { $(tests!($t, [|low: $t, high: $t| high - low < $t::EPSILON], [is_finite], []);)* };
    }

    tests_signed!(i8, i16, i32, i64, i128);
    tests_unsigned!(u8, u16, u32, u64, u128);
    tests_floating!(f32, f64);
}

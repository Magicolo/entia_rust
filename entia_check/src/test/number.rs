use super::*;

mod range {
    use super::*;

    macro_rules! tests {
        ($t:ident, [$f:expr], [$($m:ident),*]) => {
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
                    for value in number::<$t>().sample(COUNT) {
                        assert_that(&value.sample(1).next().unwrap()).is_equal_to(value);
                    }
                }

                #[test]
                fn is_in_range() {
                    for pair in (number::<$t>(), number::<$t>()).sample(COUNT) {
                        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                        if $f(low, high) { continue; }
                        for value in (low..high).generator().sample(COUNT) {
                            assert_that(&value).is_greater_than_or_equal_to(low);
                            assert_that(&value).is_less_than(high);
                        }
                    }
                }

                #[test]
                fn is_in_range_inclusive() {
                    for pair in (number::<$t>(), number::<$t>()).sample(COUNT) {
                        let (low, high) = (pair.0.min(pair.1), pair.0.max(pair.1));
                        for value in (low..=high).generator().sample(COUNT) {
                            assert_that(&value).is_greater_than_or_equal_to(low);
                            assert_that(&value).is_less_than_or_equal_to(high);
                        }
                    }
                }

                #[test]
                fn is_in_range_from() {
                    for low in number::<$t>().sample(COUNT) {
                        for value in (low..).generator().sample(COUNT) {
                            assert_that(&value).is_greater_than_or_equal_to(low);
                        }
                    }
                }

                #[test]
                fn is_in_range_to() {
                    for high in number::<$t>().sample(COUNT) {
                        if $f($t::MIN, high) { continue; }
                        for value in (..high).generator().sample(COUNT) {
                            assert_that(&value).is_less_than(high);
                        }
                    }
                }

                #[test]
                fn is_in_range_to_inclusive() {
                    for high in number::<$t>().sample(COUNT) {
                        for value in (..=high).generator().sample(COUNT) {
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

                $($m!(INNER $t);)*
            }
        };
    }

    macro_rules! tests_integer {
        (INNER $t:ident) => {
            #[test]
            fn check_less_than_finds_minimum() {
                for right in positive::<$t>().sample(COUNT) {
                    if let Err(error) = positive::<$t>().check(COUNT, |&left| left < right) {
                        assert_that(&(right - *error.shrunk()))
                            .is_less_than_or_equal_to(right / 100 as $t)
                    }
                }
            }

            #[test]
            fn check_tuple_shrinks_irrelevant_items() {
                for right in positive::<$t>().sample(COUNT) {
                    if let Err(error) =
                        (positive::<$t>(), positive::<$t>()).check(COUNT, |&(left, _)| left < right)
                    {
                        assert_that(&error.shrunk().1).is_equal_to(0 as $t);
                    }
                }
            }
        };
        ($t:ident, $m:ident) => {
            tests!($t, [|low, high| low == high], [$m, tests_integer]);
        };
    }

    macro_rules! tests_signed {
        (INNER $t:ident) => {
            #[test]
            fn is_negative() {
                for value in negative::<$t>().sample(COUNT) {
                    assert_that(&value).is_less_than(0 as $t);
                }
            }

            #[test]
            fn check_greater_than_finds_maximum() {
                for right in negative::<$t>().sample(COUNT) {
                    if let Err(error) = negative::<$t>().check(COUNT, |&left| left > right) {
                        assert_that(&(*error.shrunk() - right))
                            .is_less_than_or_equal_to(right.abs() / 100 as $t)
                    }
                }
            }
        };
        ($($t:ident),*) => { $(tests_integer!($t, tests_signed);)* };
    }

    macro_rules! tests_unsigned {
        (INNER $t:ident) => {};
        ($($t:ident),*) => { $(tests_integer!($t, tests_unsigned);)* };
    }

    macro_rules! tests_floating {
        (INNER $t:ident) => {
            #[test]
            fn is_negative() {
                for value in negative::<$t>().sample(COUNT) {
                    assert_that(&value).is_less_than(0 as $t);
                }
            }
        };
        ($($t:ident),*) => { $(tests!($t, [|low: $t, high: $t| high - low < $t::EPSILON], [tests_floating]);)* };
    }

    tests_signed!(i8, i16, i32, i64, i128);
    tests_unsigned!(u8, u16, u32, u64, u128);
    tests_floating!(f32, f64);
}

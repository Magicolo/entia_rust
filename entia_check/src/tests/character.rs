use super::*;

#[test]
fn has_sample() {
    char::generator().sample(1).next().unwrap();
}

#[test]
fn sample_has_count() {
    for i in 0..COUNT {
        assert_that(&char::generator().sample(i).len()).is_equal_to(i);
    }
}

#[test]
#[should_panic]
fn empty_range() {
    let value = char::generator().sample(1).next().unwrap();
    (value..value).sample(1).next().unwrap();
}

#[test]
fn is_constant() {
    for value in char::generator().sample(COUNT) {
        assert_that(&clone(value).sample(1).next().unwrap()).is_equal_to(value);
    }
}

#[test]
fn is_ascii() {
    for value in ascii().sample(COUNT) {
        assert_that(&value.is_ascii()).is_true();
    }
}

#[test]
fn is_digit() {
    for value in digit().sample(COUNT) {
        assert_that(&value.is_ascii_digit()).is_true();
    }
}

#[test]
fn is_alphabetic() {
    for value in letter().sample(COUNT) {
        assert_that(&value.is_ascii_alphabetic()).is_true();
    }
}

#[test]
fn full_does_not_panic() {
    for _ in <char>::generator().sample(COUNT) {}
}

macro_rules! collection {
    ($m:ident, $t:ty $(, $i:ident)?) => {
        mod $m {
            use super::*;

            #[test]
            fn has_constant_count() {
                for i in 0..COUNT {
                    let value = char::generator().collect_with::<_, $t>(clone(i)).sample(1).next().unwrap();
                    assert!(value $(.$i())? .count() == i)
                }
            }

            #[test]
            fn is_ascii() {
                assert!(ascii()
                    .collect::<$t>()
                    .sample(COUNT)
                    .all(|value| value $(.$i())? .all(|value| value.is_ascii())))
            }

            #[test]
            fn is_digit() {
                assert!(digit()
                    .collect::<$t>()
                    .sample(COUNT)
                    .all(|value| value $(.$i())? .all(|value| value.is_ascii_digit())))
            }

            #[test]
            fn is_alphabetic() {
                assert!(letter()
                    .collect::<$t>()
                    .sample(COUNT)
                    .all(|value| value $(.$i())? .all(|value| value.is_ascii_alphabetic())))
            }
        }
    };
}

collection!(string, String, chars);
collection!(vec_char, Vec<char>, into_iter);
collection!(box_char, Box<[char]>, into_iter);

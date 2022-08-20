use std::{
    array, iter,
    mem::replace,
    ops::{Deref, DerefMut},
    vec,
};

pub enum Few<T> {
    Zero,
    One([T; 1]),
    Two([T; 2]),
    Three([T; 3]),
    Four([T; 4]),
    More(Vec<T>),
}

pub enum Iterator<T> {
    Zero,
    One(array::IntoIter<T, 1>),
    Two(array::IntoIter<T, 2>),
    Three(array::IntoIter<T, 3>),
    Four(array::IntoIter<T, 4>),
    More(vec::IntoIter<T>),
}

impl<T> Few<T> {
    pub fn len(&self) -> usize {
        match self {
            Few::Zero => 0,
            Few::One(_) => 1,
            Few::Two(_) => 2,
            Few::Three(_) => 3,
            Few::Four(_) => 4,
            Few::More(values) => values.len(),
        }
    }

    pub fn push(&mut self, value: T) {
        *self = match replace(self, Few::Zero) {
            Few::Zero => Few::One([value]),
            Few::One([_1]) => Few::Two([_1, value]),
            Few::Two([_1, _2]) => Few::Three([_1, _2, value]),
            Few::Three([_1, _2, _3]) => Few::Four([_1, _2, _3, value]),
            Few::Four([_1, _2, _3, _4]) => Few::More(vec![_1, _2, _3, _4, value]),
            Few::More(mut values) => {
                values.push(value);
                Few::More(values)
            }
        };
    }

    pub fn pop(&mut self) -> Option<T> {
        match replace(self, Few::Zero) {
            Few::Zero => None,
            Few::One([_1]) => Some(_1),
            Few::Two([_1, _2]) => {
                *self = Few::One([_1]);
                Some(_2)
            }
            Few::Three([_1, _2, _3]) => {
                *self = Few::Two([_1, _2]);
                Some(_3)
            }
            Few::Four([_1, _2, _3, _4]) => {
                *self = Few::Three([_1, _2, _3]);
                Some(_4)
            }
            Few::More(mut values) => {
                let value = values.pop();
                *self = Few::More(values);
                value
            }
        }
    }
}

impl<T> Deref for Few<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            Few::Zero => &[],
            Few::One(values) => values,
            Few::Two(values) => values,
            Few::Three(values) => values,
            Few::Four(values) => values,
            Few::More(values) => values,
        }
    }
}

impl<T> DerefMut for Few<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Few::Zero => &mut [],
            Few::One(values) => values,
            Few::Two(values) => values,
            Few::Three(values) => values,
            Few::Four(values) => values,
            Few::More(values) => values,
        }
    }
}

impl<T> IntoIterator for Few<T> {
    type Item = T;
    type IntoIter = Iterator<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        match self {
            Few::Zero => Iterator::Zero,
            Few::One(values) => Iterator::One(values.into_iter()),
            Few::Two(values) => Iterator::Two(values.into_iter()),
            Few::Three(values) => Iterator::Three(values.into_iter()),
            Few::Four(values) => Iterator::Four(values.into_iter()),
            Few::More(values) => Iterator::More(values.into_iter()),
        }
    }
}

impl<T> iter::Iterator for Iterator<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Iterator::Zero => None,
            Iterator::One(iterator) => iterator.next(),
            Iterator::Two(iterator) => iterator.next(),
            Iterator::Three(iterator) => iterator.next(),
            Iterator::Four(iterator) => iterator.next(),
            Iterator::More(iterator) => iterator.next(),
        }
    }
}

impl<T> FromIterator<T> for Few<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut iterator = iter.into_iter();
        let _1 = match iterator.next() {
            Some(_1) => _1,
            None => return Self::Zero,
        };
        let _2 = match iterator.next() {
            Some(_2) => _2,
            None => return Self::One([_1]),
        };
        let _3 = match iterator.next() {
            Some(_3) => _3,
            None => return Self::Two([_1, _2]),
        };
        let _4 = match iterator.next() {
            Some(_4) => _4,
            None => return Self::Three([_1, _2, _3]),
        };
        let _5 = match iterator.next() {
            Some(_5) => _5,
            None => return Self::Four([_1, _2, _3, _4]),
        };
        let mut values = vec![_1, _2, _3, _4, _5];
        values.extend(iterator);
        Self::More(values)
    }
}

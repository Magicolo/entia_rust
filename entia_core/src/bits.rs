use std::convert::TryInto;
use std::iter::{self, FromIterator};
use std::mem::size_of;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};
use std::{cmp::min, hash::Hash};

#[derive(Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bits {
    buckets: Vec<u128>,
}

pub struct Iterator<'a> {
    index: usize,
    shift: usize,
    bits: &'a Bits,
}

impl Bits {
    const SIZE: usize = size_of::<u128>() * 8;

    #[inline]
    pub const fn new() -> Self {
        Self {
            buckets: Vec::new(),
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.buckets.len() * Self::SIZE
    }

    pub fn has(&self, index: usize) -> bool {
        if index < self.capacity() {
            let bit = 1 << (index % Self::SIZE);
            (self.buckets[index / Self::SIZE] & bit) == bit
        } else {
            false
        }
    }

    pub fn has_all(&self, bits: &Bits) -> bool {
        self.buckets.len() == bits.buckets.len()
            && self
                .buckets
                .iter()
                .zip(bits.buckets.iter())
                .all(|(&left, &right)| left & right == right)
    }

    pub fn has_any(&self, bits: &Bits) -> bool {
        self.buckets
            .iter()
            .zip(bits.buckets.iter())
            .any(|(&left, &right)| left & right > 0)
    }

    pub fn set(&mut self, index: usize, value: bool) {
        if value {
            self.ensure(index + 1);
            let bit = 1 << (index % Self::SIZE);
            self.buckets[index / Self::SIZE] |= bit;
        } else if index < self.capacity() {
            let bit = 1 << (index % Self::SIZE);
            self.buckets[index / Self::SIZE] &= !bit;
            self.shrink();
        }
    }

    pub fn not(&mut self) {
        self.buckets.iter_mut().for_each(|value| *value = !*value);
        self.shrink();
    }

    pub fn or(&mut self, bits: &Bits) {
        // No need to shrink since an 'or' operation cannot make add more '0' bits to a bucket.
        self.binary(bits, true, false, |left, right| left | right);
    }

    pub fn or_not(&mut self, bits: &Bits) {
        // No need to shrink since an 'or' operation cannot make add more '0' bits to a bucket.
        self.binary(bits, true, false, |left, right| left | !right);
    }

    pub fn and(&mut self, bits: &Bits) {
        self.binary(bits, false, true, |left, right| left & right);
    }

    pub fn and_not(&mut self, bits: &Bits) {
        self.binary(bits, false, true, |left, right| left & !right);
    }

    pub fn xor(&mut self, bits: &Bits) {
        self.binary(bits, true, true, |left, right| left ^ right);
    }

    pub fn xor_not(&mut self, bits: &Bits) {
        self.binary(bits, true, true, |left, right| left ^ !right);
    }

    fn ensure(&mut self, capacity: usize) {
        while self.capacity() < capacity {
            self.buckets.push(0);
        }
    }

    fn shrink(&mut self) {
        while let Some(value) = self.buckets.pop() {
            if value > 0 {
                self.buckets.push(value);
                break;
            }
        }
    }

    fn binary(&mut self, bits: &Bits, ensure: bool, shrink: bool, merge: fn(u128, u128) -> u128) {
        let count = if ensure {
            self.ensure(bits.capacity());
            self.buckets.len()
        } else {
            min(self.buckets.len(), bits.buckets.len())
        };

        for i in 0..count {
            self.buckets[i] = merge(self.buckets[i], bits.buckets[i]);
        }

        if shrink {
            self.shrink();
        }
    }
}

impl<'a> IntoIterator for &'a Bits {
    type Item = usize;
    type IntoIter = Iterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iterator {
            index: 0,
            shift: 0,
            bits: self,
        }
    }
}

impl iter::Iterator for Iterator<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&value) = self.bits.buckets.get(self.index) {
            if value > 0 {
                while self.shift < Bits::SIZE {
                    let shift = self.shift;
                    let bit = 1 << shift;
                    self.shift += 1;
                    if (value & bit) == bit {
                        return Some(self.index * Bits::SIZE + shift);
                    }
                }
            }

            self.index += 1;
            self.shift = 0;
        }
        None
    }
}

impl BitOr<&Bits> for Bits {
    type Output = Bits;

    #[inline]
    fn bitor(mut self, rhs: &Bits) -> Self::Output {
        self.or(rhs);
        self
    }
}

impl BitOrAssign<&Bits> for Bits {
    #[inline]
    fn bitor_assign(&mut self, rhs: &Bits) {
        self.or(rhs);
    }
}

impl BitAnd<&Bits> for Bits {
    type Output = Bits;

    #[inline]
    fn bitand(mut self, rhs: &Bits) -> Self::Output {
        self.and(rhs);
        self
    }
}

impl BitAndAssign<&Bits> for Bits {
    #[inline]
    fn bitand_assign(&mut self, rhs: &Bits) {
        self.and(rhs);
    }
}

impl BitXor<&Bits> for Bits {
    type Output = Bits;

    #[inline]
    fn bitxor(mut self, rhs: &Bits) -> Self::Output {
        self.xor(rhs);
        self
    }
}

impl BitXorAssign<&Bits> for Bits {
    #[inline]
    fn bitxor_assign(&mut self, rhs: &Bits) {
        self.xor(rhs);
    }
}

impl Not for Bits {
    type Output = Bits;

    #[inline]
    fn not(mut self) -> Self::Output {
        (&mut self).not();
        self
    }
}

impl<I: TryInto<usize>> FromIterator<I> for Bits {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        let mut bits = Bits::new();
        for index in iter {
            if let Ok(index) = index.try_into() {
                bits.set(index, true);
            }
        }
        bits
    }
}

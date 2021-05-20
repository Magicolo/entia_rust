use std::hash::Hash;
use std::hash::Hasher;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

#[derive(Default, Clone)]
pub struct Bits {
    bits: Vec<usize>,
}

pub struct BitsIterator<'a> {
    index: usize,
    shift: usize,
    bits: &'a Bits,
}

impl Bits {
    const SIZE: usize = std::mem::size_of::<usize>() * 8;

    #[inline]
    pub const fn new() -> Self {
        Self { bits: Vec::new() }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.bits.len() * Self::SIZE
    }

    pub fn has(&self, index: usize) -> bool {
        if index < self.capacity() {
            let bit = 1 << (index % Self::SIZE);
            (self.bits[index / Self::SIZE] & bit) == bit
        } else {
            false
        }
    }

    pub fn has_all(&self, bits: &Bits) -> bool {
        if self.bits.len() == bits.bits.len() {
            for i in 0..self.bits.len() {
                let value = bits.bits[i];
                if (self.bits[i] & value) != value {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn has_any(&self, bits: &Bits) -> bool {
        let count = std::cmp::min(self.bits.len(), bits.bits.len());
        for i in 0..count {
            if (self.bits[i] & bits.bits[i]) > 0 {
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn has_none(&self, bits: &Bits) -> bool {
        !self.has_any(bits)
    }

    pub fn not(&mut self) {
        for value in self.bits.iter_mut() {
            *value = !*value;
        }
    }

    pub fn or(&mut self, bits: &Bits) {
        self.ensure(bits.capacity());
        for (index, &value) in bits.bits.iter().enumerate() {
            self.bits[index] |= value;
        }
    }

    pub fn and(&mut self, bits: &Bits) {
        self.ensure(bits.capacity());
        for (index, &value) in bits.bits.iter().enumerate() {
            self.bits[index] &= value;
        }
    }

    pub fn xor(&mut self, bits: &Bits) {
        self.ensure(bits.capacity());
        for (index, &value) in bits.bits.iter().enumerate() {
            self.bits[index] ^= value;
        }
    }

    pub fn add(&mut self, index: usize) {
        self.ensure(index + 1);
        let bit = 1 << (index % Self::SIZE);
        self.bits[index / Self::SIZE] |= bit;
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.capacity() {
            let bit = 1 << (index % Self::SIZE);
            self.bits[index / Self::SIZE] &= !bit;
        }
        self.shrink();
    }

    pub fn remove_all(&mut self, bits: &Bits) {
        let count = std::cmp::min(self.bits.len(), bits.bits.len());
        for i in 0..count {
            self.bits[i] &= !bits.bits[i];
        }
        self.shrink();
    }

    fn ensure(&mut self, capacity: usize) {
        while self.capacity() < capacity {
            self.bits.push(0);
        }
    }

    fn shrink(&mut self) {
        while let Some(value) = self.bits.pop() {
            if value > 0 {
                self.bits.push(value);
                break;
            }
        }
    }
}

impl<'a> IntoIterator for &'a Bits {
    type Item = usize;
    type IntoIter = BitsIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        BitsIterator {
            index: 0,
            shift: 0,
            bits: self,
        }
    }
}

impl Iterator for BitsIterator<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&value) = self.bits.bits.get(self.index) {
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

impl PartialEq<Bits> for Bits {
    fn eq(&self, other: &Bits) -> bool {
        if self.bits.len() == other.bits.len() {
            for i in 0..self.bits.len() {
                if self.bits[i] != other.bits[i] {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }
}

impl Eq for Bits {}

impl Hash for Bits {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for &value in self.bits.iter() {
            state.write_usize(value);
        }
    }
}

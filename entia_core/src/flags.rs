use crate::utility::short_type_name;
use std::{
    cmp,
    fmt::{Debug, Formatter, Result},
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not},
};

pub trait IntoFlags<T = Self> {
    type Value;
    fn flags(self) -> Flags<T, Self::Value>;
}

pub struct Flags<T: ?Sized, F = usize>(F, PhantomData<T>);

impl<T, F> Flags<T, F> {
    #[inline]
    pub const fn new(value: F) -> Self {
        Self(value, PhantomData)
    }

    #[inline]
    pub fn has_all<R, O>(&self, other: R) -> bool
    where
        Self: BitAnd<R, Output = O> + Clone,
        O: PartialEq<Self>,
    {
        (self.clone() & other) == self.clone()
    }

    #[inline]
    pub fn has_any<R, O>(&self, other: R) -> bool
    where
        Self: BitAnd<R, Output = O> + Clone + BitXor,
        O: PartialEq<<Self as BitXor>::Output>,
    {
        (self.clone() & other) != (self.clone() ^ self.clone())
    }

    #[inline]
    pub fn has_none<R, O>(&self, other: R) -> bool
    where
        Self: BitAnd<R, Output = O> + Clone + BitXor,
        O: PartialEq<<Self as BitXor>::Output>,
    {
        !self.has_any(other)
    }
}

impl<T, F: IntoFlags<T> + Clone> IntoFlags<T> for &mut F {
    type Value = F::Value;

    #[inline]
    fn flags(self) -> Flags<T, Self::Value> {
        Flags::new(self.clone().flags().0)
    }
}

impl<T, F: IntoFlags<T> + Clone> IntoFlags<T> for &F {
    type Value = F::Value;

    #[inline]
    fn flags(self) -> Flags<T, Self::Value> {
        Flags::new(self.clone().flags().0)
    }
}

impl<T, F> IntoFlags<T> for Flags<T, F> {
    type Value = F;

    #[inline]
    fn flags(self) -> Flags<T, F> {
        self
    }
}

impl<T: Into<F>, F> From<T> for Flags<T, F> {
    #[inline]
    fn from(value: T) -> Self {
        Self::new(value.into())
    }
}

impl<T, F: Copy> Copy for Flags<T, F> {}

impl<T, F: Clone> Clone for Flags<T, F> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.0.clone())
    }
}

impl<T, F: Default> Default for Flags<T, F> {
    #[inline]
    fn default() -> Self {
        Self::new(F::default())
    }
}

impl<T, F: Debug> Debug for Flags<T, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_tuple(&short_type_name::<T>())
            .field(&self.0)
            .finish()
    }
}

impl<T, F: Hash> Hash for Flags<T, F> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T, R: IntoFlags<T> + Clone, F: PartialEq<R::Value>> PartialEq<R> for Flags<T, F> {
    #[inline]
    fn eq(&self, other: &R) -> bool {
        self.0.eq(&other.flags().0)
    }

    #[inline]
    fn ne(&self, other: &R) -> bool {
        self.0.ne(&other.flags().0)
    }
}

impl<T, F: PartialEq<Self> + Eq + Clone> Eq for Flags<T, F> {}

impl<T, R: IntoFlags<T> + Clone, F: PartialOrd<R::Value>> PartialOrd<R> for Flags<T, F> {
    #[inline]
    fn partial_cmp(&self, other: &R) -> Option<cmp::Ordering> {
        self.0.partial_cmp(&other.flags().0)
    }
}

impl<T, F: PartialOrd<Self> + Ord + Clone> Ord for Flags<T, F> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T, F: Not> Not for Flags<T, F> {
    type Output = Flags<T, F::Output>;

    #[inline]
    fn not(self) -> Self::Output {
        Flags::new(!self.0)
    }
}

impl<T, R: IntoFlags<T>, F: BitOr<R::Value>> BitOr<R> for Flags<T, F> {
    type Output = Flags<T, F::Output>;

    #[inline]
    fn bitor(self, rhs: R) -> Self::Output {
        Flags::new(self.0 | rhs.flags().0)
    }
}

impl<T, R: IntoFlags<T>, F: BitAnd<R::Value>> BitAnd<R> for Flags<T, F> {
    type Output = Flags<T, F::Output>;

    #[inline]
    fn bitand(self, rhs: R) -> Self::Output {
        Flags::new(self.0 & rhs.flags().0)
    }
}

impl<T, R: IntoFlags<T>, F: BitXor<R::Value>> BitXor<R> for Flags<T, F> {
    type Output = Flags<T, F::Output>;

    #[inline]
    fn bitxor(self, rhs: R) -> Self::Output {
        Flags::new(self.0 ^ rhs.flags().0)
    }
}

impl<T, R: IntoFlags<T>, F: BitOrAssign<R::Value>> BitOrAssign<R> for Flags<T, F> {
    #[inline]
    fn bitor_assign(&mut self, rhs: R) {
        self.0 |= rhs.flags().0;
    }
}

impl<T, R: IntoFlags<T>, F: BitAndAssign<R::Value>> BitAndAssign<R> for Flags<T, F> {
    #[inline]
    fn bitand_assign(&mut self, rhs: R) {
        self.0 &= rhs.flags().0;
    }
}

impl<T, R: IntoFlags<T>, F: BitXorAssign<R::Value>> BitXorAssign<R> for Flags<T, F> {
    #[inline]
    fn bitxor_assign(&mut self, rhs: R) {
        self.0 ^= rhs.flags().0;
    }
}

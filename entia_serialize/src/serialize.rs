use crate::{recurse, serializer::*};
use entia_macro::count;
use std::{
    marker::PhantomData,
    ops::{Bound, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
};

pub trait Serialize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error>;
}

impl<T: Serialize + ?Sized> Serialize for &T {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        (&**self).serialize(serializer)
    }
}

impl<T: Serialize + ?Sized> Serialize for &mut T {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        (&**self).serialize(serializer)
    }
}

impl<T: Serialize> Serialize for Option<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        let enumeration = serializer.enumeration::<Self>()?;
        match self {
            None => enumeration.variant::<0, 2>("None")?.unit(),
            Some(value) => enumeration
                .variant::<1, 2>("Some")?
                .tuple::<1>()?
                .items([value]),
        }
    }
}

impl<T: Serialize, E: Serialize> Serialize for Result<T, E> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        let enumeration = serializer.enumeration::<Self>()?;
        match self {
            Ok(value) => enumeration
                .variant::<1, 2>("Ok")?
                .tuple::<1>()?
                .items([value]),
            Err(error) => enumeration
                .variant::<1, 2>("Err")?
                .tuple::<1>()?
                .items([error]),
        }
    }
}

impl<T: ?Sized> Serialize for PhantomData<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer.structure::<Self>()?.unit()
    }
}

impl Serialize for str {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer.string(self)
    }
}

impl Serialize for String {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer.string(self)
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer.slice(self)
    }
}

impl<T: Serialize> Serialize for [T] {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer.slice(self)
    }
}

impl<T: Serialize, const N: usize> Serialize for [T; N] {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer.array(self)
    }
}

impl Serialize for RangeFull {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer.structure::<Self>()?.unit()
    }
}

impl<T: Serialize> Serialize for Range<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer
            .structure::<Self>()?
            .map::<2>()?
            .pairs([("start", &self.start), ("end", &self.end)])
    }
}

impl<T: Serialize> Serialize for RangeInclusive<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer
            .structure::<Self>()?
            .map::<2>()?
            .pairs([("start", self.start()), ("end", self.end())])
    }
}

impl<T: Serialize> Serialize for RangeTo<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer
            .structure::<Self>()?
            .map::<2>()?
            .pairs([("end", &self.end)])
    }
}

impl<T: Serialize> Serialize for RangeToInclusive<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer
            .structure::<Self>()?
            .map::<2>()?
            .pairs([("end", &self.end)])
    }
}

impl<T: Serialize> Serialize for RangeFrom<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer
            .structure::<Self>()?
            .map::<2>()?
            .pairs([("start", &self.start)])
    }
}

impl<T: Serialize> Serialize for Bound<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        let enumeration = serializer.enumeration::<Self>()?;
        match self {
            Bound::Included(value) => enumeration
                .variant::<0, 3>("Included")?
                .tuple::<1>()?
                .items([value]),
            Bound::Excluded(value) => enumeration
                .variant::<1, 3>("Excluded")?
                .tuple::<1>()?
                .items([value]),
            Bound::Unbounded => enumeration.variant::<2, 3>("Unbounded")?.unit(),
        }
    }
}

macro_rules! primitive {
    ($t:ident) => {
        impl Serialize for $t {
            #[inline]
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
                serializer.$t(*self)
            }
        }
    };
    ($($t:ident),*) => {
        $(primitive!($t);)*
    }
}

primitive!(bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);

macro_rules! tuple {
    () => {
        impl Serialize for () {
            #[inline]
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
                serializer.unit()
            }
        }
    };
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Serialize,)*> Serialize for ($($t,)*) {
            #[inline]
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
                let ($($p,)*) = self;
                serializer.tuple::<{ count!($($p),*) }>()? $(.item($p)?)* .end()
            }
        }
    };
}

recurse!(tuple);

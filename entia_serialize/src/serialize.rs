use crate::{serializer::*, tuples};
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
            None => enumeration.variant("None", 0)?.unit(),
            Some(value) => enumeration.variant("Some", 1)?.tuple()?.item(value)?.end(),
        }
    }
}

impl<T: Serialize, E: Serialize> Serialize for Result<T, E> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        let enumeration = serializer.enumeration::<Self>()?;
        match self {
            Ok(value) => enumeration.variant("Ok", 0)?.tuple()?.item(value)?.end(),
            Err(error) => enumeration.variant("Err", 1)?.tuple()?.item(error)?.end(),
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
            .map()?
            .pairs([("start", &self.start), ("end", &self.end)])
    }
}

impl<T: Serialize> Serialize for RangeInclusive<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer
            .structure::<Self>()?
            .map()?
            .pairs([("start", self.start()), ("end", self.end())])
    }
}

impl<T: Serialize> Serialize for RangeTo<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer
            .structure::<Self>()?
            .map()?
            .pair("end", &self.end)?
            .end()
    }
}

impl<T: Serialize> Serialize for RangeToInclusive<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer
            .structure::<Self>()?
            .map()?
            .pair("end", &self.end)?
            .end()
    }
}

impl<T: Serialize> Serialize for RangeFrom<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        serializer
            .structure::<Self>()?
            .map()?
            .pair("start", &self.start)?
            .end()
    }
}

impl<T: Serialize> Serialize for Bound<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
        let enumeration = serializer.enumeration::<Self>()?;
        match self {
            Bound::Included(value) => enumeration
                .variant("Included", 0)?
                .tuple()?
                .item(value)?
                .end(),
            Bound::Excluded(value) => enumeration
                .variant("Excluded", 1)?
                .tuple()?
                .item(value)?
                .end(),
            Bound::Unbounded => enumeration.variant("Unbounded", 2)?.unit(),
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
                serializer.tuple::<Self>()? $(.item($p)?)* .end()
            }
        }
    };
}

tuples!(tuple);

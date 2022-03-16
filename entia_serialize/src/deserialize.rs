use crate::{deserializer::*, recurse};
use std::{
    marker::PhantomData,
    str::{self},
};

pub trait Deserialize {
    type Value;
    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error>;
}

#[derive(Debug)]
pub struct New<T: ?Sized>(PhantomData<T>);

impl<T: ?Sized> New<T> {
    #[inline]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: ?Sized> Copy for New<T> {}
impl<T: ?Sized> Clone for New<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}
impl<T: ?Sized> Default for New<T> {
    #[inline]
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: ?Sized> Deserialize for &New<T>
where
    New<T>: Deserialize,
{
    type Value = <New<T> as Deserialize>::Value;

    #[inline]
    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        (*self).deserialize(deserializer)
    }
}

impl<T: ?Sized> Deserialize for &mut New<T>
where
    New<T>: Deserialize,
{
    type Value = <New<T> as Deserialize>::Value;

    #[inline]
    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        (*self).deserialize(deserializer)
    }
}

impl<T: Deserialize, const N: usize> Deserialize for [T; N] {
    type Value = [T::Value; N];

    #[inline]
    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.array(self)
    }
}

impl<T, const N: usize> Deserialize for New<[T; N]>
where
    New<T>: Deserialize,
{
    type Value = [<New<T> as Deserialize>::Value; N];

    #[inline]
    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.array([New::<T>::new(); N])
    }
}

impl<'a, T, const N: usize> Deserialize for &'a mut [T; N]
where
    &'a mut T: Deserialize,
{
    type Value = [<&'a mut T as Deserialize>::Value; N];

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        // TODO: Use 'self.0.each_mut' when it stabilizes.
        let mut iterator = self.iter_mut();
        deserializer.array([(); N].map(|_| iterator.next().unwrap()))
    }
}

impl<'a, T> Deserialize for &'a mut [T]
where
    for<'b> &'b mut T: Deserialize,
{
    type Value = &'a mut [T];

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.slice::<T>(self, false)
    }
}

impl<'a> Deserialize for &'a mut str {
    type Value = &'a mut str;

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.string(self, false)
    }
}

impl<T> Deserialize for New<Vec<T>>
where
    New<T>: Deserialize<Value = T>,
{
    type Value = Vec<T>;

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let mut list = deserializer.list()?;
        let mut value = Vec::new();
        while let Some(item) = list.item()? {
            value.push(item.value(New::<T>::new())?);
        }
        Ok(value)
    }
}

impl<T> Deserialize for &mut Vec<T>
where
    for<'a> &'a mut T: Deserialize,
    New<T>: Deserialize<Value = T>,
{
    type Value = ();

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let mut list = deserializer.list()?;
        let mut index = 0;
        while let Some(item) = list.item()? {
            match self.get_mut(index) {
                Some(value) => {
                    item.value(value)?;
                }
                None => self.push(item.value(New::<T>::new())?),
            }
            index += 1;
        }
        self.truncate(index);
        Ok(())
    }
}

impl Deserialize for New<String> {
    type Value = String;

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let mut list = deserializer.list()?;
        let mut value = String::new();
        while let Some(item) = list.item()? {
            value.push(item.value(New::<char>::new())?);
        }
        Ok(value)
    }
}

impl Deserialize for &mut String {
    type Value = ();

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let mut list = deserializer.list()?;
        self.clear();
        while let Some(item) = list.item()? {
            self.push(item.value(New::<char>::new())?);
        }
        Ok(())
    }
}

macro_rules! primitive {
    ($t:ident) => {
        impl Deserialize for New<$t> {
            type Value = $t;

            #[inline]
            fn deserialize<D: Deserializer>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                deserializer.$t()
            }
        }

        impl Deserialize for &mut $t {
            type Value = ();

            #[inline]
            fn deserialize<D: Deserializer>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                *self = New::<$t>::new().deserialize(deserializer)?;
                Ok(())
            }
        }
    };
    ($($t:ident),*) => { $(primitive!($t);)* }
}

primitive!(bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);

macro_rules! tuple {
    () => {
        impl Deserialize for () {
            type Value = ();

            #[inline]
            fn deserialize<D: Deserializer>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                deserializer.unit()
            }
        }

        impl Deserialize for New<()> {
            type Value = ();

            #[inline]
            fn deserialize<D: Deserializer>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                ().deserialize(deserializer)
            }
        }

        impl Deserialize for &mut () {
            type Value = ();

            #[inline]
            fn deserialize<D: Deserializer>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                ().deserialize(deserializer)
            }
        }
    };
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Deserialize,)*> Deserialize for ($($t,)*) {
            type Value = ($($t::Value,)*);

            #[inline]
            fn deserialize<D: Deserializer>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                let ($($p,)*) = self;
                let mut list = deserializer.list()?;
                $(let $p = match list.item()? { Some(item) => item.value($p)?, None => list.miss($p)?, };)*
                while let Some(item) = list.item()? {
                    item.excess()?;
                }
                Ok(($($p,)*))
            }
        }

        impl<$($t,)*> Deserialize for New<($($t,)*)>
        where
            $(New<$t>: Deserialize,)*
        {
            type Value = ($(<New<$t> as Deserialize>::Value,)*);

            #[inline]
            fn deserialize<D: Deserializer>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                ($(New::<$t>::new(),)*).deserialize(deserializer)
            }
        }

        impl<'a, $($t,)*> Deserialize for &'a mut ($($t,)*)
        where
            $(&'a mut $t: Deserialize,)*
        {
            type Value = ($(<&'a mut $t as Deserialize>::Value,)*);

            #[inline]
            fn deserialize<D: Deserializer>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                let ($($p,)*) = self;
                ($($p,)*).deserialize(deserializer)
            }
        }
    };
}

recurse!(tuple);

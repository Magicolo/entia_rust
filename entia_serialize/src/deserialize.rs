use crate::{deserializer::*, recurse};
use std::marker::PhantomData;

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
        let mut list = deserializer.list()?;
        let mut values = [(); N].map(|_| None);
        let mut index = 0;
        for value in self {
            values[index] = Some(match list.item()? {
                Some(item) => item.value(value)?,
                None => list.miss(value)?,
            });
            index += 1;
        }
        list.drain()?;
        Ok(values.map(Option::unwrap))
    }
}

impl<T, const N: usize> Deserialize for New<[T; N]>
where
    New<T>: Deserialize,
{
    type Value = [<New<T> as Deserialize>::Value; N];

    #[inline]
    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        [New::<T>::new(); N].deserialize(deserializer)
    }
}

impl<'a, T, const N: usize> Deserialize for &'a mut [T; N]
where
    &'a mut T: Deserialize,
{
    type Value = [<&'a mut T as Deserialize>::Value; N];

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let mut iterator = self.iter_mut();
        [(); N]
            .map(|_| iterator.next().unwrap())
            .deserialize(deserializer)
    }
}

impl<'a, T> Deserialize for &'a mut [T]
where
    for<'b> &'b mut T: Deserialize,
{
    type Value = &'a mut [T];

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let mut list = deserializer.list()?;
        for i in 0..self.len() {
            match list.item()? {
                Some(item) => item.value(&mut self[i])?,
                None => return Ok(&mut self[..i]),
            };
        }
        list.drain()?;
        Ok(self)
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
        let mut value = String::new();
        value.deserialize(deserializer)?;
        Ok(value)
    }
}

impl Deserialize for &mut String {
    type Value = ();

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        self.clear();
        let mut list = deserializer.list()?;
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
        tuple!([NEW]);
        tuple!([MUT]);
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
                list.drain()?;
                Ok(($($p,)*))
            }
        }

        tuple!([NEW] $($p, $t),*);
        tuple!([MUT] $($p, $t),*);
    };
    ([NEW] $($p:ident, $t:ident),*) => {
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
    };
    ([MUT] $($p:ident, $t:ident),*) => {
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

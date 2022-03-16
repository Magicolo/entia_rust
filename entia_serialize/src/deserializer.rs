use crate::deserialize::*;
use std::str::{self, Utf8Error};

pub trait Deserializer: Sized {
    type Error: From<Utf8Error>;
    type Structure: Structure<Error = Self::Error>;
    type Enumeration: Enumeration<Error = Self::Error>;
    type List: List<Error = Self::Error>;
    type Map: Map<Error = Self::Error>;

    fn unit(self) -> Result<(), Self::Error>;
    fn bool(self) -> Result<bool, Self::Error>;
    fn char(self) -> Result<char, Self::Error>;
    fn u8(self) -> Result<u8, Self::Error>;
    fn u16(self) -> Result<u16, Self::Error>;
    fn u32(self) -> Result<u32, Self::Error>;
    fn u64(self) -> Result<u64, Self::Error>;
    fn u128(self) -> Result<u128, Self::Error>;
    fn usize(self) -> Result<usize, Self::Error>;
    fn i8(self) -> Result<i8, Self::Error>;
    fn i16(self) -> Result<i16, Self::Error>;
    fn i32(self) -> Result<i32, Self::Error>;
    fn i64(self) -> Result<i64, Self::Error>;
    fn i128(self) -> Result<i128, Self::Error>;
    fn isize(self) -> Result<isize, Self::Error>;
    fn f32(self) -> Result<f32, Self::Error>;
    fn f64(self) -> Result<f64, Self::Error>;

    fn list(self) -> Result<Self::List, Self::Error>;
    fn map(self) -> Result<Self::Map, Self::Error>;

    #[inline]
    fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error> {
        self.list()
    }

    #[inline]
    fn string(self, value: &mut str, fill: bool) -> Result<&mut str, Self::Error> {
        let value = unsafe { value.as_bytes_mut() };
        let (index, error) = match self.bytes(value, fill) {
            Ok(value) => match str::from_utf8_mut(value) {
                Ok(value) => (value.len(), None),
                Err(error) => {
                    // TODO: Do I really want to recover from an invalid 'str'?
                    if fill {
                        (error.valid_up_to(), Some(error.into()))
                    } else {
                        (error.valid_up_to(), None)
                    }
                }
            },
            Err(error) => (0, Some(error)),
        };

        // Ensures that no invalid byte remains in the 'str'.
        value[index..].fill(0);
        match error {
            Some(error) => Err(error),
            // SAFETY: 'str' has been validated up to the index which is guarateed by 'error.valid_up_to'.
            None => Ok(unsafe { str::from_utf8_unchecked_mut(&mut value[..index]) }),
        }
    }

    #[inline]
    fn bytes(self, value: &mut [u8], fill: bool) -> Result<&mut [u8], Self::Error> {
        self.slice::<u8>(value, fill)
    }

    #[inline]
    fn array<T: Deserialize, const N: usize>(
        self,
        value: [T; N],
    ) -> Result<[T::Value; N], Self::Error> {
        let mut list = self.list()?;
        let mut values = [(); N].map(|_| None);
        let mut index = 0;
        for value in value {
            values[index] = Some(match list.item()? {
                Some(item) => item.value(value)?,
                None => list.miss(value)?,
            });
            index += 1;
        }
        list.drain()?;
        Ok(values.map(Option::unwrap))
    }

    #[inline]
    fn slice<T>(self, value: &mut [T], fill: bool) -> Result<&mut [T], Self::Error>
    where
        for<'a> &'a mut T: Deserialize,
    {
        let mut list = self.list()?;
        let mut index = 0;
        while let Some(item) = list.item()? {
            match value.get_mut(index) {
                Some(value) => {
                    item.value(value)?;
                    index += 1;
                }
                None => item.excess()?,
            }
        }

        if fill {
            for value in &mut value[index..] {
                list.miss(value)?;
            }
        }
        Ok(&mut value[..index])
    }

    fn structure<T: ?Sized>(self) -> Result<Self::Structure, Self::Error>;
    fn enumeration<T: ?Sized>(self) -> Result<Self::Enumeration, Self::Error>;
}

pub trait Structure {
    type Error;
    type List: List<Error = Self::Error>;
    type Map: Map<Error = Self::Error>;

    fn unit(self) -> Result<(), Self::Error>;
    fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error>;
    fn map<const N: usize>(self) -> Result<Self::Map, Self::Error>;
}

pub trait Enumeration {
    type Error;
    type Variant: Variant<Error = Self::Error>;

    fn never<K: Deserialize>(self, key: K) -> Result<K::Value, Self::Error>;
    fn variant<K: Deserialize, const N: usize>(
        self,
        key: K,
    ) -> Result<(K::Value, Self::Variant), Self::Error>;
}

pub trait Variant {
    type Error;
    type Map: Map<Error = Self::Error>;
    type List: List<Error = Self::Error>;

    fn unit(self, name: &'static str, index: usize) -> Result<(), Self::Error>;
    fn map<const N: usize>(
        self,
        name: &'static str,
        index: usize,
    ) -> Result<Self::Map, Self::Error>;
    fn tuple<const N: usize>(
        self,
        name: &'static str,
        index: usize,
    ) -> Result<Self::List, Self::Error>;
    fn excess<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error>;
}

pub trait Map {
    type Error;
    type Field: Field<Error = Self::Error>;

    fn field<K: Deserialize>(
        &mut self,
        key: K,
    ) -> Result<Option<(K::Value, Self::Field)>, Self::Error>;
    fn miss<K, V: Deserialize>(&mut self, key: K, value: V) -> Result<V::Value, Self::Error>;
}

pub trait Field: Sized {
    type Error;

    fn value<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error>;

    #[inline]
    fn excess(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub trait List: Sized {
    type Error;
    type Item: Item<Error = Self::Error>;

    fn item(&mut self) -> Result<Option<Self::Item>, Self::Error>;
    fn miss<V: Deserialize>(&mut self, value: V) -> Result<V::Value, Self::Error>;

    #[inline]
    fn drain(mut self) -> Result<(), Self::Error> {
        while let Some(item) = self.item()? {
            item.excess()?;
        }
        Ok(())
    }
}

pub trait Item: Sized {
    type Error;

    fn value<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error>;

    #[inline]
    fn excess(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

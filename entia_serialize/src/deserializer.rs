use crate::deserialize::*;
use std::str;

pub trait Deserializer: Sized {
    type Error;
    type List: List<Error = Self::Error>;
    type Map: Map<Error = Self::Error>;
    type Structure: Structure<Error = Self::Error>;
    type Enumeration: Enumeration<Error = Self::Error>;

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
    fn structure(self) -> Result<Self::Structure, Self::Error>;
    fn enumeration(self) -> Result<Self::Enumeration, Self::Error>;
}

pub trait Structure {
    type Error;
    type List: List<Error = Self::Error>;
    type Map: Map<Error = Self::Error>;

    fn unit(self) -> Result<(), Self::Error>;
    fn tuple(self) -> Result<Self::List, Self::Error>;
    fn map(self) -> Result<Self::Map, Self::Error>;
}

pub trait Enumeration {
    type Error;
    type Variant: Variant<Error = Self::Error>;

    fn never(self) -> Self::Error;
    fn variant<K: Deserialize>(self, key: K) -> Result<(K::Value, Self::Variant), Self::Error>;
}

pub trait Variant {
    type Error;
    type Map: Map<Error = Self::Error>;
    type List: List<Error = Self::Error>;

    fn unit(self, name: &str, index: usize) -> Result<(), Self::Error>;
    fn map(self, name: &str, index: usize) -> Result<Self::Map, Self::Error>;
    fn tuple(self, name: &str, index: usize) -> Result<Self::List, Self::Error>;
    fn miss<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error>;
}

pub trait Map {
    type Error;
    type Item: Item<Error = Self::Error>;

    fn pair<K: Deserialize>(
        &mut self,
        key: K,
    ) -> Result<Option<(K::Value, Self::Item)>, Self::Error>;
    fn miss<V: Deserialize>(&mut self, value: V) -> Result<V::Value, Self::Error>;

    #[inline]
    fn pairs<K: Deserialize, V: Deserialize, I: IntoIterator<Item = (K, V)>>(
        mut self,
        key: K,
        values: I,
    ) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        for (key, value) in values {
            match self.pair(key)? {
                Some((_, item)) => item.value(value)?,
                None => self.miss(value)?,
            };
        }
        self.drain(key)
    }

    #[inline]
    fn drain<K: Deserialize>(mut self, key: K) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        todo!()
        // while let Some((_, item)) = self.pair(key)? {
        //     item.excess()?;
        // }
        // Ok(())
    }
}

pub trait List {
    type Error;
    type Item: Item<Error = Self::Error>;

    fn item(&mut self) -> Result<Option<Self::Item>, Self::Error>;
    fn miss<V: Deserialize>(&mut self, value: V) -> Result<V::Value, Self::Error>;

    #[inline]
    fn items<V: Deserialize, I: IntoIterator<Item = V>>(
        mut self,
        values: I,
    ) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        for value in values {
            match self.item()? {
                Some(item) => item.value(value)?,
                None => self.miss(value)?,
            };
        }
        self.drain()
    }

    #[inline]
    fn drain(mut self) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        while let Some(item) = self.item()? {
            item.excess()?;
        }
        Ok(())
    }
}

pub trait Item {
    type Error;

    fn value<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error>;

    #[inline]
    fn excess(self) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        Ok(())
    }
}

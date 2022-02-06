use crate::{deserialize::*, node::Node};
use std::str::{self, Utf8Error};

pub trait Deserializer {
    type Error;
    type Primitive: Primitive<Error = Self::Error>;
    type Structure: Structure<Error = Self::Error>;
    type Sequence: Sequence<Error = Self::Error>;
    type Enumeration: Enumeration<Error = Self::Error>;

    fn primitive(self) -> Result<Self::Primitive, Self::Error>;
    fn structure(self, name: &'static str) -> Result<Self::Structure, Self::Error>;
    fn enumeration(self, name: &'static str) -> Result<Self::Enumeration, Self::Error>;
    fn sequence(self) -> Result<Self::Sequence, Self::Error>;
}

pub trait Primitive: Sized {
    type Error;

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

    // #[inline]
    // fn shared<'a, T: ?Sized>(self) -> Result<&'a T, Self::Error> {
    //     Ok(self.exclusive()?)
    // }
    // fn exclusive<'a, T: ?Sized>(self) -> Result<&'a mut T, Self::Error>;

    // #[inline]
    // fn constant<T: ?Sized>(self) -> Result<*const T, Self::Error> {
    //     Ok(self.mutable()?)
    // }
    // fn mutable<T: ?Sized>(self) -> Result<*mut T, Self::Error>;
}

pub trait Structure {
    type Error;
    type Fields: Fields<Error = Self::Error>;
    fn map<const N: usize>(self) -> Result<Self::Fields, Self::Error>;
}

pub trait Sequence: Sized {
    // TODO: Is there a better way to achieve this than 'From<Utf8Error>'?
    type Error: From<Utf8Error>;
    type Items: Items<Error = Self::Error>;
    type Fields: Fields<Error = Self::Error>;

    fn list(self) -> Result<Self::Items, Self::Error>;
    fn map(self) -> Result<Self::Fields, Self::Error>;

    fn tuple<const N: usize>(self) -> Result<Self::Items, Self::Error> {
        self.list()
    }

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

    fn bytes(self, value: &mut [u8], fill: bool) -> Result<&mut [u8], Self::Error> {
        self.slice::<u8>(value, fill)
    }

    fn array<T: Deserialize, const N: usize>(
        self,
        value: [T; N],
    ) -> Result<[T::Value; N], Self::Error> {
        let mut items = self.list()?;
        let mut values = [(); N].map(|_| None);
        let mut index = 0;
        for value in value {
            index += 1;
            values[index] = Some(match items.item()? {
                Some(item) => item.value(value)?,
                None => items.miss(value)?,
            });
        }
        Ok(values.map(Option::unwrap))
    }

    fn slice<T>(self, value: &mut [T], fill: bool) -> Result<&mut [T], Self::Error>
    where
        for<'a> &'a mut T: Deserialize,
    {
        let mut items = self.list()?;
        let mut index = 0;
        while let Some(item) = items.item()? {
            match value.get_mut(index) {
                Some(value) => {
                    item.value(value)?;
                    index += 1;
                }
                None => item.excess()?,
            }
        }

        if fill {
            while index < value.len() {
                items.miss(&mut value[index])?;
                index += 1;
            }
        }

        Ok(&mut value[..index])
    }
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
    type Fields: Fields<Error = Self::Error>;
    type Items: Items<Error = Self::Error>;

    fn unit(self, name: &'static str, index: usize) -> Result<(), Self::Error>;
    fn map<const N: usize>(
        self,
        name: &'static str,
        index: usize,
    ) -> Result<Self::Fields, Self::Error>;
    fn tuple<const N: usize>(
        self,
        name: &'static str,
        index: usize,
    ) -> Result<Self::Items, Self::Error>;
    fn excess<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error>;
}

pub trait Fields: Sized {
    type Error;
    type Field: Field<Error = Self::Error>;

    // For a json parser:
    // - Tries to parse at head as 'K'.
    // - If 'Ok(K)', move the head right after the ':' and return 'Some((K, Field(head)))'.
    // - If 'Err(E)', move the head before the next key and loop; if no keys are left return 'None'.
    fn field<K: Deserialize>(
        &mut self,
        key: K,
    ) -> Result<Option<(K::Value, Self::Field)>, Self::Error>;
    // The implementator will decide if a missing field should produce an error or if it can be recovered
    // by producing a default value of type 'V'.
    fn miss<K, V: Deserialize>(&mut self, key: K, value: V) -> Result<V::Value, Self::Error>;
}

pub trait Field {
    type Error;
    fn value<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error>;
    fn excess(self) -> Result<(), Self::Error>;
}

pub trait Items: Sized {
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

pub trait Item {
    type Error;
    fn value<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error>;
    fn excess(self) -> Result<(), Self::Error>;
}

pub struct NodeDeserializer<'a>(&'a Node);
pub struct FieldsDeserializer<'a>(&'a [(Node, Node)], usize);
pub struct ItemsDeserializer<'a>(&'a [Node], usize);

pub enum Error {
    ExpectedObjectNode,
    Utf8(Utf8Error),
}

impl From<Utf8Error> for Error {
    fn from(error: Utf8Error) -> Self {
        Error::Utf8(error)
    }
}

impl NodeDeserializer<'_> {
    pub fn deserialize<T>(node: &Node) -> Result<<New<T> as Deserialize>::Value, Error>
    where
        New<T>: Deserialize,
    {
        New::<T>::new().deserialize(NodeDeserializer(node))
    }
}

impl Deserializer for NodeDeserializer<'_> {
    type Error = Error;
    type Primitive = Self;
    type Structure = Self;
    type Sequence = Self;
    type Enumeration = Self;

    #[inline]
    fn primitive(self) -> Result<Self::Primitive, Self::Error> {
        Ok(self)
    }
    #[inline]
    fn structure(self, _: &'static str) -> Result<Self::Structure, Self::Error> {
        Ok(self)
    }
    #[inline]
    fn enumeration(self, _: &'static str) -> Result<Self::Enumeration, Self::Error> {
        Ok(self)
    }
    #[inline]
    fn sequence(self) -> Result<Self::Sequence, Self::Error> {
        Ok(self)
    }
}

impl Primitive for NodeDeserializer<'_> {
    type Error = Error;

    #[inline]
    fn unit(self) -> Result<(), Self::Error> {
        Ok(())
    }
    #[inline]
    fn bool(self) -> Result<bool, Self::Error> {
        Ok(self.0.boolean().unwrap_or_default())
    }
    #[inline]
    fn char(self) -> Result<char, Self::Error> {
        Ok(self.0.character().unwrap_or_default())
    }
    #[inline]
    fn u8(self) -> Result<u8, Self::Error> {
        Ok(self.isize()? as _)
    }
    #[inline]
    fn u16(self) -> Result<u16, Self::Error> {
        Ok(self.isize()? as _)
    }
    #[inline]
    fn u32(self) -> Result<u32, Self::Error> {
        Ok(self.isize()? as _)
    }
    #[inline]
    fn u64(self) -> Result<u64, Self::Error> {
        Ok(self.isize()? as _)
    }
    #[inline]
    fn u128(self) -> Result<u128, Self::Error> {
        Ok(self.isize()? as _)
    }
    #[inline]
    fn usize(self) -> Result<usize, Self::Error> {
        Ok(self.isize()? as _)
    }
    #[inline]
    fn i8(self) -> Result<i8, Self::Error> {
        Ok(self.isize()? as _)
    }
    #[inline]
    fn i16(self) -> Result<i16, Self::Error> {
        Ok(self.isize()? as _)
    }
    #[inline]
    fn i32(self) -> Result<i32, Self::Error> {
        Ok(self.isize()? as _)
    }
    #[inline]
    fn i64(self) -> Result<i64, Self::Error> {
        Ok(self.isize()? as _)
    }
    #[inline]
    fn i128(self) -> Result<i128, Self::Error> {
        Ok(self.isize()? as _)
    }
    #[inline]
    fn isize(self) -> Result<isize, Self::Error> {
        Ok(self.0.integer().unwrap_or_default())
    }
    #[inline]
    fn f32(self) -> Result<f32, Self::Error> {
        Ok(self.f64()? as _)
    }
    #[inline]
    fn f64(self) -> Result<f64, Self::Error> {
        Ok(self.0.floating().unwrap_or_default())
    }
}

impl<'a> Structure for NodeDeserializer<'a> {
    type Error = Error;
    type Fields = FieldsDeserializer<'a>;

    fn map<const N: usize>(self) -> Result<Self::Fields, Self::Error> {
        match self.0 {
            Node::Object(pairs) => Ok(FieldsDeserializer(pairs, 0)),
            _ => Err(Error::ExpectedObjectNode),
        }
    }
}

impl Enumeration for NodeDeserializer<'_> {
    type Error = Error;
    type Variant = Self;

    // TODO: Is this right?
    #[inline]
    fn never<K: Deserialize>(self, key: K) -> Result<K::Value, Self::Error> {
        key.deserialize(self)
    }

    #[inline]
    fn variant<K: Deserialize, const N: usize>(
        self,
        key: K,
    ) -> Result<(K::Value, Self::Variant), Self::Error> {
        match self.0 {
            Node::Object(pairs) if pairs.len() > 0 => {
                let pair = &pairs[0];
                let key = key.deserialize(NodeDeserializer(&pair.0))?;
                Ok((key, self))
            }
            _ => Err(Error::ExpectedObjectNode),
        }
    }
}

impl<'a> Variant for NodeDeserializer<'a> {
    type Error = Error;
    type Fields = FieldsDeserializer<'a>;
    type Items = ItemsDeserializer<'a>;

    fn unit(self, _: &'static str, _: usize) -> Result<(), Self::Error> {
        todo!()
    }

    fn map<const N: usize>(self, _: &'static str, _: usize) -> Result<Self::Fields, Self::Error> {
        todo!()
    }

    fn tuple<const N: usize>(self, _: &'static str, _: usize) -> Result<Self::Items, Self::Error> {
        todo!()
    }

    #[inline]
    fn excess<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
        value.deserialize(self)
    }
}

impl<'a> Sequence for NodeDeserializer<'a> {
    type Error = Error;
    type Items = ItemsDeserializer<'a>;
    type Fields = FieldsDeserializer<'a>;

    fn list(self) -> Result<Self::Items, Self::Error> {
        todo!()
    }

    fn map(self) -> Result<Self::Fields, Self::Error> {
        todo!()
    }
}

impl<'a> Fields for FieldsDeserializer<'a> {
    type Error = Error;
    type Field = NodeDeserializer<'a>;

    fn field<K: Deserialize>(
        &mut self,
        key: K,
    ) -> Result<Option<(K::Value, Self::Field)>, Self::Error> {
        match self.0.get(self.1) {
            Some(pair) => {
                self.1 += 1;
                let key = key.deserialize(NodeDeserializer(&pair.0))?;
                let field = NodeDeserializer(&pair.1);
                Ok(Some((key, field)))
            }
            None => Ok(None),
        }
    }

    fn miss<K, V: Deserialize>(&mut self, _: K, value: V) -> Result<V::Value, Self::Error> {
        value.deserialize(NodeDeserializer(&Node::Null))
    }
}

impl Field for NodeDeserializer<'_> {
    type Error = Error;

    fn value<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
        value.deserialize(self)
    }

    fn excess(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a> Items for ItemsDeserializer<'a> {
    type Error = Error;
    type Item = NodeDeserializer<'a>;

    fn item(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        match self.0.get(self.1) {
            Some(node) => {
                self.1 += 1;
                Ok(Some(NodeDeserializer(node)))
            }
            None => Ok(None),
        }
    }

    fn miss<V: Deserialize>(&mut self, value: V) -> Result<V::Value, Self::Error> {
        value.deserialize(NodeDeserializer(&Node::Null))
    }
}

impl Item for NodeDeserializer<'_> {
    type Error = Error;

    fn value<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
        value.deserialize(self)
    }

    fn excess(self) -> Result<(), Self::Error> {
        Ok(())
    }
}

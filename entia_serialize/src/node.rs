use crate::{
    deserialize::Deserialize,
    deserializer::{self, Deserializer},
    serialize::Serialize,
    serializer::{self, adapt::Adapt, state::State, Serializer},
};
use std::mem;

use self::{deserialize::NodeDeserializer, serialize::NodeSerializer};

#[derive(Clone, Debug)]
pub enum Node {
    Unit,
    Bool(bool),
    Char(char),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    F32(f32),
    F64(f64),
    Bytes(Vec<u8>),
    String(String),
    Variant(&'static str, usize, Box<Node>),
    List(Vec<Node>),
    Map(Vec<(Node, Node)>),
}

macro_rules! integer {
    ($t:ident) => {
        impl From<Node> for $t {
            #[inline]
            fn from(node: Node) -> $t {
                node.$t()
            }
        }

        impl Node {
            #[inline]
            pub fn $t(&self) -> $t {
                match self {
                    Node::Unit => $t::default(),
                    Node::Bool(value) => *value as u8 as $t,
                    Node::Char(value) => *value as u32 as $t,
                    Node::U8(value) => *value as $t,
                    Node::U16(value) => *value as $t,
                    Node::U32(value) => *value as $t,
                    Node::U64(value) => *value as $t,
                    Node::U128(value) => *value as $t,
                    Node::Usize(value) => *value as $t,
                    Node::I8(value) => *value as $t,
                    Node::I16(value) => *value as $t,
                    Node::I32(value) => *value as $t,
                    Node::I64(value) => *value as $t,
                    Node::I128(value) => *value as $t,
                    Node::Isize(value) => *value as $t,
                    Node::F32(value) => *value as $t,
                    Node::F64(value) => *value as $t,
                    Node::Bytes(value) => value[..mem::size_of::<$t>()].try_into().map($t::from_ne_bytes).unwrap_or_default(),
                    Node::String(value) => value.parse().unwrap_or_default(),
                    Node::Variant(_, _, value) => value.$t(),
                    Node::List(nodes) => nodes.get(0).map(|node| node.$t()).unwrap_or_default(),
                    Node::Map(nodes) => nodes.get(0).map(|pair| pair.1.$t()).unwrap_or_default(),
                }
            }
        }
    };
    ($($t:ident),*) => { $(integer!($t);)* }
}

impl Node {
    #[inline]
    pub fn bool(&self) -> bool {
        match self {
            Node::Bool(value) => *value,
            node => node.u8() > 0,
        }
    }

    #[inline]
    pub fn char(&self) -> char {
        match self {
            Node::Char(value) => *value,
            Node::String(value) => value.chars().next().unwrap_or_default(),
            node => char::from_u32(node.u32()).unwrap_or_default(),
        }
    }
}

integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);

impl Default for Node {
    #[inline]
    fn default() -> Self {
        Node::Unit
    }
}

pub mod serialize {
    use super::*;

    pub struct NodeSerializer;
    pub struct ListSerializer(pub Vec<Node>);
    pub struct MapSerializer(pub Vec<(Node, Node)>);

    #[derive(Clone, Debug)]
    pub enum Error {}

    impl Serializer for NodeSerializer {
        type Value = Node;
        type Error = Error;
        type Map = MapSerializer;
        type List = ListSerializer;
        type Structure = Self;
        type Enumeration = Self;

        fn unit(self) -> Result<Self::Value, Self::Error> {
            Ok(Node::Unit)
        }
        fn bool(self, value: bool) -> Result<Self::Value, Self::Error> {
            Ok(Node::Bool(value))
        }
        fn char(self, value: char) -> Result<Self::Value, Self::Error> {
            Ok(Node::Char(value))
        }
        fn u8(self, value: u8) -> Result<Self::Value, Self::Error> {
            Ok(Node::U8(value))
        }
        fn u16(self, value: u16) -> Result<Self::Value, Self::Error> {
            Ok(Node::U16(value))
        }
        fn u32(self, value: u32) -> Result<Self::Value, Self::Error> {
            Ok(Node::U32(value))
        }
        fn u64(self, value: u64) -> Result<Self::Value, Self::Error> {
            Ok(Node::U64(value))
        }
        fn usize(self, value: usize) -> Result<Self::Value, Self::Error> {
            Ok(Node::Usize(value))
        }
        fn u128(self, value: u128) -> Result<Self::Value, Self::Error> {
            Ok(Node::U128(value))
        }
        fn i8(self, value: i8) -> Result<Self::Value, Self::Error> {
            Ok(Node::I8(value))
        }
        fn i16(self, value: i16) -> Result<Self::Value, Self::Error> {
            Ok(Node::I16(value))
        }
        fn i32(self, value: i32) -> Result<Self::Value, Self::Error> {
            Ok(Node::I32(value))
        }
        fn i64(self, value: i64) -> Result<Self::Value, Self::Error> {
            Ok(Node::I64(value))
        }
        fn isize(self, value: isize) -> Result<Self::Value, Self::Error> {
            Ok(Node::Isize(value))
        }
        fn i128(self, value: i128) -> Result<Self::Value, Self::Error> {
            Ok(Node::I128(value))
        }
        fn f32(self, value: f32) -> Result<Self::Value, Self::Error> {
            Ok(Node::F32(value))
        }
        fn f64(self, value: f64) -> Result<Self::Value, Self::Error> {
            Ok(Node::F64(value))
        }

        fn list(self) -> Result<Self::List, Self::Error> {
            Ok(ListSerializer(Vec::new()))
        }
        fn map(self) -> Result<Self::Map, Self::Error> {
            Ok(MapSerializer(Vec::new()))
        }
        fn structure<T: ?Sized>(self) -> Result<Self::Structure, Self::Error> {
            Ok(self)
        }
        fn enumeration<T: ?Sized>(self) -> Result<Self::Enumeration, Self::Error> {
            Ok(self)
        }
    }

    impl serializer::List for ListSerializer {
        type Value = Node;
        type Error = Error;

        fn item<T: Serialize>(mut self, item: T) -> Result<Self, Self::Error> {
            self.0.push(item.serialize(NodeSerializer)?);
            Ok(self)
        }

        fn end(self) -> Result<Self::Value, Self::Error> {
            Ok(Node::List(self.0))
        }
    }

    impl serializer::Map for MapSerializer {
        type Value = Node;
        type Error = Error;

        fn pair<K: Serialize, V: Serialize>(
            mut self,
            key: K,
            value: V,
        ) -> Result<Self, Self::Error> {
            self.0.push((
                key.serialize(NodeSerializer)?,
                value.serialize(NodeSerializer)?,
            ));
            Ok(self)
        }

        fn end(self) -> Result<Self::Value, Self::Error> {
            Ok(Node::Map(self.0))
        }
    }

    impl serializer::Structure for NodeSerializer {
        type Value = Node;
        type Error = Error;
        type Map = MapSerializer;
        type List = ListSerializer;

        fn unit(self) -> Result<Self::Value, Self::Error> {
            Serializer::unit(self)
        }
        fn tuple(self) -> Result<Self::List, Self::Error> {
            Serializer::list(self)
        }
        fn map(self) -> Result<Self::Map, Self::Error> {
            Serializer::map(self)
        }
    }

    impl serializer::Enumeration for NodeSerializer {
        type Value = Node;
        type Error = Error;
        type Structure = Adapt<State<Self, (&'static str, usize)>, Self::Value, Self::Error>;

        fn never(self) -> Result<Self::Value, Self::Error> {
            Serializer::unit(self)
        }

        fn variant(self, name: &'static str, index: usize) -> Result<Self::Structure, Self::Error> {
            let adapt: fn(
                Result<(Self::Value, (&'static str, usize)), Self::Error>,
            ) -> Result<Self::Value, Self::Error> = |result| {
                result.map(|(node, (name, index))| Node::Variant(name, index, node.into()))
            };
            Ok(self.state((name, index)).adapt(adapt))
        }
    }
}

pub mod deserialize {
    use super::*;
    use std::str::Utf8Error;

    pub struct NodeDeserializer(pub Node);
    pub struct ChildDeserializer(Node, usize);

    #[derive(Clone, Debug)]
    pub enum Error {
        Invalid,
        Utf8(Utf8Error),
    }

    impl Default for NodeDeserializer {
        #[inline]
        fn default() -> Self {
            Self(Node::default())
        }
    }

    impl From<Utf8Error> for Error {
        fn from(error: Utf8Error) -> Self {
            Error::Utf8(error)
        }
    }

    impl Deserializer for NodeDeserializer {
        type Error = Error;
        type Structure = Self;
        type Enumeration = Self;
        type List = ChildDeserializer;
        type Map = ChildDeserializer;

        #[inline]
        fn unit(self) -> Result<(), Self::Error> {
            Ok(())
        }
        #[inline]
        fn bool(self) -> Result<bool, Self::Error> {
            Ok(self.0.bool())
        }
        #[inline]
        fn char(self) -> Result<char, Self::Error> {
            Ok(self.0.char())
        }
        #[inline]
        fn u8(self) -> Result<u8, Self::Error> {
            Ok(self.0.u8())
        }
        #[inline]
        fn u16(self) -> Result<u16, Self::Error> {
            Ok(self.0.u16())
        }
        #[inline]
        fn u32(self) -> Result<u32, Self::Error> {
            Ok(self.0.u32())
        }
        #[inline]
        fn u64(self) -> Result<u64, Self::Error> {
            Ok(self.0.u64())
        }
        #[inline]
        fn u128(self) -> Result<u128, Self::Error> {
            Ok(self.0.u128())
        }
        #[inline]
        fn usize(self) -> Result<usize, Self::Error> {
            Ok(self.0.usize())
        }
        #[inline]
        fn i8(self) -> Result<i8, Self::Error> {
            Ok(self.0.i8())
        }
        #[inline]
        fn i16(self) -> Result<i16, Self::Error> {
            Ok(self.0.i16())
        }
        #[inline]
        fn i32(self) -> Result<i32, Self::Error> {
            Ok(self.0.i32())
        }
        #[inline]
        fn i64(self) -> Result<i64, Self::Error> {
            Ok(self.0.i64())
        }
        #[inline]
        fn i128(self) -> Result<i128, Self::Error> {
            Ok(self.0.i128())
        }
        #[inline]
        fn isize(self) -> Result<isize, Self::Error> {
            Ok(self.0.isize())
        }
        #[inline]
        fn f32(self) -> Result<f32, Self::Error> {
            Ok(self.0.f32())
        }
        #[inline]
        fn f64(self) -> Result<f64, Self::Error> {
            Ok(self.0.f64())
        }

        #[inline]
        fn list(self) -> Result<Self::List, Self::Error> {
            Ok(ChildDeserializer(self.0, 0))
        }

        #[inline]
        fn map(self) -> Result<Self::Map, Self::Error> {
            Ok(ChildDeserializer(self.0, 0))
        }

        #[inline]
        fn structure<T: ?Sized>(self) -> Result<Self::Structure, Self::Error> {
            Ok(self)
        }
        #[inline]
        fn enumeration<T: ?Sized>(self) -> Result<Self::Enumeration, Self::Error> {
            Ok(self)
        }
    }

    impl deserializer::Structure for NodeDeserializer {
        type Error = Error;
        type List = ChildDeserializer;
        type Map = ChildDeserializer;

        fn unit(self) -> Result<(), Self::Error> {
            Deserializer::unit(self)
        }
        fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error> {
            Deserializer::list(self)
        }
        fn map<const N: usize>(self) -> Result<Self::Map, Self::Error> {
            Deserializer::map(self)
        }
    }

    impl deserializer::Enumeration for NodeDeserializer {
        type Error = Error;
        type Variant = Self;

        fn never<K: Deserialize>(self, key: K) -> Result<K::Value, Self::Error> {
            key.deserialize(NodeDeserializer::default())
        }

        fn variant<K: Deserialize, const N: usize>(
            self,
            key: K,
        ) -> Result<(K::Value, Self::Variant), Self::Error> {
            match self.0 {
                Node::Variant(name, index, value) => {
                    todo!()
                    // let key = key.deserialize(NodeDeserializer(*index))?;
                    // Ok((key, NodeDeserializer(*value)))
                }
                _ => Err(Error::Invalid),
            }
            // todo!()
            // match self.0 {
            //     Enumeration::Variant(value, index) => Ok((
            //         key.deserialize(NodeDeserializer(Node::Usize(index)))?,
            //         VariantDeserializer(value, index),
            //     )),
            //     node => Err(Error::Invalid(Node::Enumeration(node))),
            // }
        }
    }

    impl deserializer::Variant for NodeDeserializer {
        type Error = Error;
        type Map = ChildDeserializer;
        type List = ChildDeserializer;

        fn unit(self, name: &'static str, index: usize) -> Result<(), Self::Error> {
            Deserializer::unit(self)
        }
        fn map<const N: usize>(
            self,
            name: &'static str,
            index: usize,
        ) -> Result<Self::Map, Self::Error> {
            Deserializer::map(self)
        }
        fn tuple<const N: usize>(
            self,
            name: &'static str,
            index: usize,
        ) -> Result<Self::List, Self::Error> {
            Deserializer::list(self)
        }
        fn excess<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
            value.deserialize(NodeDeserializer::default())
        }
    }

    impl deserializer::List for ChildDeserializer {
        type Error = Error;
        type Item = NodeDeserializer;

        fn item(&mut self) -> Result<Option<Self::Item>, Self::Error> {
            let item = match &self.0 {
                Node::List(nodes) => nodes.get(self.1).map(|node| NodeDeserializer(node.clone())),
                Node::Map(nodes) => nodes
                    .get(self.1)
                    .map(|node| NodeDeserializer(node.1.clone())),
                Node::Variant(_, _, node) if self.1 == 0 => {
                    Some(NodeDeserializer(node.as_ref().clone()))
                }
                node if self.1 == 0 => Some(NodeDeserializer(node.clone())),
                _ => None,
            };
            self.1 += 1;
            Ok(item)
        }

        fn miss<V: Deserialize>(&mut self, value: V) -> Result<V::Value, Self::Error> {
            value.deserialize(NodeDeserializer::default())
        }
    }

    impl deserializer::Item for NodeDeserializer {
        type Error = Error;

        fn value<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
            value.deserialize(NodeDeserializer(self.0))
        }
    }

    impl deserializer::Map for ChildDeserializer {
        type Error = Error;
        type Field = NodeDeserializer;

        fn field<K: Deserialize>(
            &mut self,
            key: K,
        ) -> Result<Option<(K::Value, Self::Field)>, Self::Error> {
            let pair = match &self.0 {
                Node::List(nodes) => match nodes.get(self.1) {
                    Some(node) => Some((
                        key.deserialize(NodeDeserializer(Node::Usize(self.1)))?,
                        NodeDeserializer(node.clone()),
                    )),
                    None => None,
                },
                Node::Map(nodes) => match nodes.get(self.1) {
                    Some(pair) => Some((
                        key.deserialize(NodeDeserializer(pair.0.clone()))?,
                        NodeDeserializer(pair.1.clone()),
                    )),
                    None => None,
                },
                Node::Variant(_, index, node) if self.1 == 0 => Some((
                    key.deserialize(NodeDeserializer(Node::Usize(*index)))?,
                    NodeDeserializer(node.as_ref().clone()),
                )),
                node if self.1 == 0 => Some((
                    key.deserialize(NodeDeserializer(Node::Usize(self.1)))?,
                    NodeDeserializer(node.clone()),
                )),
                _ => None,
            };
            self.1 += 1;
            Ok(pair)
        }

        fn miss<K, V: Deserialize>(&mut self, _: K, value: V) -> Result<V::Value, Self::Error> {
            value.deserialize(NodeDeserializer::default())
        }
    }

    impl deserializer::Field for NodeDeserializer {
        type Error = Error;

        fn value<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
            value.deserialize(NodeDeserializer(self.0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{convert::*, *};
    use crate::deserialize::New;

    #[derive(Debug, Clone, Copy)]
    struct Boba(bool);
    #[derive(Debug, Clone, Copy)]
    struct Fett(bool);

    impl Serialize for Boba {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
            use serializer::*;
            serializer.structure::<Boba>()?.tuple()?.item(self.0)?.end()
        }
    }

    impl Deserialize for New<Fett> {
        type Value = Fett;

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            use deserializer::*;
            let mut list = deserializer.structure::<Fett>()?.tuple::<1>()?;
            let a = if let Some(item) = list.item()? {
                item.value(New::<bool>::new())
            } else {
                list.miss(New::<bool>::new())
            }?;
            list.drain()?;
            Ok(Fett(a))
        }
    }

    #[test]
    fn boba_to_fett() -> Result<(), Error> {
        Boba(true).convert(New::<Fett>::new()).map(|_| ())
    }
}

pub mod convert {
    use super::*;

    #[derive(Clone, Debug)]
    pub enum Error {
        Serialize(serialize::Error),
        Deserialize(deserialize::Error),
    }

    pub trait Convert<T>: Serialize {
        type Value;
        type Error;
        fn convert(self, with: T) -> Result<Self::Value, Self::Error>;
    }

    impl<S: Serialize, D: Deserialize> Convert<D> for S {
        type Value = D::Value;
        type Error = Error;

        fn convert(self, to: D) -> Result<Self::Value, Self::Error> {
            let serializer = NodeSerializer;
            let node = self.serialize(serializer).map_err(Error::Serialize)?;
            let deserializer = NodeDeserializer(node);
            to.deserialize(deserializer).map_err(Error::Deserialize)
        }
    }
}

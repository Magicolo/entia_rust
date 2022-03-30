use self::{deserialize::NodeDeserializer, serialize::NodeSerializer};
use crate::{
    deserialize::{Deserialize, New},
    deserializer::{
        self, Deserializer, Enumeration as _, List as _, Map as _, Structure as _, Variant as _,
    },
    serialize::Serialize,
    serializer::{
        self, adapt::Adapt, state::State, Enumeration as _, List as _, Map as _, Serializer,
        Structure as _,
    },
};
use entia_core::each::{EachMut, EachRef};
use std::mem::size_of;

#[derive(Clone, Debug)]
pub enum Node {
    Unit,
    Bool(bool),
    Char(char),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Usize(usize),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Isize(isize),
    I128(i128),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Items),
    List(Items),
    Map(Pairs),
    Structure(Structure),
    Enumeration(Enumeration),
}

#[derive(Clone, Debug)]
pub struct Items(Vec<Node>);
#[derive(Clone, Debug)]
pub struct Pairs(Vec<(Node, Node)>);

#[derive(Clone, Debug)]
pub enum Structure {
    Unit,
    Tuple(Items),
    Map(Pairs),
}

#[derive(Clone, Debug)]
pub enum Enumeration {
    Never,
    Variant(String, usize, Structure),
}

impl From<Node> for Structure {
    fn from(node: Node) -> Self {
        match node {
            Node::Unit => Structure::Unit,
            Node::List(items) | Node::Array(items) => Structure::Tuple(items),
            Node::Map(pairs) => Structure::Map(pairs),
            Node::Structure(structure) => structure,
            Node::Enumeration(enumeration) => enumeration.into(),
            node => Structure::Tuple(Items(vec![node])),
        }
    }
}

impl From<Enumeration> for Structure {
    fn from(enumeration: Enumeration) -> Self {
        match enumeration {
            Enumeration::Never => Structure::Unit,
            Enumeration::Variant(_, _, structure) => structure,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Error {
    Invalid,
}

macro_rules! from {
    ($t:ident) => {
        impl From<Node> for $t {
            #[inline]
            fn from(node: Node) -> $t {
                node.$t().unwrap_or_default()
            }
        }
    };
    ($($t:ident),*) => { $(from!($t);)* }
}

macro_rules! number {
    ($t:ident) => {
        impl Node {
            #[inline]
            pub fn $t(&self) -> Option<$t> {
                match self {
                    Node::Unit
                    | Node::Structure(Structure::Unit)
                    | Node::Enumeration(Enumeration::Never)
                    | Node::Enumeration(Enumeration::Variant(_, _, Structure::Unit)) => Some($t::default()),
                    Node::Bool(value) => Some(*value as u8 as $t),
                    Node::Char(value) => Some(*value as u32 as $t),
                    Node::U8(value) => Some(*value as $t),
                    Node::U16(value) => Some(*value as $t),
                    Node::U32(value) => Some(*value as $t),
                    Node::U64(value) => Some(*value as $t),
                    Node::Usize(value) => Some(*value as $t),
                    Node::U128(value) => Some(*value as $t),
                    Node::I8(value) => Some(*value as $t),
                    Node::I16(value) => Some(*value as $t),
                    Node::I32(value) => Some(*value as $t),
                    Node::I64(value) => Some(*value as $t),
                    Node::Isize(value) => Some(*value as $t),
                    Node::I128(value) => Some(*value as $t),
                    Node::F32(value) => Some(*value as $t),
                    Node::F64(value) => Some(*value as $t),
                    Node::String(value) => value.parse().ok(),
                    Node::Bytes(value) => Some($t::from_ne_bytes(value[..size_of::<$t>()].try_into().ok()?)),
                    Node::List(Items(items))
                    | Node::Array(Items(items))
                    | Node::Structure(Structure::Tuple(Items(items)))
                    | Node::Enumeration(Enumeration::Variant(_, _, Structure::Tuple(Items(items)))) => items.iter().find_map(|node| node.$t()),
                    Node::Map(Pairs(pairs))
                    | Node::Structure(Structure::Map(Pairs(pairs)))
                    | Node::Enumeration(Enumeration::Variant(_, _, Structure::Map(Pairs(pairs)))) => pairs.iter().find_map(|pair| pair.1.$t()),
                }
            }
        }
    };
    ($($t:ident),*) => { $(number!($t);)* }
}

impl Node {
    #[inline]
    pub fn bool(&self) -> Option<bool> {
        match self {
            Node::Bool(value) => Some(*value),
            node => Some(node.u8()? > 0),
        }
    }

    #[inline]
    pub fn char(&self) -> Option<char> {
        match self {
            Node::Char(value) => Some(*value),
            node => char::from_u32(node.u32()?),
        }
    }
}

from!(char, bool, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
number!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);

impl Default for Node {
    #[inline]
    fn default() -> Self {
        Node::Unit
    }
}

pub mod serialize {
    use super::*;

    pub struct NodeSerializer;
    pub struct ListSerializer(Vec<Node>, fn(Items) -> Node);
    pub struct MapSerializer(Vec<(Node, Node)>, fn(Pairs) -> Node);

    impl Serialize for Node {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Value, S::Error> {
            match self {
                Node::Unit => serializer.unit(),
                Node::Bool(value) => serializer.bool(*value),
                Node::Char(value) => serializer.char(*value),
                Node::U8(value) => serializer.u8(*value),
                Node::U16(value) => serializer.u16(*value),
                Node::U32(value) => serializer.u32(*value),
                Node::U64(value) => serializer.u64(*value),
                Node::Usize(value) => serializer.usize(*value),
                Node::U128(value) => serializer.u128(*value),
                Node::I8(value) => serializer.i8(*value),
                Node::I16(value) => serializer.i16(*value),
                Node::I32(value) => serializer.i32(*value),
                Node::I64(value) => serializer.i64(*value),
                Node::Isize(value) => serializer.isize(*value),
                Node::I128(value) => serializer.i128(*value),
                Node::F32(value) => serializer.f32(*value),
                Node::F64(value) => serializer.f64(*value),
                Node::String(value) => serializer.string(value),
                Node::Bytes(value) => serializer.bytes(value),
                Node::Array(Items(_items)) => todo!(), // serializer.array(items),
                Node::List(Items(items)) => serializer.list()?.items(items),
                Node::Map(Pairs(pairs)) => {
                    serializer.map()?.pairs(pairs.iter().map(EachRef::each_ref))
                }
                Node::Enumeration(enumeration) => {
                    let serializer = serializer.enumeration()?;
                    match enumeration {
                        Enumeration::Never => serializer.never(),
                        Enumeration::Variant(name, index, structure) => {
                            let serializer = serializer.variant(name, *index)?;
                            match structure {
                                Structure::Unit => serializer.unit(),
                                Structure::Tuple(Items(items)) => serializer.tuple()?.items(items),
                                Structure::Map(Pairs(pairs)) => {
                                    serializer.map()?.pairs(pairs.iter().map(EachRef::each_ref))
                                }
                            }
                        }
                    }
                }
                Node::Structure(structure) => {
                    let serializer = serializer.structure()?;
                    match structure {
                        Structure::Unit => serializer.unit(),
                        Structure::Tuple(Items(items)) => serializer.tuple()?.items(items),
                        Structure::Map(Pairs(pairs)) => {
                            serializer.map()?.pairs(pairs.iter().map(EachRef::each_ref))
                        }
                    }
                }
            }
        }
    }

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

        fn bytes(self, value: &[u8]) -> Result<Self::Value, Self::Error> {
            Ok(Node::Bytes(value.iter().copied().collect()))
        }
        fn string(self, value: &str) -> Result<Self::Value, Self::Error> {
            Ok(Node::String(value.into()))
        }
        fn array<T: Serialize, const N: usize>(
            self,
            value: &[T; N],
        ) -> Result<Self::Value, Self::Error> {
            ListSerializer(Vec::with_capacity(N), Node::Array).items(value)
        }
        fn list(self) -> Result<Self::List, Self::Error> {
            Ok(ListSerializer(Vec::new(), Node::List))
        }
        fn map(self) -> Result<Self::Map, Self::Error> {
            Ok(MapSerializer(Vec::new(), Node::Map))
        }
        fn structure(self) -> Result<Self::Structure, Self::Error> {
            Ok(self)
        }
        fn enumeration(self) -> Result<Self::Enumeration, Self::Error> {
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
            Ok(self.1(Items(self.0)))
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
            Ok(self.1(Pairs(self.0)))
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
        type Structure = Adapt<State<Self, (String, usize)>, Self::Value, Self::Error>;

        fn never(self) -> Result<Self::Value, Self::Error> {
            Serializer::unit(self)
        }

        fn variant(self, name: &str, index: usize) -> Result<Self::Structure, Self::Error> {
            let adapt: fn(
                Result<(Self::Value, (String, usize)), Self::Error>,
            ) -> Result<Self::Value, Self::Error> = |result| {
                result.map(|(node, (name, index))| {
                    Node::Enumeration(Enumeration::Variant(name, index, node.into()))
                })
            };
            Ok(self.state((name.into(), index)).adapt(adapt))
        }
    }
}

pub mod deserialize {
    use super::*;

    pub struct NodeDeserializer(pub Node);
    pub struct ChildDeserializer(Node, usize);

    impl Default for NodeDeserializer {
        #[inline]
        fn default() -> Self {
            Self(Node::default())
        }
    }

    impl Deserialize for New<Node> {
        type Value = Node;

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            todo!()
        }
    }

    impl Deserialize for &mut Node {
        type Value = ();

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            match self {
                Node::Unit => deserializer.unit()?,
                Node::Bool(value) => value.deserialize(deserializer)?,
                Node::Char(value) => value.deserialize(deserializer)?,
                Node::U8(value) => value.deserialize(deserializer)?,
                Node::U16(value) => value.deserialize(deserializer)?,
                Node::U32(value) => value.deserialize(deserializer)?,
                Node::U64(value) => value.deserialize(deserializer)?,
                Node::Usize(value) => value.deserialize(deserializer)?,
                Node::U128(value) => value.deserialize(deserializer)?,
                Node::I8(value) => value.deserialize(deserializer)?,
                Node::I16(value) => value.deserialize(deserializer)?,
                Node::I32(value) => value.deserialize(deserializer)?,
                Node::I64(value) => value.deserialize(deserializer)?,
                Node::Isize(value) => value.deserialize(deserializer)?,
                Node::I128(value) => value.deserialize(deserializer)?,
                Node::F32(value) => value.deserialize(deserializer)?,
                Node::F64(value) => value.deserialize(deserializer)?,
                Node::String(value) => value.deserialize(deserializer)?,
                Node::Bytes(value) => value.deserialize(deserializer)?,
                Node::Array(Items(items)) => items.deserialize(deserializer)?,
                Node::List(Items(items)) => deserializer.list()?.items(items)?,
                Node::Map(Pairs(pairs)) => deserializer
                    .map()?
                    .pairs(&mut Node::Unit, pairs.iter_mut().map(EachMut::each_mut))?,
                Node::Enumeration(enumeration) => {
                    let deserializer = deserializer.enumeration()?;
                    match enumeration {
                        Enumeration::Never => Err(deserializer.never())?,
                        Enumeration::Variant(name, index, structure) => {
                            // TODO: The variant key seems wrong...
                            let (_, deserializer) = deserializer.variant(&mut name.clone())?;
                            match structure {
                                Structure::Unit => deserializer.unit(name, *index)?,
                                Structure::Tuple(Items(items)) => {
                                    deserializer.tuple(name, *index)?.items(items)?
                                }
                                Structure::Map(Pairs(pairs)) => {
                                    deserializer.map(name, *index)?.pairs(
                                        &mut Node::Unit,
                                        pairs.iter_mut().map(EachMut::each_mut),
                                    )?
                                }
                            }
                        }
                    }
                }
                Node::Structure(structure) => {
                    let deserializer = deserializer.structure()?;
                    match structure {
                        Structure::Unit => deserializer.unit()?,
                        Structure::Tuple(Items(items)) => deserializer.tuple()?.items(items)?,
                        Structure::Map(Pairs(pairs)) => deserializer
                            .map()?
                            .pairs(&mut Node::Unit, pairs.iter_mut().map(EachMut::each_mut))?,
                    }
                }
            }
            Ok(())
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
            Ok(self.0.into())
        }
        #[inline]
        fn char(self) -> Result<char, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn u8(self) -> Result<u8, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn u16(self) -> Result<u16, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn u32(self) -> Result<u32, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn u64(self) -> Result<u64, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn u128(self) -> Result<u128, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn usize(self) -> Result<usize, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn i8(self) -> Result<i8, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn i16(self) -> Result<i16, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn i32(self) -> Result<i32, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn i64(self) -> Result<i64, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn i128(self) -> Result<i128, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn isize(self) -> Result<isize, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn f32(self) -> Result<f32, Self::Error> {
            Ok(self.0.into())
        }
        #[inline]
        fn f64(self) -> Result<f64, Self::Error> {
            Ok(self.0.into())
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
        fn structure(self) -> Result<Self::Structure, Self::Error> {
            Ok(self)
        }
        #[inline]
        fn enumeration(self) -> Result<Self::Enumeration, Self::Error> {
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
        fn tuple(self) -> Result<Self::List, Self::Error> {
            Deserializer::list(self)
        }
        fn map(self) -> Result<Self::Map, Self::Error> {
            Deserializer::map(self)
        }
    }

    impl deserializer::Enumeration for NodeDeserializer {
        type Error = Error;
        type Variant = Self;

        fn never(self) -> Self::Error {
            Error::Invalid
        }

        fn variant<K: Deserialize>(self, key: K) -> Result<(K::Value, Self::Variant), Self::Error> {
            match self.0 {
                Node::Enumeration(Enumeration::Variant(name, _, structure)) => {
                    let key = key.deserialize(NodeDeserializer(Node::String(name)))?;
                    Ok((key, NodeDeserializer(Node::Structure(structure))))
                }
                _ => Err(Error::Invalid),
            }
        }
    }

    impl deserializer::Variant for NodeDeserializer {
        type Error = Error;
        type Map = ChildDeserializer;
        type List = ChildDeserializer;

        fn unit(self, _: &str, _: usize) -> Result<(), Self::Error> {
            Deserializer::unit(self)
        }
        fn map(self, _: &str, _: usize) -> Result<Self::Map, Self::Error> {
            Deserializer::map(self)
        }
        fn tuple(self, _: &str, _: usize) -> Result<Self::List, Self::Error> {
            Deserializer::list(self)
        }
        fn miss<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
            value.deserialize(NodeDeserializer::default())
        }
    }

    impl deserializer::List for ChildDeserializer {
        type Error = Error;
        type Item = NodeDeserializer;

        fn item(&mut self) -> Result<Option<Self::Item>, Self::Error> {
            let item = match &self.0 {
                Node::List(Items(nodes))
                | Node::Array(Items(nodes))
                | Node::Structure(Structure::Tuple(Items(nodes)))
                | Node::Enumeration(Enumeration::Variant(_, _, Structure::Tuple(Items(nodes)))) => {
                    nodes.get(self.1).map(|node| NodeDeserializer(node.clone()))
                }
                Node::Map(Pairs(nodes))
                | Node::Structure(Structure::Map(Pairs(nodes)))
                | Node::Enumeration(Enumeration::Variant(_, _, Structure::Map(Pairs(nodes)))) => {
                    nodes
                        .get(self.1)
                        .map(|node| NodeDeserializer(node.1.clone()))
                }
                Node::Bytes(value) => value
                    .get(self.1)
                    .map(|&value| NodeDeserializer(Node::U8(value))),
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
        type Item = NodeDeserializer;

        fn pair<K: Deserialize>(
            &mut self,
            key: K,
        ) -> Result<Option<(K::Value, Self::Item)>, Self::Error> {
            let pair = match &self.0 {
                Node::List(Items(nodes))
                | Node::Array(Items(nodes))
                | Node::Structure(Structure::Tuple(Items(nodes)))
                | Node::Enumeration(Enumeration::Variant(_, _, Structure::Tuple(Items(nodes)))) => {
                    match nodes.get(self.1) {
                        Some(node) => Some((
                            key.deserialize(NodeDeserializer(Node::Usize(self.1)))?,
                            NodeDeserializer(node.clone()),
                        )),
                        None => None,
                    }
                }
                Node::Map(Pairs(nodes))
                | Node::Structure(Structure::Map(Pairs(nodes)))
                | Node::Enumeration(Enumeration::Variant(_, _, Structure::Map(Pairs(nodes)))) => {
                    match nodes.get(self.1) {
                        Some(pair) => Some((
                            key.deserialize(NodeDeserializer(pair.0.clone()))?,
                            NodeDeserializer(pair.1.clone()),
                        )),
                        None => None,
                    }
                }
                Node::Bytes(value) => match value.get(self.1) {
                    Some(&value) => Some((
                        key.deserialize(NodeDeserializer(Node::Usize(self.1)))?,
                        NodeDeserializer(Node::U8(value)),
                    )),
                    None => None,
                },
                node if self.1 == 0 => Some((
                    key.deserialize(NodeDeserializer(Node::Usize(self.1)))?,
                    NodeDeserializer(node.clone()),
                )),
                _ => None,
            };
            self.1 += 1;
            Ok(pair)
        }

        fn miss<V: Deserialize>(&mut self, value: V) -> Result<V::Value, Self::Error> {
            value.deserialize(NodeDeserializer::default())
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
            serializer.structure()?.tuple()?.item(self.0)?.end()
        }
    }

    impl Deserialize for New<Fett> {
        type Value = Fett;

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            use deserializer::*;
            let mut list = deserializer.structure()?.tuple()?;
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
    fn u8_to_f32() -> Result<(), Error> {
        1u8.convert(&mut 0f32)
    }

    #[test]
    fn boba_to_fett() -> Result<(), Error> {
        Boba(true).convert(New::<Fett>::new()).map(|_| ())
    }
}

pub mod convert {
    use super::*;

    pub trait Convert<T>: Serialize {
        type Value;
        type Error;
        fn convert(self, to: T) -> Result<Self::Value, Self::Error>;
    }

    impl<S: Serialize, D: Deserialize> Convert<D> for S {
        type Value = D::Value;
        type Error = Error;

        fn convert(self, to: D) -> Result<Self::Value, Self::Error> {
            let node = self.serialize(NodeSerializer)?;
            to.deserialize(NodeDeserializer(node))
        }
    }
}

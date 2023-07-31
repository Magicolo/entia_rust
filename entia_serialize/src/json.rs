use std::str::{self};

pub struct Boba {
    pub a: bool,
    pub b: usize,
    pub c: Vec<bool>,
}

pub enum Fett {
    A(bool),
    B(usize),
    C(Vec<bool>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Null,
    Boolean(bool),
    Integer(isize),
    Number(f64),
    String(String),
    Array(Vec<Node>),
    Object(Vec<(Node, Node)>),
}

impl Node {
    #[inline]
    pub fn boolean(&self) -> Option<bool> {
        match self {
            Node::Null => Some(false),
            Node::Boolean(value) => Some(*value),
            Node::Integer(value) => Some(*value != 0),
            Node::Number(value) => Some(*value != 0.),
            Node::String(value) if value.eq_ignore_ascii_case("true") => Some(true),
            Node::String(value) if value.eq_ignore_ascii_case("false") => Some(false),
            _ => None,
        }
    }

    #[inline]
    pub fn string(&self) -> Option<&str> {
        match self {
            Node::Null => Some(""),
            Node::Boolean(true) => Some("true"),
            Node::Boolean(false) => Some("false"),
            Node::String(value) => Some(value),
            _ => None,
        }
    }

    #[inline]
    pub fn character(&self) -> Option<char> {
        match self {
            Node::String(value) => value.chars().next(),
            _ => char::from_u32(self.integer()? as u32),
        }
    }

    #[inline]
    pub fn integer(&self) -> Option<isize> {
        match self {
            Node::Null => Some(0),
            Node::Boolean(true) => Some(1),
            Node::Boolean(false) => Some(0),
            Node::Integer(value) => Some(*value),
            Node::Number(value) => Some(*value as isize),
            Node::String(value) => value.parse().ok(),
            _ => None,
        }
    }

    #[inline]
    pub fn number(&self) -> Option<f64> {
        match self {
            Node::Null => Some(0.),
            Node::Boolean(true) => Some(1.),
            Node::Boolean(false) => Some(0.),
            Node::Integer(value) => Some(*value as f64),
            Node::Number(value) => Some(*value),
            Node::String(value) => value.parse().ok(),
            _ => None,
        }
    }
}

pub mod node {
    use super::*;
    use crate::deserialize::*;
    use crate::deserializer::*;

    pub struct NodeDeserializer<'a>(&'a Node);
    pub struct MapDeserializer<'a>(&'a [(Node, Node)], usize);
    pub struct ListDeserializer<'a>(&'a [Node], usize);

    pub enum Error {
        Never,
        ExpectedArrayNode,
        ExpectedObjectNode,
    }

    impl NodeDeserializer<'_> {
        #[inline]
        pub fn deserialize<T>(node: &Node) -> Result<<New<T> as Deserialize>::Value, Error>
        where
            New<T>: Deserialize,
        {
            New::<T>::new().deserialize(NodeDeserializer(node))
        }
    }

    impl<'a> Deserializer for NodeDeserializer<'a> {
        type Error = Error;
        type Structure = Self;
        type Enumeration = Self;
        type List = ListDeserializer<'a>;
        type Map = MapDeserializer<'a>;

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
            Ok(self.0.number().unwrap_or_default())
        }
        #[inline]
        fn list(self) -> Result<Self::List, Self::Error> {
            match self.0 {
                Node::Array(nodes) => Ok(ListDeserializer(nodes, 0)),
                _ => Err(Error::ExpectedArrayNode),
            }
        }
        #[inline]
        fn map(self) -> Result<Self::Map, Self::Error> {
            match self.0 {
                Node::Object(pairs) => Ok(MapDeserializer(pairs, 0)),
                _ => Err(Error::ExpectedObjectNode),
            }
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

    impl<'a> Structure for NodeDeserializer<'a> {
        type Error = Error;
        type List = ListDeserializer<'a>;
        type Map = MapDeserializer<'a>;

        #[inline]
        fn unit(self) -> Result<(), Self::Error> {
            Ok(())
        }
        #[inline]
        fn tuple(self) -> Result<Self::List, Self::Error> {
            Deserializer::list(self)
        }
        #[inline]
        fn map(self) -> Result<Self::Map, Self::Error> {
            Deserializer::map(self)
        }
    }

    impl Enumeration for NodeDeserializer<'_> {
        type Error = Error;
        type Variant = Self;

        #[inline]
        fn never(self) -> Self::Error {
            Error::Never
        }

        #[inline]
        fn variant<K: Deserialize>(self, key: K) -> Result<(K::Value, Self::Variant), Self::Error> {
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
        type Map = MapDeserializer<'a>;
        type List = ListDeserializer<'a>;

        #[inline]
        fn unit(self, _: &str, _: usize) -> Result<(), Self::Error> {
            Deserializer::unit(self)
        }
        #[inline]
        fn map(self, _: &str, _: usize) -> Result<Self::Map, Self::Error> {
            Deserializer::map(self)
        }
        #[inline]
        fn tuple(self, _: &str, _: usize) -> Result<Self::List, Self::Error> {
            Deserializer::list(self)
        }
        #[inline]
        fn miss<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
            value.deserialize(self)
        }
    }

    impl<'a> Map for MapDeserializer<'a> {
        type Error = Error;
        type Item = NodeDeserializer<'a>;

        fn pair<K: Deserialize>(
            &mut self,
            key: K,
        ) -> Result<Option<(K::Value, Self::Item)>, Self::Error> {
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

        fn miss<V: Deserialize>(&mut self, value: V) -> Result<V::Value, Self::Error> {
            value.deserialize(NodeDeserializer(&Node::Null))
        }
    }

    impl<'a> List for ListDeserializer<'a> {
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
}

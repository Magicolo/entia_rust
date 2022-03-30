use std::str::{self};

pub struct Boba {
    a: bool,
    b: usize,
    c: Vec<bool>,
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
    Floating(f64),
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
            Node::Floating(value) => Some(*value != 0.),
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
            Node::Floating(value) => Some(*value as isize),
            Node::String(value) => value.parse().ok(),
            _ => None,
        }
    }

    #[inline]
    pub fn floating(&self) -> Option<f64> {
        match self {
            Node::Null => Some(0.),
            Node::Boolean(true) => Some(1.),
            Node::Boolean(false) => Some(0.),
            Node::Integer(value) => Some(*value as f64),
            Node::Floating(value) => Some(*value),
            Node::String(value) => value.parse().ok(),
            _ => None,
        }
    }
}

mod node {
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
            Ok(self.0.floating().unwrap_or_default())
        }
        #[inline]
        fn list(self) -> Result<Self::List, Self::Error> {
            todo!()
        }
        #[inline]
        fn map(self) -> Result<Self::Map, Self::Error> {
            todo!()
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

        fn unit(self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn tuple(self) -> Result<Self::List, Self::Error> {
            match self.0 {
                Node::Array(nodes) => Ok(ListDeserializer(nodes, 0)),
                _ => Err(Error::ExpectedArrayNode),
            }
        }

        fn map(self) -> Result<Self::Map, Self::Error> {
            match self.0 {
                Node::Object(pairs) => Ok(MapDeserializer(pairs, 0)),
                _ => Err(Error::ExpectedObjectNode),
            }
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

        fn unit(self, name: &str, index: usize) -> Result<(), Self::Error> {
            todo!()
        }

        fn map(self, name: &str, index: usize) -> Result<Self::Map, Self::Error> {
            todo!()
        }

        fn tuple(self, name: &str, index: usize) -> Result<Self::List, Self::Error> {
            todo!()
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

// impl Visit for Node {
//     #[inline]
//     fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
//         match self {
//             Node::Null => visitor.primitive().unit(),
//             Node::Boolean(value) => visitor.primitive().bool(*value),
//             Node::Integer(value) => visitor.primitive().isize(*value),
//             Node::Floating(value) => visitor.primitive().f64(*value),
//             Node::String(value) => visitor.sequence().string(value),
//             Node::Array(nodes) => visitor.sequence().items(nodes),
//             Node::Object(nodes) => visitor
//                 .sequence()
//                 .fields(nodes.iter().map(|(key, value)| (key, value))),
//         }
//     }
// }

// mod poulah_serialize {
//     use super::*;

//     // TODO: Remove this and implement 'Serialize' directly? What other wrapper type could be relevant here?
//     pub struct Source<'a, T: ?Sized>(&'a T); // TODO: impl Deref?

//     pub trait Serialize {
//         fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error>;
//     }

//     pub trait Serializer {
//         type Value;
//         type Error;
//         type Primitive: Primitive<Value = Self::Value, Error = Self::Error>;
//         type Structure: Structure<Value = Self::Value, Error = Self::Error>;

//         fn primitive(self) -> Self::Primitive;
//         fn structure(self, name: &'static str) -> Self::Structure;
//     }

//     pub trait Primitive {
//         type Value;
//         type Error;
//         fn bool(self, value: bool) -> Result<Self::Value, Self::Error>;
//     }

//     pub trait Structure {
//         type Value;
//         type Error;
//         type Fields: Fields<Error = Self::Error> + Into<Result<Self::Value, Self::Error>>;
//         fn map<const N: usize>(self) -> Result<Self::Fields, Self::Error>;
//     }

//     pub trait Fields: Sized {
//         type Error;
//         fn field<K: Serialize, V: Serialize>(
//             &mut self,
//             key: K,
//             value: V,
//         ) -> Result<(), Self::Error>;
//     }

//     impl<'a, T: ?Sized> Serialize for &'a T
//     where
//         Source<'a, T>: Serialize,
//     {
//         #[inline]
//         fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
//             Source(self).serialize(serializer)
//         }
//     }

//     impl<'a, T: ?Sized> Serialize for &'a mut T
//     where
//         &'a T: Serialize,
//     {
//         #[inline]
//         fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
//             (&*self).serialize(serializer)
//         }
//     }

//     impl<'a, T: ?Sized> Serialize for Source<'a, &T>
//     where
//         Source<'a, T>: Serialize,
//     {
//         fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
//             Source(&**self.0).serialize(serializer)
//         }
//     }

//     impl<'a, T: ?Sized> Serialize for Source<'a, &mut T>
//     where
//         Source<'a, T>: Serialize,
//     {
//         fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
//             Source(&**self.0).serialize(serializer)
//         }
//     }

//     impl Serialize for Source<'_, Boba> {
//         fn serialize<'a, S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
//             let Boba { a, b, c } = self.0;
//             let mut fields = serializer.structure("Boba").map::<3>()?;
//             fields.field("a", Source(a))?;
//             fields.field("b", Source(b))?;
//             fields.field("c", Source(c))?;
//             fields.into()
//         }
//     }

//     impl Serialize for Source<'_, str> {
//         fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
//             todo!()
//         }
//     }

//     impl Serialize for Source<'_, bool> {
//         fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
//             serializer.primitive().bool(*self.0)
//         }
//     }

//     impl Serialize for Source<'_, usize> {
//         fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
//             todo!()
//         }
//     }

//     impl<'a, T> Serialize for Source<'a, Vec<T>>
//     where
//         Source<'a, T>: Serialize,
//     {
//         fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
//             todo!()
//         }
//     }
// }

// mod deserialize {

//     pub struct NodeDeserializer<'a>(&'a Node);
//     pub struct MapDeserializer<'a>(&'a [(Node, Node)], usize);
//     pub struct ListDeserializer<'a>(&'a [Node], usize);

//     pub enum Error {
//         ExpectedObjectNode,
//     }

//     impl NodeDeserializer<'_> {
//         pub fn deserialize<T>(node: &Node) -> Result<<New<T> as Deserialize>::Value, Error>
//         where
//             New<T>: Deserialize,
//         {
//             New::<T>::new().deserialize(NodeDeserializer(node))
//         }
//     }

//     impl<'a> Deserializer for NodeDeserializer<'a> {
//         type Error = Error;
//         type Structure = Self;
//         type Enumeration = Self;
//         type List = ListDeserializer<'a>;
//         type Map = MapDeserializer<'a>;

//         #[inline]
//         fn unit(self) -> Result<(), Self::Error> {
//             Ok(())
//         }
//         #[inline]
//         fn bool(self) -> Result<bool, Self::Error> {
//             Ok(self.0.boolean().unwrap_or_default())
//         }
//         #[inline]
//         fn char(self) -> Result<char, Self::Error> {
//             Ok(self.0.character().unwrap_or_default())
//         }
//         #[inline]
//         fn u8(self) -> Result<u8, Self::Error> {
//             Ok(self.isize()? as _)
//         }
//         #[inline]
//         fn u16(self) -> Result<u16, Self::Error> {
//             Ok(self.isize()? as _)
//         }
//         #[inline]
//         fn u32(self) -> Result<u32, Self::Error> {
//             Ok(self.isize()? as _)
//         }
//         #[inline]
//         fn u64(self) -> Result<u64, Self::Error> {
//             Ok(self.isize()? as _)
//         }
//         #[inline]
//         fn u128(self) -> Result<u128, Self::Error> {
//             Ok(self.isize()? as _)
//         }
//         #[inline]
//         fn usize(self) -> Result<usize, Self::Error> {
//             Ok(self.isize()? as _)
//         }
//         #[inline]
//         fn i8(self) -> Result<i8, Self::Error> {
//             Ok(self.isize()? as _)
//         }
//         #[inline]
//         fn i16(self) -> Result<i16, Self::Error> {
//             Ok(self.isize()? as _)
//         }
//         #[inline]
//         fn i32(self) -> Result<i32, Self::Error> {
//             Ok(self.isize()? as _)
//         }
//         #[inline]
//         fn i64(self) -> Result<i64, Self::Error> {
//             Ok(self.isize()? as _)
//         }
//         #[inline]
//         fn i128(self) -> Result<i128, Self::Error> {
//             Ok(self.isize()? as _)
//         }
//         #[inline]
//         fn isize(self) -> Result<isize, Self::Error> {
//             Ok(self.0.integer().unwrap_or_default())
//         }
//         #[inline]
//         fn f32(self) -> Result<f32, Self::Error> {
//             Ok(self.f64()? as _)
//         }
//         #[inline]
//         fn f64(self) -> Result<f64, Self::Error> {
//             Ok(self.0.floating().unwrap_or_default())
//         }
//         #[inline]
//         fn list(self) -> Result<Self::List, Self::Error> {
//             todo!()
//         }
//         #[inline]
//         fn map(self) -> Result<Self::Map, Self::Error> {
//             todo!()
//         }
//         #[inline]
//         fn structure<T: ?Sized>(self) -> Result<Self::Structure, Self::Error> {
//             Ok(self)
//         }
//         #[inline]
//         fn enumeration<T: ?Sized>(self) -> Result<Self::Enumeration, Self::Error> {
//             Ok(self)
//         }
//     }

//     impl<'a> Structure for NodeDeserializer<'a> {
//         type Error = Error;
//         type Fields = MapDeserializer<'a>;

//         fn map<const N: usize>(self) -> Result<Self::Fields, Self::Error> {
//             match self.0 {
//                 Node::Object(pairs) => Ok(MapDeserializer(pairs, 0)),
//                 _ => Err(Error::ExpectedObjectNode),
//             }
//         }
//     }

//     impl Enumeration for NodeDeserializer<'_> {
//         type Error = Error;
//         type Variant = Self;

//         // TODO: Is this right?
//         #[inline]
//         fn never<K: Deserialize>(self, key: K) -> Result<K::Value, Self::Error> {
//             key.deserialize(self)
//         }

//         #[inline]
//         fn variant<K: Deserialize, const N: usize>(
//             self,
//             key: K,
//         ) -> Result<(K::Value, Self::Variant), Self::Error> {
//             match self.0 {
//                 Node::Object(pairs) if pairs.len() > 0 => {
//                     let pair = &pairs[0];
//                     let key = key.deserialize(NodeDeserializer(&pair.0))?;
//                     Ok((key, self))
//                 }
//                 _ => Err(Error::ExpectedObjectNode),
//             }
//         }
//     }

//     impl<'a> Variant for NodeDeserializer<'a> {
//         type Error = Error;
//         type Fields = MapDeserializer<'a>;
//         type Items = ListDeserializer<'a>;

//         fn unit(self, _: &'static str, _: usize) -> Result<(), Self::Error> {
//             todo!()
//         }

//         fn map<const N: usize>(
//             self,
//             _: &'static str,
//             _: usize,
//         ) -> Result<Self::Fields, Self::Error> {
//             todo!()
//         }

//         fn tuple<const N: usize>(
//             self,
//             _: &'static str,
//             _: usize,
//         ) -> Result<Self::Items, Self::Error> {
//             todo!()
//         }

//         #[inline]
//         fn excess<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
//             value.deserialize(self)
//         }
//     }

//     impl<'a> Map for MapDeserializer<'a> {
//         type Error = Error;
//         type Field = NodeDeserializer<'a>;

//         fn field<K: Deserialize>(
//             &mut self,
//             key: K,
//         ) -> Result<Option<(K::Value, Self::Field)>, Self::Error> {
//             match self.0.get(self.1) {
//                 Some(pair) => {
//                     self.1 += 1;
//                     let key = key.deserialize(NodeDeserializer(&pair.0))?;
//                     let field = NodeDeserializer(&pair.1);
//                     Ok(Some((key, field)))
//                 }
//                 None => Ok(None),
//             }
//         }

//         fn miss<K, V: Deserialize>(&mut self, _: K, value: V) -> Result<V::Value, Self::Error> {
//             value.deserialize(NodeDeserializer(&Node::Null))
//         }
//     }

//     impl Field for NodeDeserializer<'_> {
//         type Error = Error;

//         fn value<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
//             value.deserialize(self)
//         }

//         fn excess(self) -> Result<(), Self::Error> {
//             Ok(())
//         }
//     }

//     impl<'a> List for ListDeserializer<'a> {
//         type Error = Error;
//         type Item = NodeDeserializer<'a>;

//         fn item(&mut self) -> Result<Option<Self::Item>, Self::Error> {
//             match self.0.get(self.1) {
//                 Some(node) => {
//                     self.1 += 1;
//                     Ok(Some(NodeDeserializer(node)))
//                 }
//                 None => Ok(None),
//             }
//         }

//         fn miss<V: Deserialize>(&mut self, value: V) -> Result<V::Value, Self::Error> {
//             value.deserialize(NodeDeserializer(&Node::Null))
//         }
//     }

//     impl Item for NodeDeserializer<'_> {
//         type Error = Error;

//         fn value<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
//             value.deserialize(self)
//         }

//         fn excess(self) -> Result<(), Self::Error> {
//             Ok(())
//         }
//     }
// }

// mod json {
//     use super::*;

//     pub struct Serializer(String);
//     pub struct ScopeSerializer<const O: char, const C: char>(String, bool);
//     pub struct VariantSerializer<const O: char, const C: char>(ScopeSerializer<O, C>);

//     impl Visitor for Serializer {
//         type Result = String;
//         type Primitive = Self;
//         type Structure = Self;
//         type Enumeration = Self;
//         type Sequence = Self;

//         #[inline]
//         fn primitive(self) -> Self::Primitive {
//             self
//         }
//         #[inline]
//         fn structure(self, _: &'static str) -> Self::Structure {
//             self
//         }
//         #[inline]
//         fn enumeration(self, _: &'static str) -> Self::Enumeration {
//             self
//         }
//         #[inline]
//         fn sequence(self) -> Self::Sequence {
//             self
//         }
//     }

//     impl Structure for Serializer {
//         type Result = String;
//         type Fields = ScopeSerializer<'{', '}'>;
//         type Items = ScopeSerializer<'[', ']'>;

//         #[inline]
//         fn unit(self) -> Self::Result {
//             Primitive::unit(self)
//         }
//         #[inline]
//         fn tuple<const N: usize>(self) -> Self::Items {
//             Sequence::list(self, N)
//         }
//         #[inline]
//         fn map<const N: usize>(self) -> Self::Fields {
//             Sequence::map(self, N)
//         }
//     }

//     impl Sequence for Serializer {
//         type Result = String;
//         type Fields = ScopeSerializer<'{', '}'>;
//         type Items = ScopeSerializer<'[', ']'>;

//         #[inline]
//         fn list(self, _: usize) -> Self::Items {
//             ScopeSerializer::new(self.0)
//         }

//         #[inline]
//         fn map(self, _: usize) -> Self::Fields {
//             ScopeSerializer::new(self.0)
//         }

//         #[inline]
//         fn string(mut self, value: &str) -> Self::Result {
//             self.0.push('"');
//             self.0.push_str(value);
//             self.0.push('"');
//             self.0
//         }

//         #[inline]
//         fn bytes(self, value: &[u8]) -> Self::Result {
//             match str::from_utf8(value) {
//                 Ok(string) => self.string(string),
//                 Err(_) => self.slice(value),
//             }
//         }
//     }

//     impl Primitive for Serializer {
//         type Result = String;

//         #[inline]
//         fn unit(mut self) -> Self::Result {
//             self.0.push_str("null");
//             self.0
//         }
//         #[inline]
//         fn never(self) -> Self::Result {
//             Primitive::unit(self)
//         }
//         #[inline]
//         fn bool(mut self, value: bool) -> Self::Result {
//             self.0.push_str(if value { "true" } else { "false" });
//             self.0
//         }
//         #[inline]
//         fn char(mut self, value: char) -> Self::Result {
//             self.0.push(value);
//             self.0
//         }
//         #[inline]
//         fn u8(mut self, value: u8) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn u16(mut self, value: u16) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn u32(mut self, value: u32) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn u64(mut self, value: u64) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn u128(mut self, value: u128) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn usize(mut self, value: usize) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn i8(mut self, value: i8) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn i16(mut self, value: i16) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn i32(mut self, value: i32) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn i64(mut self, value: i64) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn i128(mut self, value: i128) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn isize(mut self, value: isize) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn f32(mut self, value: f32) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn f64(mut self, value: f64) -> Self::Result {
//             write!(&mut self.0, "{}", value).unwrap();
//             self.0
//         }
//         #[inline]
//         fn shared<T: ?Sized>(self, _: &T) -> Self::Result {
//             Primitive::unit(self)
//         }
//         #[inline]
//         fn constant<T: ?Sized>(self, _: *const T) -> Self::Result {
//             Primitive::unit(self)
//         }
//     }

//     impl Enumeration for Serializer {
//         type Result = String;
//         type Variant = Self;

//         #[inline]
//         fn never(self) -> Self::Result {
//             Primitive::unit(self)
//         }

//         #[inline]
//         fn variant<const N: usize>(self) -> Self::Variant {
//             self
//         }
//     }

//     impl Variant for Serializer {
//         type Result = String;
//         type Fields = VariantSerializer<'{', '}'>;
//         type Items = VariantSerializer<'[', ']'>;

//         #[inline]
//         fn unit(mut self, name: &'static str, _: usize) -> Self::Result {
//             self.0.push_str(name);
//             self.0
//         }

//         #[inline]
//         fn tuple<const N: usize>(mut self, name: &'static str, _: usize) -> Self::Items {
//             self.0.push('{');
//             self.0.push_str(name);
//             self.0.push(':');
//             VariantSerializer(Sequence::list(self, N))
//         }

//         #[inline]
//         fn map<const N: usize>(mut self, name: &'static str, _: usize) -> Self::Fields {
//             self.0.push('{');
//             self.0.push_str(name);
//             self.0.push(':');
//             VariantSerializer(Sequence::map(self, N))
//         }
//     }

//     impl<const O: char, const C: char> Fields for VariantSerializer<O, C> {
//         #[inline]
//         fn field<K: Visit, V: Visit>(self, key: K, value: V) -> Self {
//             Self(self.0.field(key, value))
//         }
//     }

//     impl<const O: char, const C: char> Items for VariantSerializer<O, C> {
//         #[inline]
//         fn item<T: Visit>(self, value: T) -> Self {
//             Self(self.0.item(value))
//         }
//     }

//     impl<const O: char, const C: char> Into<String> for VariantSerializer<O, C> {
//         #[inline]
//         fn into(self) -> String {
//             let mut buffer: String = self.0.into();
//             buffer.push('}');
//             buffer
//         }
//     }

//     impl<const O: char, const C: char> Into<String> for ScopeSerializer<O, C> {
//         #[inline]
//         fn into(self) -> String {
//             let mut buffer = self.0;
//             buffer.push(C);
//             buffer
//         }
//     }

//     impl<const O: char, const C: char> ScopeSerializer<O, C> {
//         pub fn new(mut buffer: String) -> Self {
//             buffer.push(O);
//             Self(buffer, false)
//         }
//     }

//     impl<const O: char, const C: char> Fields for ScopeSerializer<O, C> {
//         fn field<K: Visit, V: Visit>(mut self, key: K, value: V) -> Self {
//             if self.1 {
//                 self.0.push(',');
//             }
//             let mut buffer = key.visit(Serializer(self.0));
//             buffer.push(':');
//             Self(value.visit(Serializer(buffer)), true)
//         }
//     }

//     impl<const O: char, const C: char> Items for ScopeSerializer<O, C> {
//         fn item<T: Visit>(mut self, value: T) -> Self {
//             if self.1 {
//                 self.0.push(',');
//             }
//             Self(value.visit(Serializer(self.0)), true)
//         }
//     }
// }

// mod serialize {
//     use super::*;

//     pub struct Serializer;
//     pub struct FieldsSerializer(Vec<(Node, Node)>);
//     pub struct ItemsSerializer(Vec<Node>);
//     pub struct VariantSerializer<S>(&'static str, S);

//     pub trait Serialize<T> {
//         fn serialize(&self) -> T;
//     }

//     impl<T: Visit> Serialize<Node> for T {
//         #[inline]
//         fn serialize(&self) -> Node {
//             self.visit(Serializer)
//         }
//     }

//     impl Visitor for Serializer {
//         type Result = Node;
//         type Primitive = Self;
//         type Structure = Self;
//         type Enumeration = Self;
//         type Sequence = Self;

//         #[inline]
//         fn primitive(self) -> Self::Primitive {
//             self
//         }
//         #[inline]
//         fn structure(self, _: &'static str) -> Self::Structure {
//             self
//         }
//         #[inline]
//         fn enumeration(self, _: &'static str) -> Self::Enumeration {
//             self
//         }
//         #[inline]
//         fn sequence(self) -> Self::Sequence {
//             self
//         }
//     }

//     impl Structure for Serializer {
//         type Result = Node;
//         type Fields = FieldsSerializer;
//         type Items = ItemsSerializer;

//         #[inline]
//         fn unit(self) -> Self::Result {
//             Primitive::unit(self)
//         }

//         #[inline]
//         fn tuple<const N: usize>(self) -> Self::Items {
//             Sequence::list(self, N)
//         }

//         #[inline]
//         fn map<const N: usize>(self) -> Self::Fields {
//             Sequence::map(self, N)
//         }
//     }

//     impl Enumeration for Serializer {
//         type Result = Node;
//         type Variant = Self;

//         #[inline]
//         fn never(self) -> Self::Result {
//             Primitive::unit(self)
//         }
//         #[inline]
//         fn variant<const N: usize>(self) -> Self::Variant {
//             self
//         }
//     }

//     impl Sequence for Serializer {
//         type Result = Node;
//         type Fields = FieldsSerializer;
//         type Items = ItemsSerializer;

//         #[inline]
//         fn list(self, capacity: usize) -> Self::Items {
//             ItemsSerializer(Vec::with_capacity(capacity))
//         }

//         #[inline]
//         fn map(self, capacity: usize) -> Self::Fields {
//             FieldsSerializer(Vec::with_capacity(capacity))
//         }

//         #[inline]
//         fn string(self, value: &str) -> Self::Result {
//             Node::String(value.into())
//         }

//         #[inline]
//         fn bytes(self, value: &[u8]) -> Self::Result {
//             match str::from_utf8(value) {
//                 Ok(value) => self.string(value),
//                 Err(_) => self.slice(value),
//             }
//         }
//     }

//     impl Primitive for Serializer {
//         type Result = Node;

//         #[inline]
//         fn unit(self) -> Self::Result {
//             Node::Null
//         }
//         #[inline]
//         fn never(self) -> Self::Result {
//             Primitive::unit(self)
//         }
//         #[inline]
//         fn bool(self, value: bool) -> Self::Result {
//             Node::Boolean(value)
//         }
//         #[inline]
//         fn char(self, value: char) -> Self::Result {
//             Node::String(value.into())
//         }
//         #[inline]
//         fn u8(self, value: u8) -> Self::Result {
//             self.isize(value as _)
//         }
//         #[inline]
//         fn u16(self, value: u16) -> Self::Result {
//             self.isize(value as _)
//         }
//         #[inline]
//         fn u32(self, value: u32) -> Self::Result {
//             self.isize(value as _)
//         }
//         #[inline]
//         fn u64(self, value: u64) -> Self::Result {
//             self.isize(value as _)
//         }
//         #[inline]
//         fn u128(self, value: u128) -> Self::Result {
//             self.isize(value as _)
//         }
//         #[inline]
//         fn usize(self, value: usize) -> Self::Result {
//             self.isize(value as _)
//         }
//         #[inline]
//         fn i8(self, value: i8) -> Self::Result {
//             self.isize(value as _)
//         }
//         #[inline]
//         fn i16(self, value: i16) -> Self::Result {
//             self.isize(value as _)
//         }
//         #[inline]
//         fn i32(self, value: i32) -> Self::Result {
//             self.isize(value as _)
//         }
//         #[inline]
//         fn i64(self, value: i64) -> Self::Result {
//             self.isize(value as _)
//         }
//         #[inline]
//         fn i128(self, value: i128) -> Self::Result {
//             self.isize(value as _)
//         }
//         #[inline]
//         fn isize(self, value: isize) -> Self::Result {
//             Node::Integer(value)
//         }
//         #[inline]
//         fn f32(self, value: f32) -> Self::Result {
//             self.f64(value as _)
//         }
//         #[inline]
//         fn f64(self, value: f64) -> Self::Result {
//             Node::Floating(value)
//         }
//         #[inline]
//         fn shared<T: ?Sized>(self, _: &T) -> Self::Result {
//             Primitive::unit(self)
//         }
//         #[inline]
//         fn constant<T: ?Sized>(self, _: *const T) -> Self::Result {
//             Primitive::unit(self)
//         }
//     }

//     impl Items for ItemsSerializer {
//         fn item<T: Visit>(mut self, value: T) -> Self {
//             self.0.push(value.visit(Serializer));
//             self
//         }
//     }

//     impl Into<Node> for FieldsSerializer {
//         #[inline]
//         fn into(self) -> Node {
//             Node::Object(self.0)
//         }
//     }

//     impl Fields for FieldsSerializer {
//         fn field<K: Visit, V: Visit>(mut self, key: K, value: V) -> Self {
//             match (key.visit(Serializer), value.visit(Serializer)) {
//                 (Node::Null, _) | (_, Node::Null) => {}
//                 (key, value) => self.0.push((key, value)),
//             }
//             self
//         }
//     }

//     impl Into<Node> for ItemsSerializer {
//         #[inline]
//         fn into(self) -> Node {
//             Node::Array(self.0)
//         }
//     }

//     impl Variant for Serializer {
//         type Result = Node;
//         type Fields = VariantSerializer<FieldsSerializer>;
//         type Items = VariantSerializer<ItemsSerializer>;

//         #[inline]
//         fn unit(self, name: &'static str, _: usize) -> Self::Result {
//             VariantSerializer(name, Node::Null).into()
//         }

//         #[inline]
//         fn tuple<const N: usize>(self, name: &'static str, _: usize) -> Self::Items {
//             VariantSerializer(name, Sequence::list(self, N))
//         }

//         #[inline]
//         fn map<const N: usize>(self, name: &'static str, _: usize) -> Self::Fields {
//             VariantSerializer(name, Sequence::map(self, N))
//         }
//     }

//     impl<S: Items> Items for VariantSerializer<S> {
//         #[inline]
//         fn item<T: Visit>(self, value: T) -> Self {
//             Self(self.0, self.1.item(value))
//         }
//     }

//     impl<S: Fields> Fields for VariantSerializer<S> {
//         #[inline]
//         fn field<K: Visit, V: Visit>(self, key: K, value: V) -> Self {
//             Self(self.0, self.1.field(key, value))
//         }
//     }

//     impl<S: Into<Node>> Into<Node> for VariantSerializer<S> {
//         #[inline]
//         fn into(self) -> Node {
//             Node::Object(vec![(Node::String(self.0.into()), self.1.into())])
//         }
//     }
// }

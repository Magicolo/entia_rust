use crate::visit::*;
use std::{
    fmt::Write,
    str::{self},
};

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

impl Visit for Node {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        match self {
            Node::Null => visitor.primitive().unit(),
            Node::Boolean(value) => visitor.primitive().bool(*value),
            Node::Integer(value) => visitor.primitive().isize(*value),
            Node::Floating(value) => visitor.primitive().f64(*value),
            Node::String(value) => visitor.sequence().string(value),
            Node::Array(nodes) => visitor.sequence().items(nodes),
            Node::Object(nodes) => visitor
                .sequence()
                .fields(nodes.iter().map(|(key, value)| (key, value))),
        }
    }
}

mod poulah_serialize {
    use super::*;

    // TODO: Remove this and implement 'Serialize' directly? What other wrapper type could be relevant here?
    pub struct Source<'a, T: ?Sized>(&'a T); // TODO: impl Deref?

    pub trait Serialize {
        fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error>;
    }

    pub trait Serializer {
        type Value;
        type Error;
        type Primitive: Primitive<Value = Self::Value, Error = Self::Error>;
        type Structure: Structure<Value = Self::Value, Error = Self::Error>;

        fn primitive(self) -> Self::Primitive;
        fn structure(self, name: &'static str) -> Self::Structure;
    }

    pub trait Primitive {
        type Value;
        type Error;
        fn bool(self, value: bool) -> Result<Self::Value, Self::Error>;
    }

    pub trait Structure {
        type Value;
        type Error;
        type Fields: Fields<Error = Self::Error> + Into<Result<Self::Value, Self::Error>>;
        fn map<const N: usize>(self) -> Result<Self::Fields, Self::Error>;
    }

    pub trait Fields: Sized {
        type Error;
        fn field<K: Serialize, V: Serialize>(
            &mut self,
            key: K,
            value: V,
        ) -> Result<(), Self::Error>;
    }

    impl<'a, T: ?Sized> Serialize for &'a T
    where
        Source<'a, T>: Serialize,
    {
        #[inline]
        fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
            Source(self).serialize(serializer)
        }
    }

    impl<'a, T: ?Sized> Serialize for &'a mut T
    where
        &'a T: Serialize,
    {
        #[inline]
        fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
            (&*self).serialize(serializer)
        }
    }

    impl<'a, T: ?Sized> Serialize for Source<'a, &T>
    where
        Source<'a, T>: Serialize,
    {
        fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
            Source(&**self.0).serialize(serializer)
        }
    }

    impl<'a, T: ?Sized> Serialize for Source<'a, &mut T>
    where
        Source<'a, T>: Serialize,
    {
        fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
            Source(&**self.0).serialize(serializer)
        }
    }

    impl Serialize for Source<'_, Boba> {
        fn serialize<'a, S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
            let Boba { a, b, c } = self.0;
            let mut fields = serializer.structure("Boba").map::<3>()?;
            fields.field("a", Source(a))?;
            fields.field("b", Source(b))?;
            fields.field("c", Source(c))?;
            fields.into()
        }
    }

    impl Serialize for Source<'_, str> {
        fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
            todo!()
        }
    }

    impl Serialize for Source<'_, bool> {
        fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
            serializer.primitive().bool(*self.0)
        }
    }

    impl Serialize for Source<'_, usize> {
        fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
            todo!()
        }
    }

    impl<'a, T> Serialize for Source<'a, Vec<T>>
    where
        Source<'a, T>: Serialize,
    {
        fn serialize<S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
            todo!()
        }
    }
}

mod deserialize {
    use super::*;

    impl Visit for Boba {
        fn visit<V: Visitor>(visitor: V) -> Self {
            /*
               let (a, b, c) = visitor
               .structure("Boba")
               .map::<3>()
               .field("a")
               .field("b")
               .field("c")
               .into();
               Boba { a, b, c }
            */

            /*
                impl Fields for FieldsDeserializer {
                    type Field = FieldDeserializer;
                    type Error = Error;

                    // For a json parser:
                    // - Tries to parse at head as 'K'.
                    // - If 'Ok(K)', move the head right after the ':' and return 'Some((K, Field(head)))'.
                    // - If 'Err(E)', move the head before the next key and loop; if no keys are left return 'None'.
                    fn field<K: Visit>(&mut self) -> Result<(K, Self::Field), Self::Error>;
                    fn miss<K: Visit, V: Visit>(&mut self, key: K) -> Result<V, Self::Error>;
                }

                impl Field for FieldDeserializer {
                    type Error = Error;
                    // If 'value' is 'Some(&mut V)', deserialize in place.
                    fn value<'a, V: Visit>(self, value: Target<V, 'a>) -> Result<(), Self::Error>;
                    fn excess(self) -> Result<(), Self::Error>;
                }

                fn usage() {
                    let boba = Boba { ... };
                    json::serialize(&boba);
                    json::deserialize::<Boba>();
                    json::deserialize_in(&mut boba);
                }

                mod json {
                    fn serialize<'a, T: Into<Source<T, 'a>>(value: T) -> Node where Source<T, 'a>: Serialize;
                    fn deserialize<T: Into<Target<T, 'a>>(value: T) -> Node where Target<T, 'a>: Deserialize;

                }

                struct Source<T, 'a>(&'a T); // impl Deref?
                enum Target<T, 'a> {
                    New(&'a mut Option<T>),
                    Existing(&'a mut T),
                }

                trait Set {
                    type Value;
                    fn set(&mut self, value: Self::Value);
                }

                impl<T> Set for Target<T, '_> {
                    type Value = T;

                    #[inline]
                    fn set(&mut self, value: Self::Value) {
                        match self {
                            Target::New(Some(target)) | Target::Existing(target) => *target = value,
                            Target::New(target) => *target = Some(value),
                        }
                    }
                }

                impl<'a, T> Serialize for &'a T where Source<T, 'a>: Serialize {
                    #[inline]
                    fn serialize<'a, S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
                        Source(self).serialize(serializer)
                    }
                }

                impl<'a, T> Serialize for &'a mut T where Source<T, 'a>: Serialize {
                    #[inline]
                    fn serialize<'a, S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
                        Source(self).serialize(serializer)
                    }
                }

                impl Serialize for Source<Boba, '_> {
                    fn serialize<'a, S: Serializer>(self, serializer: S) -> Result<S::Value, S::Error> {
                        let Boba { a, b, c } = self.0;
                        let mut fields = serializer.structure("Boba").map::<3>()?;
                        fields.field("a", Source(a))?;
                        fields.field("b", Source(b))?;
                        fields.field("c", Source(c))?;
                        fields.into()
                    }
                }

                impl<'a, T> Deserialize for &'a mut T where Target<T, 'a>: Deserialize {
                    #[inline]
                    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<S::Value, S::Error> {
                        Target::Existing(self).deserialize(deserializer)
                    }
                }

                impl<'a, T> Deserialize for &'a mut Option<T> where Target<T, 'a>: Deserialize {
                    #[inline]
                    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<S::Value, S::Error> {
                        Target::New(self).deserialize(deserializer)
                    }
                }

                impl Deserialize for Target<bool, '_> {
                    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<D::Value, D::Error> {
                        deserializer.primitive().bool(self)
                    }
                }

                impl<T> Deserialize for Target<Vec<T>, '_> {
                    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<D::Value, D::Error> {
                        let mut items = deserializer.sequence("Vec").list()?;
                        let (low, _) = items.size_hint();
                        match self {
                            Target::New(Some(target)) | Target::Existing(target) => {
                                target.clear();
                                target.reserve(low);
                                for item in items {
                                    target.push(item.value()?);
                                }
                            }
                            Target::New(target) => {
                                let value = Vec::with_capacity(low);
                                for item in items {
                                    target.push(item.value()?);
                                }
                                *target = Some(value);
                            }
                        }
                        items.into()
                    }
                }

                impl Deserialize for Target<Boba, '_> {
                    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<D::Value, D::Error> {
                        let mut fields = deserializer.structure("Boba").map::<3>()?;
                        match self {
                            Target::New(Some(Boba { a, b, c })) | Target::Existing(Boba { a, b, c }) => {
                                for (key, field) in fields {
                                    match key {
                                        "a" => field.value(Target::Existing(a))?,
                                        "b" => field.value(Target::Existing(b))?,
                                        "c" => field.value(Target::Existing(c))?,
                                        _ => field.excess()?,
                                    }
                                }
                            },
                            Target::New(target) => {
                                let (mut a, mut b, mut c) = (None, None, None);
                                for (key, field) in fields {
                                    match key {
                                        "a" => field.value(Target::New(&mut a))?,
                                        "b" => field.value(Target::New(&mut b))?,
                                        "c" => field.value(Target::New(&mut c))?,
                                        _ => field.excess()?,
                                    }
                                }
                                *target = Some(Boba {
                                    a: if let Some(a) = a { a } else { fields.miss("a")? },
                                    b: if let Some(b) = b { a } else { fields.miss("b")? },
                                    c: if let Some(c) = c { a } else { fields.miss("c")? },
                                });
                            }
                        }
                        fields.into()
                    }
                }

            */

            /*
                struct FieldsSerializer();

                impl<'a> Fields<'a> for FieldsSerializer {
                    fn field<K: Visit, V: Visit, FK: Fn(K) -> bool + 'a, FV: FnOnce(V) + 'a>(&mut self, key: FK, value: FV) -> Self {
                        // For json parser:
                        // -
                    }
                }

                let (mut a, mut b, mut c) = (None, None, None);
                visitor.structure("Boba").map::<3>()
                    .field("a", Target::New(&mut a))
                    .field("b", Target::New(&mut b))
                    .field("c", Target::New(&mut c))
                    .into()?;

            */

            /*
                let (mut a, mut b, mut c) = (None, None, None);
                // Passing a 'FnMut' to 'map', gives the flexibility to a parser to skip missing or invalid keys.
                // - The 'fields' parameter can be easily aligned with the 'key'.
                visitor.structure("Boba").map::<3>(|key, fields| match key {
                    "a" => a = Some(fields.field()),
                    "b" => b = Some(fields.field()),
                    "c" => c = Some(fields.field()),
                });
                Boba { a, b, c, } ???
            */

            /*
                let mut fields = visitor.structure("Boba").map::<3>();
                match self {
                    Target::New(target) => {
                        let (mut a, mut b, mut c) = (None, None, None);
                        fields.field("a", Target::New(&mut a))?;
                        fields.field("b", Target::New(&mut b))?;
                        fields.field("c", Target::New(&mut c))?;
                        *target = Some(Boba {
                            a: match a { Some(a) => a, None => fields.miss("a")? },
                            b: match b { Some(b) => b, None => fields.miss("b")? },
                            c: match c { Some(c) => c, None => fields.miss("c")? },
                        });
                    },
                    Target::Exist(Boba { a, b, c }) => {
                        fields.field("a", Target::Exist(a));
                        fields.field("b", Target::Exist(b));
                        fields.field("c", Target::Exist(c));
                    }
                }
                fields.into()


            */

            let mut fields = visitor.structure("Boba").map::<3>();
            Boba {
                a: fields.field("a"),
                b: fields.field("b"),
                c: fields.field("c"),
            }
        }
    }

    impl Visit for Fett {
        fn visit<V: Visitor>(visitor: V) -> Self {
            // visitor
            //     .enumeration("Fett")
            //     .variant("A", |variant| Fett::A(variant.tuple::<1>().item()))
            //     .variant("B", |variant| Fett::B(variant.tuple::<1>().item()))
            //     .variant("C", |variant| Fett::C(variant.tuple::<1>().item()))
            //     .into()

            let (key, variant) = visitor.enumeration("Fett").variant();
            match key {
                "A" => Fett::A(variant.tuple::<1>().item()),
                "B" => Fett::B(variant.tuple::<1>().item()),
                "C" => Fett::C(variant.tuple::<1>().item()),
                _ => todo!(),
            }
        }
    }

    struct Deserializer<'a>(&'a Node);
    struct FieldsDeserializer<'a>(&'a [(Node, Node)], &'a [Node], usize);

    pub trait Deserialize<T> {
        fn deserialize(&self) -> T;
    }

    impl<T: Visit> Deserialize<T> for Node {
        fn deserialize(&self) -> T {
            T::visit(Deserializer(self))
        }
    }

    pub trait Visit {
        fn visit<V: Visitor>(visitor: V) -> Self;
    }

    pub trait Visitor {
        type Structure: Structure;
        type Enumeration: Enumeration;

        fn structure(self, name: &'static str) -> Self::Structure;
        fn enumeration(self, name: &'static str) -> Self::Enumeration;
    }

    pub trait Structure {
        type Fields: Fields;
        fn map<const N: usize>(self) -> Self::Fields;
    }

    pub trait Enumeration {
        type Variant: Variant;
        fn variant<K: Visit>(self) -> (K, Self::Variant);
    }

    pub trait Variant {
        type Fields: Fields;
        type Items: Items;

        fn tuple<const N: usize>(self) -> Self::Items;
    }

    pub trait Fields {
        fn field<V: Visit>(&mut self, key: &'static str) -> V;
    }

    pub trait Items {
        fn item<T: Visit>(&mut self) -> T;
    }

    impl Visit for &str {
        fn visit<V: Visitor>(visitor: V) -> Self {
            todo!()
        }
    }

    impl Visit for bool {
        fn visit<V: Visitor>(visitor: V) -> Self {
            todo!()
            // visitor.primitive(|primitive| primitive.bool())
        }
    }

    impl Visit for usize {
        fn visit<V: Visitor>(visitor: V) -> Self {
            todo!()
            // visitor.primitive(|primitive| primitive.usize())
        }
    }

    impl<T: Visit> Visit for Vec<T> {
        fn visit<V: Visitor>(visitor: V) -> Self {
            todo!()
        }
    }

    impl Visitor for Deserializer<'_> {
        type Structure = Self;
        type Enumeration = Self;

        #[inline]
        fn structure(self, _: &'static str) -> Self::Structure {
            self
        }

        #[inline]
        fn enumeration(self, _: &'static str) -> Self::Enumeration {
            self
        }
    }

    impl<'a> Structure for Deserializer<'a> {
        type Fields = FieldsDeserializer<'a>;

        #[inline]
        fn map<const N: usize>(self) -> Self::Fields {
            match self.0 {
                Node::Object(nodes) => FieldsDeserializer(nodes, &[], 0),
                Node::Array(nodes) => FieldsDeserializer(&[], nodes, 0),
                _ => FieldsDeserializer(&[], &[], 0),
            }
        }
    }

    impl Enumeration for Deserializer<'_> {
        type Variant = Self;

        fn variant<K: Visit>(self) -> (K, Self::Variant) {
            todo!()
        }
    }

    impl<'a> Variant for Deserializer<'a> {
        type Fields = FieldsDeserializer<'a>;
        type Items = FieldsDeserializer<'a>;

        fn tuple<const N: usize>(self) -> Self::Items {
            todo!()
        }
    }

    impl Fields for FieldsDeserializer<'_> {
        #[inline]
        fn field<V: Visit>(&mut self, key: &'static str) -> V {
            let node = self
                .0
                .iter()
                .find_map(|pair| {
                    if pair.0.string() == Some(key) {
                        Some(&pair.1)
                    } else {
                        None
                    }
                })
                .unwrap_or(&Node::Null);
            V::visit(Deserializer(node))
        }
    }

    impl Items for FieldsDeserializer<'_> {
        fn item<T: Visit>(&mut self) -> T {
            let node = match self.1.get(self.2) {
                Some(value) => value,
                None => match self.0.get(self.2) {
                    Some((_, value)) => value,
                    None => &Node::Null,
                },
            };
            self.2 += 1;
            T::visit(Deserializer(node))
        }
    }
}

mod json {
    use super::*;

    pub struct Serializer(String);
    pub struct ScopeSerializer<const O: char, const C: char>(String, bool);
    pub struct VariantSerializer<const O: char, const C: char>(ScopeSerializer<O, C>);

    impl Visitor for Serializer {
        type Result = String;
        type Primitive = Self;
        type Structure = Self;
        type Enumeration = Self;
        type Sequence = Self;

        #[inline]
        fn primitive(self) -> Self::Primitive {
            self
        }
        #[inline]
        fn structure(self, _: &'static str) -> Self::Structure {
            self
        }
        #[inline]
        fn enumeration(self, _: &'static str) -> Self::Enumeration {
            self
        }
        #[inline]
        fn sequence(self) -> Self::Sequence {
            self
        }
    }

    impl Structure for Serializer {
        type Result = String;
        type Fields = ScopeSerializer<'{', '}'>;
        type Items = ScopeSerializer<'[', ']'>;

        #[inline]
        fn unit(self) -> Self::Result {
            Primitive::unit(self)
        }
        #[inline]
        fn tuple<const N: usize>(self) -> Self::Items {
            Sequence::list(self, N)
        }
        #[inline]
        fn map<const N: usize>(self) -> Self::Fields {
            Sequence::map(self, N)
        }
    }

    impl Sequence for Serializer {
        type Result = String;
        type Fields = ScopeSerializer<'{', '}'>;
        type Items = ScopeSerializer<'[', ']'>;

        #[inline]
        fn list(self, _: usize) -> Self::Items {
            ScopeSerializer::new(self.0)
        }

        #[inline]
        fn map(self, _: usize) -> Self::Fields {
            ScopeSerializer::new(self.0)
        }

        #[inline]
        fn string(mut self, value: &str) -> Self::Result {
            self.0.push('"');
            self.0.push_str(value);
            self.0.push('"');
            self.0
        }

        #[inline]
        fn bytes(self, value: &[u8]) -> Self::Result {
            match str::from_utf8(value) {
                Ok(string) => self.string(string),
                Err(_) => self.slice(value),
            }
        }
    }

    impl Primitive for Serializer {
        type Result = String;

        #[inline]
        fn unit(mut self) -> Self::Result {
            self.0.push_str("null");
            self.0
        }
        #[inline]
        fn never(self) -> Self::Result {
            Primitive::unit(self)
        }
        #[inline]
        fn bool(mut self, value: bool) -> Self::Result {
            self.0.push_str(if value { "true" } else { "false" });
            self.0
        }
        #[inline]
        fn char(mut self, value: char) -> Self::Result {
            self.0.push(value);
            self.0
        }
        #[inline]
        fn u8(mut self, value: u8) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn u16(mut self, value: u16) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn u32(mut self, value: u32) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn u64(mut self, value: u64) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn u128(mut self, value: u128) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn usize(mut self, value: usize) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn i8(mut self, value: i8) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn i16(mut self, value: i16) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn i32(mut self, value: i32) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn i64(mut self, value: i64) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn i128(mut self, value: i128) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn isize(mut self, value: isize) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn f32(mut self, value: f32) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn f64(mut self, value: f64) -> Self::Result {
            write!(&mut self.0, "{}", value).unwrap();
            self.0
        }
        #[inline]
        fn shared<T: ?Sized>(self, _: &T) -> Self::Result {
            Primitive::unit(self)
        }
        #[inline]
        fn constant<T: ?Sized>(self, _: *const T) -> Self::Result {
            Primitive::unit(self)
        }
    }

    impl Enumeration for Serializer {
        type Result = String;
        type Variant = Self;

        #[inline]
        fn never(self) -> Self::Result {
            Primitive::unit(self)
        }

        #[inline]
        fn variant<const N: usize>(self) -> Self::Variant {
            self
        }
    }

    impl Variant for Serializer {
        type Result = String;
        type Fields = VariantSerializer<'{', '}'>;
        type Items = VariantSerializer<'[', ']'>;

        #[inline]
        fn unit(mut self, name: &'static str, _: usize) -> Self::Result {
            self.0.push_str(name);
            self.0
        }

        #[inline]
        fn tuple<const N: usize>(mut self, name: &'static str, _: usize) -> Self::Items {
            self.0.push('{');
            self.0.push_str(name);
            self.0.push(':');
            VariantSerializer(Sequence::list(self, N))
        }

        #[inline]
        fn map<const N: usize>(mut self, name: &'static str, _: usize) -> Self::Fields {
            self.0.push('{');
            self.0.push_str(name);
            self.0.push(':');
            VariantSerializer(Sequence::map(self, N))
        }
    }

    impl<const O: char, const C: char> Fields for VariantSerializer<O, C> {
        #[inline]
        fn field<K: Visit, V: Visit>(self, key: K, value: V) -> Self {
            Self(self.0.field(key, value))
        }
    }

    impl<const O: char, const C: char> Items for VariantSerializer<O, C> {
        #[inline]
        fn item<T: Visit>(self, value: T) -> Self {
            Self(self.0.item(value))
        }
    }

    impl<const O: char, const C: char> Into<String> for VariantSerializer<O, C> {
        #[inline]
        fn into(self) -> String {
            let mut buffer: String = self.0.into();
            buffer.push('}');
            buffer
        }
    }

    impl<const O: char, const C: char> Into<String> for ScopeSerializer<O, C> {
        #[inline]
        fn into(self) -> String {
            let mut buffer = self.0;
            buffer.push(C);
            buffer
        }
    }

    impl<const O: char, const C: char> ScopeSerializer<O, C> {
        pub fn new(mut buffer: String) -> Self {
            buffer.push(O);
            Self(buffer, false)
        }
    }

    impl<const O: char, const C: char> Fields for ScopeSerializer<O, C> {
        fn field<K: Visit, V: Visit>(mut self, key: K, value: V) -> Self {
            if self.1 {
                self.0.push(',');
            }
            let mut buffer = key.visit(Serializer(self.0));
            buffer.push(':');
            Self(value.visit(Serializer(buffer)), true)
        }
    }

    impl<const O: char, const C: char> Items for ScopeSerializer<O, C> {
        fn item<T: Visit>(mut self, value: T) -> Self {
            if self.1 {
                self.0.push(',');
            }
            Self(value.visit(Serializer(self.0)), true)
        }
    }
}

mod serialize {
    use super::*;

    pub struct Serializer;
    pub struct FieldsSerializer(Vec<(Node, Node)>);
    pub struct ItemsSerializer(Vec<Node>);
    pub struct VariantSerializer<S>(&'static str, S);

    pub trait Serialize<T> {
        fn serialize(&self) -> T;
    }

    impl<T: Visit> Serialize<Node> for T {
        #[inline]
        fn serialize(&self) -> Node {
            self.visit(Serializer)
        }
    }

    impl Visitor for Serializer {
        type Result = Node;
        type Primitive = Self;
        type Structure = Self;
        type Enumeration = Self;
        type Sequence = Self;

        #[inline]
        fn primitive(self) -> Self::Primitive {
            self
        }
        #[inline]
        fn structure(self, _: &'static str) -> Self::Structure {
            self
        }
        #[inline]
        fn enumeration(self, _: &'static str) -> Self::Enumeration {
            self
        }
        #[inline]
        fn sequence(self) -> Self::Sequence {
            self
        }
    }

    impl Structure for Serializer {
        type Result = Node;
        type Fields = FieldsSerializer;
        type Items = ItemsSerializer;

        #[inline]
        fn unit(self) -> Self::Result {
            Primitive::unit(self)
        }

        #[inline]
        fn tuple<const N: usize>(self) -> Self::Items {
            Sequence::list(self, N)
        }

        #[inline]
        fn map<const N: usize>(self) -> Self::Fields {
            Sequence::map(self, N)
        }
    }

    impl Enumeration for Serializer {
        type Result = Node;
        type Variant = Self;

        #[inline]
        fn never(self) -> Self::Result {
            Primitive::unit(self)
        }
        #[inline]
        fn variant<const N: usize>(self) -> Self::Variant {
            self
        }
    }

    impl Sequence for Serializer {
        type Result = Node;
        type Fields = FieldsSerializer;
        type Items = ItemsSerializer;

        #[inline]
        fn list(self, capacity: usize) -> Self::Items {
            ItemsSerializer(Vec::with_capacity(capacity))
        }

        #[inline]
        fn map(self, capacity: usize) -> Self::Fields {
            FieldsSerializer(Vec::with_capacity(capacity))
        }

        #[inline]
        fn string(self, value: &str) -> Self::Result {
            Node::String(value.into())
        }

        #[inline]
        fn bytes(self, value: &[u8]) -> Self::Result {
            match str::from_utf8(value) {
                Ok(value) => self.string(value),
                Err(_) => self.slice(value),
            }
        }
    }

    impl Primitive for Serializer {
        type Result = Node;

        #[inline]
        fn unit(self) -> Self::Result {
            Node::Null
        }
        #[inline]
        fn never(self) -> Self::Result {
            Primitive::unit(self)
        }
        #[inline]
        fn bool(self, value: bool) -> Self::Result {
            Node::Boolean(value)
        }
        #[inline]
        fn char(self, value: char) -> Self::Result {
            Node::String(value.into())
        }
        #[inline]
        fn u8(self, value: u8) -> Self::Result {
            self.isize(value as _)
        }
        #[inline]
        fn u16(self, value: u16) -> Self::Result {
            self.isize(value as _)
        }
        #[inline]
        fn u32(self, value: u32) -> Self::Result {
            self.isize(value as _)
        }
        #[inline]
        fn u64(self, value: u64) -> Self::Result {
            self.isize(value as _)
        }
        #[inline]
        fn u128(self, value: u128) -> Self::Result {
            self.isize(value as _)
        }
        #[inline]
        fn usize(self, value: usize) -> Self::Result {
            self.isize(value as _)
        }
        #[inline]
        fn i8(self, value: i8) -> Self::Result {
            self.isize(value as _)
        }
        #[inline]
        fn i16(self, value: i16) -> Self::Result {
            self.isize(value as _)
        }
        #[inline]
        fn i32(self, value: i32) -> Self::Result {
            self.isize(value as _)
        }
        #[inline]
        fn i64(self, value: i64) -> Self::Result {
            self.isize(value as _)
        }
        #[inline]
        fn i128(self, value: i128) -> Self::Result {
            self.isize(value as _)
        }
        #[inline]
        fn isize(self, value: isize) -> Self::Result {
            Node::Integer(value)
        }
        #[inline]
        fn f32(self, value: f32) -> Self::Result {
            self.f64(value as _)
        }
        #[inline]
        fn f64(self, value: f64) -> Self::Result {
            Node::Floating(value)
        }
        #[inline]
        fn shared<T: ?Sized>(self, _: &T) -> Self::Result {
            Primitive::unit(self)
        }
        #[inline]
        fn constant<T: ?Sized>(self, _: *const T) -> Self::Result {
            Primitive::unit(self)
        }
    }

    impl Items for ItemsSerializer {
        fn item<T: Visit>(mut self, value: T) -> Self {
            self.0.push(value.visit(Serializer));
            self
        }
    }

    impl Into<Node> for FieldsSerializer {
        #[inline]
        fn into(self) -> Node {
            Node::Object(self.0)
        }
    }

    impl Fields for FieldsSerializer {
        fn field<K: Visit, V: Visit>(mut self, key: K, value: V) -> Self {
            match (key.visit(Serializer), value.visit(Serializer)) {
                (Node::Null, _) | (_, Node::Null) => {}
                (key, value) => self.0.push((key, value)),
            }
            self
        }
    }

    impl Into<Node> for ItemsSerializer {
        #[inline]
        fn into(self) -> Node {
            Node::Array(self.0)
        }
    }

    impl Variant for Serializer {
        type Result = Node;
        type Fields = VariantSerializer<FieldsSerializer>;
        type Items = VariantSerializer<ItemsSerializer>;

        #[inline]
        fn unit(self, name: &'static str, _: usize) -> Self::Result {
            VariantSerializer(name, Node::Null).into()
        }

        #[inline]
        fn tuple<const N: usize>(self, name: &'static str, _: usize) -> Self::Items {
            VariantSerializer(name, Sequence::list(self, N))
        }

        #[inline]
        fn map<const N: usize>(self, name: &'static str, _: usize) -> Self::Fields {
            VariantSerializer(name, Sequence::map(self, N))
        }
    }

    impl<S: Items> Items for VariantSerializer<S> {
        #[inline]
        fn item<T: Visit>(self, value: T) -> Self {
            Self(self.0, self.1.item(value))
        }
    }

    impl<S: Fields> Fields for VariantSerializer<S> {
        #[inline]
        fn field<K: Visit, V: Visit>(self, key: K, value: V) -> Self {
            Self(self.0, self.1.field(key, value))
        }
    }

    impl<S: Into<Node>> Into<Node> for VariantSerializer<S> {
        #[inline]
        fn into(self) -> Node {
            Node::Object(vec![(Node::String(self.0.into()), self.1.into())])
        }
    }
}

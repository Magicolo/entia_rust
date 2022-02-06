use crate::{deserializer::*, node::Node, recurse};
use std::{
    marker::PhantomData,
    str::{self, Utf8Error},
};

pub struct Boba {
    a: bool,
    b: usize,
    c: Vec<bool>,
}

pub enum Fett {
    A,
    B(bool, usize, Boba),
    C { a: Vec<bool>, b: char, c: String },
}

#[derive(Debug)]
pub struct New<T: ?Sized>(PhantomData<T>);

pub trait Deserialize {
    type Value;
    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error>;
}

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
        deserializer.sequence()?.array(self)
    }
}

impl<T, const N: usize> Deserialize for New<[T; N]>
where
    New<T>: Deserialize,
{
    type Value = [<New<T> as Deserialize>::Value; N];

    #[inline]
    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.sequence()?.array([New::<T>::new(); N])
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
        deserializer
            .sequence()?
            .array([(); N].map(|_| iterator.next().unwrap()))
    }
}

impl<'a, T> Deserialize for &'a mut [T]
where
    for<'b> &'b mut T: Deserialize,
{
    type Value = &'a mut [T];

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.sequence()?.slice::<T>(self, false)
    }
}

impl<'a> Deserialize for &'a mut str {
    type Value = &'a mut str;

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.sequence()?.string(self, false)
    }
}

impl<T> Deserialize for New<Vec<T>>
where
    New<T>: Deserialize<Value = T>,
{
    type Value = Vec<T>;

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let mut items = deserializer.sequence()?.list()?;
        // TODO: Get capacity somehow? Maybe through 'impl Iterator for Items'?
        let mut value = Vec::new();
        while let Some(item) = items.item()? {
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
        let mut items = deserializer.sequence()?.list()?;
        let mut index = 0;
        while let Some(item) = items.item()? {
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
        todo!()
        // let mut items = deserializer.sequence()?.list()?;
        // // TODO: Get capacity somehow? Maybe through 'impl Iterator for Items'?
        // let mut value = Vec::new();
        // while let Some(item) = items.item()? {
        //     value.push(item.value(New::<char>::new())?);
        // }
        // Ok(value)
    }
}

impl Deserialize for &mut String {
    type Value = ();

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        todo!()
        // let mut items = deserializer.sequence()?.list()?;
        // let mut index = 0;
        // while let Some(item) = items.item()? {
        //     match self.get_mut(index) {
        //         Some(value) => {
        //             item.value(value)?;
        //         }
        //         None => self.push(item.value(New::<T>::new())?),
        //     }
        //     index += 1;
        // }
        // self.truncate(index);
        // Ok(())
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
                deserializer.primitive()?.$t()
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
                deserializer.primitive()?.unit()
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
                let mut items = deserializer.sequence()?.list()?;
                $(let $p = match items.item()? { Some(item) => item.value($p)?, None => items.miss($p)?, };)*
                while let Some(item) = items.item()? {
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

#[inline]
fn value_or_miss<I: Items, V: Deserialize>(items: &mut I, value: V) -> Result<V::Value, I::Error> {
    match items.item()? {
        Some(item) => item.value(value),
        None => items.miss(value),
    }
}

#[inline]
fn value_or_skip<I: Items, V: Deserialize>(items: &mut I, value: V) -> Result<(), I::Error> {
    if let Some(item) = items.item()? {
        item.value(value)?;
    }
    Ok(())
}

mod example {
    use super::*;

    macro_rules! new_map {
        ($f:expr, $l:expr $(,$p:ident: $t:ty)*) => {{
            $(let mut $p = None;)*
            while let Some((key, field)) = $f.field(unsafe { str::from_utf8_unchecked_mut(&mut [0; $l]) })? {
                match &*key {
                    $(stringify!($p) => $p = Some(field.value(New::<$t>::new())?),)*
                    _ => field.excess()?,
                }
            }
            ($(match $p { Some($p) => $p, None => $f.miss(stringify!($p), New::<$t>::new())? },)*)
        }};
    }

    macro_rules! use_map {
        ($f:expr, $l:expr $(,$p:ident)*) => {{
            while let Some((key, field)) = $f.field(unsafe { str::from_utf8_unchecked_mut(&mut [0; $l]) })? {
                match &*key {
                    $(stringify!($p) => field.value(&mut *$p)?,)*
                    _ => field.excess()?,
                }
            }
        }};
    }

    impl Deserialize for New<Boba> {
        type Value = Boba;

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            let mut fields = deserializer.structure("Boba")?.map::<3>()?;
            let (a, b, c) = new_map!(fields, 1, a: bool, b: usize, c: Vec<bool>);
            Ok(Boba { a, b, c })
        }
    }

    impl Deserialize for &mut Boba {
        type Value = ();

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            let mut fields = deserializer.structure("Boba")?.map::<3>()?;
            let Boba { a, b, c } = self;
            use_map!(fields, 1, a, b, c);
            Ok(())
        }
    }

    impl Deserialize for New<Fett> {
        type Value = Fett;

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            let key = &mut [0];
            let (key, variant) = deserializer
                .enumeration("Fett")?
                .variant::<_, 3>(unsafe { str::from_utf8_unchecked_mut(key) })?;
            match &*key {
                "A" => {
                    variant.unit("A", 0)?;
                    Ok(Fett::A)
                }
                "B" => {
                    let mut items = variant.tuple::<3>("B", 1)?;
                    let a = value_or_miss(&mut items, New::<bool>::new())?;
                    let b = value_or_miss(&mut items, New::<usize>::new())?;
                    let c = value_or_miss(&mut items, New::<Boba>::new())?;
                    items.drain()?;
                    Ok(Fett::B(a, b, c))
                }
                "C" => {
                    let mut fields = variant.map::<3>("C", 2)?;
                    let (a, b, c) = new_map!(fields, 1, a: Vec<bool>, b: char, c: String);
                    Ok(Fett::C { a, b, c })
                }
                _ => todo!(), // variant.excess()?,
            }
        }
    }

    impl Deserialize for &mut Fett {
        type Value = ();

        #[inline]
        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            let key = &mut [0];
            let (key, variant) = deserializer
                .enumeration("Fett")?
                .variant::<_, 3>(unsafe { str::from_utf8_unchecked_mut(key) })?;
            match &*key {
                "A" => {
                    variant.unit("A", 0)?;
                    *self = Fett::A;
                    Ok(())
                }
                "B" => {
                    let mut items = variant.tuple::<3>("B", 1)?;
                    match self {
                        Fett::B(a, b, c) => {
                            value_or_skip(&mut items, a)?;
                            value_or_skip(&mut items, b)?;
                            value_or_skip(&mut items, c)?;
                        }
                        value => {
                            let a = value_or_miss(&mut items, New::<bool>::new())?;
                            let b = value_or_miss(&mut items, New::<usize>::new())?;
                            let c = value_or_miss(&mut items, New::<Boba>::new())?;
                            *value = Fett::B(a, b, c);
                        }
                    }
                    items.drain()
                }
                "C" => {
                    let mut fields = variant.map::<3>("C", 2)?;
                    match self {
                        Fett::C { a, b, c } => {
                            use_map!(fields, 1, a, b, c);
                        }
                        value => {
                            let (a, b, c) = new_map!(fields, 1, a: Vec<bool>, b: char, c: String);
                            *value = Fett::C { a, b, c };
                        }
                    }
                    Ok(())
                }
                _ => todo!(), // variant.excess()???
            }
        }
    }
}

mod node {
    use super::*;
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
        fn structure(self, name: &'static str) -> Result<Self::Structure, Self::Error> {
            Ok(self)
        }
        #[inline]
        fn enumeration(self, name: &'static str) -> Result<Self::Enumeration, Self::Error> {
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

        fn unit(self, name: &'static str, index: usize) -> Result<(), Self::Error> {
            todo!()
        }

        fn map<const N: usize>(
            self,
            name: &'static str,
            index: usize,
        ) -> Result<Self::Fields, Self::Error> {
            todo!()
        }

        fn tuple<const N: usize>(
            self,
            name: &'static str,
            index: usize,
        ) -> Result<Self::Items, Self::Error> {
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
}

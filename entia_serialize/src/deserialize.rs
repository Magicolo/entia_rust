use crate::{deserializer::*, json::Node, recurse};
use std::{
    marker::PhantomData,
    str::{self, Utf8Error},
};

pub struct Boba {
    a: bool,
    b: usize,
    c: bool,
}

pub enum Fett {
    A,
    B(bool, usize, Boba),
    C { a: bool, b: char, c: String },
}

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
        deserializer.array(self)
    }
}

impl<T, const N: usize> Deserialize for New<[T; N]>
where
    New<T>: Deserialize,
{
    type Value = [<New<T> as Deserialize>::Value; N];

    #[inline]
    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.array([New::<T>::new(); N])
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
        deserializer.array([(); N].map(|_| iterator.next().unwrap()))
    }
}

impl<'a, T> Deserialize for &'a mut [T]
where
    for<'b> &'b mut T: Deserialize,
{
    type Value = &'a mut [T];

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.slice::<T>(self, false)
    }
}

impl<'a> Deserialize for &'a mut str {
    type Value = &'a mut str;

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.string(self, false)
    }
}

impl<T> Deserialize for New<Vec<T>>
where
    New<T>: Deserialize<Value = T>,
{
    type Value = Vec<T>;

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let mut list = deserializer.list()?;
        // TODO: Get capacity somehow? Maybe through 'impl Iterator for Items'?
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
        todo!()
        // let mut list = deserializer.sequence()?.list()?;
        // // TODO: Get capacity somehow? Maybe through 'impl Iterator for Items'?
        // let mut value = Vec::new();
        // while let Some(item) = list.item()? {
        //     value.push(item.value(New::<char>::new())?);
        // }
        // Ok(value)
    }
}

impl Deserialize for &mut String {
    type Value = ();

    fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        todo!()
        // let mut list = deserializer.sequence()?.list()?;
        // let mut index = 0;
        // while let Some(item) = list.item()? {
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
                let mut list = deserializer.list()?;
                $(let $p = match list.item()? { Some(item) => item.value($p)?, None => list.miss($p)?, };)*
                while let Some(item) = list.item()? {
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
fn value_or_miss<L: List, V: Deserialize>(list: &mut L, value: V) -> Result<V::Value, L::Error> {
    match list.item()? {
        Some(item) => item.value(value),
        None => list.miss(value),
    }
}

#[inline]
fn value_or_skip<L: List, V: Deserialize>(list: &mut L, value: V) -> Result<(), L::Error> {
    if let Some(item) = list.item()? {
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
            let mut map = deserializer.structure::<Boba>()?.map::<3>()?;
            let (a, b, c) = new_map!(map, 1, a: bool, b: usize, c: bool);
            Ok(Boba { a, b, c })
        }
    }

    impl Deserialize for &mut Boba {
        type Value = ();

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            let mut map = deserializer.structure::<Boba>()?.map::<3>()?;
            let Boba { a, b, c } = self;
            use_map!(map, 1, a, b, c);
            Ok(())
        }
    }

    impl Deserialize for New<Fett> {
        type Value = Fett;

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            let key = &mut [0];
            let (key, variant) = deserializer
                .enumeration::<Fett>()?
                .variant::<_, 3>(unsafe { str::from_utf8_unchecked_mut(key) })?;
            match &*key {
                "A" => {
                    variant.unit("A", 0)?;
                    Ok(Fett::A)
                }
                "B" => {
                    let mut list = variant.tuple::<3>("B", 1)?;
                    let a = value_or_miss(&mut list, New::<bool>::new())?;
                    let b = value_or_miss(&mut list, New::<usize>::new())?;
                    let c = value_or_miss(&mut list, New::<Boba>::new())?;
                    list.drain()?;
                    Ok(Fett::B(a, b, c))
                }
                "C" => {
                    let mut map = variant.map::<3>("C", 2)?;
                    let (a, b, c) = new_map!(map, 1, a: bool, b: char, c: String);
                    Ok(Fett::C { a, b, c })
                }
                _ => variant.excess(self),
            }
        }
    }

    impl Deserialize for &mut Fett {
        type Value = ();

        #[inline]
        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            let key = &mut [0];
            let (key, variant) = deserializer
                .enumeration::<Fett>()?
                .variant::<_, 3>(unsafe { str::from_utf8_unchecked_mut(key) })?;
            match &*key {
                "A" => {
                    variant.unit("A", 0)?;
                    *self = Fett::A;
                    Ok(())
                }
                "B" => {
                    let mut list = variant.tuple::<3>("B", 1)?;
                    match self {
                        Fett::B(a, b, c) => {
                            value_or_skip(&mut list, a)?;
                            value_or_skip(&mut list, b)?;
                            value_or_skip(&mut list, c)?;
                        }
                        value => {
                            let a = value_or_miss(&mut list, New::<bool>::new())?;
                            let b = value_or_miss(&mut list, New::<usize>::new())?;
                            let c = value_or_miss(&mut list, New::<Boba>::new())?;
                            *value = Fett::B(a, b, c);
                        }
                    }
                    list.drain()
                }
                "C" => {
                    let mut map = variant.map::<3>("C", 2)?;
                    match self {
                        Fett::C { a, b, c } => {
                            use_map!(map, 1, a, b, c);
                        }
                        value => {
                            let (a, b, c) = new_map!(map, 1, a: bool, b: char, c: String);
                            *value = Fett::C { a, b, c };
                        }
                    }
                    Ok(())
                }
                _ => Variant::excess(variant, self),
            }
        }
    }
}

mod node {
    use super::*;
    pub struct NodeDeserializer<'a>(&'a Node);
    pub struct MapDeserializer<'a>(&'a [(Node, Node)], usize);
    pub struct ListDeserializer<'a>(&'a [Node], usize);

    pub enum Error {
        ExpectedArrayNode,
        ExpectedObjectNode,
        Utf8(Utf8Error),
    }

    impl From<Utf8Error> for Error {
        #[inline]
        fn from(error: Utf8Error) -> Self {
            Error::Utf8(error)
        }
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
        fn structure<T: ?Sized>(self) -> Result<Self::Structure, Self::Error> {
            Ok(self)
        }
        #[inline]
        fn enumeration<T: ?Sized>(self) -> Result<Self::Enumeration, Self::Error> {
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

        fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error> {
            match self.0 {
                Node::Array(nodes) => Ok(ListDeserializer(nodes, 0)),
                _ => Err(Error::ExpectedArrayNode),
            }
        }

        fn map<const N: usize>(self) -> Result<Self::Map, Self::Error> {
            match self.0 {
                Node::Object(pairs) => Ok(MapDeserializer(pairs, 0)),
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
        type Map = MapDeserializer<'a>;
        type List = ListDeserializer<'a>;

        fn unit(self, name: &'static str, index: usize) -> Result<(), Self::Error> {
            todo!()
        }

        fn map<const N: usize>(
            self,
            name: &'static str,
            index: usize,
        ) -> Result<Self::Map, Self::Error> {
            todo!()
        }

        fn tuple<const N: usize>(
            self,
            name: &'static str,
            index: usize,
        ) -> Result<Self::List, Self::Error> {
            todo!()
        }

        #[inline]
        fn excess<V: Deserialize>(self, value: V) -> Result<V::Value, Self::Error> {
            value.deserialize(self)
        }
    }

    impl<'a> Map for MapDeserializer<'a> {
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

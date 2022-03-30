use self::{adapt::*, state::*};
use crate::{recurse, serialize::*};
use std::marker::PhantomData;

pub trait Serializer {
    type Value;
    type Error;
    type Map: Map<Value = Self::Value, Error = Self::Error>;
    type List: List<Value = Self::Value, Error = Self::Error>;
    type Structure: Structure<Value = Self::Value, Error = Self::Error>;
    type Enumeration: Enumeration<Value = Self::Value, Error = Self::Error>;

    fn unit(self) -> Result<Self::Value, Self::Error>;
    fn bool(self, value: bool) -> Result<Self::Value, Self::Error>;
    fn char(self, value: char) -> Result<Self::Value, Self::Error>;

    #[inline]
    fn u8(self, value: u8) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.u128(value as _)
    }
    #[inline]
    fn u16(self, value: u16) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.u128(value as _)
    }
    #[inline]
    fn u32(self, value: u32) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.u128(value as _)
    }
    #[inline]
    fn u64(self, value: u64) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.u128(value as _)
    }
    #[inline]
    fn usize(self, value: usize) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.u128(value as _)
    }
    fn u128(self, value: u128) -> Result<Self::Value, Self::Error>;

    #[inline]
    fn i8(self, value: i8) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.i128(value as _)
    }
    #[inline]
    fn i16(self, value: i16) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.i128(value as _)
    }
    #[inline]
    fn i32(self, value: i32) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.i128(value as _)
    }
    #[inline]
    fn i64(self, value: i64) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.i128(value as _)
    }
    #[inline]
    fn isize(self, value: isize) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.i128(value as _)
    }
    fn i128(self, value: i128) -> Result<Self::Value, Self::Error>;

    #[inline]
    fn f32(self, value: f32) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.f64(value as _)
    }
    fn f64(self, value: f64) -> Result<Self::Value, Self::Error>;

    fn list(self) -> Result<Self::List, Self::Error>;
    fn map(self) -> Result<Self::Map, Self::Error>;

    #[inline]
    fn tuple(self) -> Result<Self::List, Self::Error>
    where
        Self: Sized,
    {
        self.list()
    }
    #[inline]
    fn string(self, value: &str) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.list()?.items(value.chars())
    }
    #[inline]
    fn slice<T: Serialize>(self, value: &[T]) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.list()?.items(value)
    }
    #[inline]
    fn array<T: Serialize, const N: usize>(self, value: &[T; N]) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.list()?.items(value)
    }
    #[inline]
    fn bytes(self, value: &[u8]) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
        self.list()?.items(value)
    }

    fn structure(self) -> Result<Self::Structure, Self::Error>;
    fn enumeration(self) -> Result<Self::Enumeration, Self::Error>;

    #[inline]
    fn state<T>(self, state: T) -> State<Self, T>
    where
        Self: Sized,
        State<Self, T>: Serializer,
    {
        State::new(self, state)
    }

    #[inline]
    fn adapt<T, E: From<Self::Error>, F: FnOnce(Result<Self::Value, Self::Error>) -> Result<T, E>>(
        self,
        adapt: F,
    ) -> Adapt<Self, T, E, F>
    where
        Self: Sized,
        Adapt<Self, T, E, F>: Serializer,
    {
        Adapt::new(self, adapt)
    }
}

pub trait Structure {
    type Value;
    type Error;
    type Map: Map<Value = Self::Value, Error = Self::Error>;
    type List: List<Value = Self::Value, Error = Self::Error>;

    fn unit(self) -> Result<Self::Value, Self::Error>;
    fn tuple(self) -> Result<Self::List, Self::Error>;
    fn map(self) -> Result<Self::Map, Self::Error>;
}

pub trait Enumeration {
    type Value;
    type Error;
    type Structure: Structure<Value = Self::Value, Error = Self::Error>;

    fn never(self) -> Result<Self::Value, Self::Error>;
    fn variant(self, name: &str, index: usize) -> Result<Self::Structure, Self::Error>;
}

pub trait Map: Sized {
    type Value;
    type Error;

    fn pair<K: Serialize, V: Serialize>(self, key: K, value: V) -> Result<Self, Self::Error>;
    fn end(self) -> Result<Self::Value, Self::Error>;

    #[inline]
    fn pairs<K: Serialize, V: Serialize, I: IntoIterator<Item = (K, V)>>(
        mut self,
        pairs: I,
    ) -> Result<Self::Value, Self::Error> {
        for (key, value) in pairs {
            self = self.pair(key, value)?;
        }
        self.end()
    }
}

pub trait List: Sized {
    type Value;
    type Error;

    fn item<T: Serialize>(self, item: T) -> Result<Self, Self::Error>;
    fn end(self) -> Result<Self::Value, Self::Error>;

    #[inline]
    fn items<T: Serialize, I: IntoIterator<Item = T>>(
        mut self,
        items: I,
    ) -> Result<Self::Value, Self::Error> {
        for item in items {
            self = self.item(item)?;
        }
        self.end()
    }
}

macro_rules! tuple {
    () => { tuple!(()); };
    ($p:ident, $t:ident $(, $ps:ident, $ts:ident)*) => { tuple!($t::Error, $p, $t $(, $ps, $ts)*); };
    ($e:ty $(, $p:ident, $t:ident)*) => {
        impl<$($t: Serializer,)*> Serializer for ($($t,)*)
        where
            $($e: From<$t::Error>,)*
        {
            type Value = ($($t::Value,)*);
            type Error = $e;
            type Map = ($($t::Map,)*);
            type List = ($($t::List,)*);
            type Structure = ($($t::Structure,)*);
            type Enumeration = ($($t::Enumeration,)*);

            #[inline]
            fn unit(self) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.unit()?,)*))
            }
            #[inline]
            fn bool(self, _value: bool) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.bool(_value)?,)*))
            }
            #[inline]
            fn char(self, _value: char) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.char(_value)?,)*))
            }
            #[inline]
            fn u8(self, _value: u8) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.u8(_value)?,)*))
            }
            #[inline]
            fn u16(self, _value: u16) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.u16(_value)?,)*))
            }
            #[inline]
            fn u32(self, _value: u32) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.u32(_value)?,)*))
            }
            #[inline]
            fn u64(self, _value: u64) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.u64(_value)?,)*))
            }
            #[inline]
            fn usize(self, _value: usize) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.usize(_value)?,)*))
            }
            #[inline]
            fn u128(self, _value: u128) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.u128(_value)?,)*))
            }
            #[inline]
            fn i8(self, _value: i8) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.i8(_value)?,)*))
            }
            #[inline]
            fn i16(self, _value: i16) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.i16(_value)?,)*))
            }
            #[inline]
            fn i32(self, _value: i32) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.i32(_value)?,)*))
            }
            #[inline]
            fn i64(self, _value: i64) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.i64(_value)?,)*))
            }
            #[inline]
            fn isize(self, _value: isize) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.isize(_value)?,)*))
            }
            #[inline]
            fn i128(self, _value: i128) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.i128(_value)?,)*))
            }
            #[inline]
            fn f32(self, _value: f32) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.f32(_value)?,)*))
            }
            #[inline]
            fn f64(self, _value: f64) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.f64(_value)?,)*))
            }
            #[inline]
            fn list(self) -> Result<Self::List, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.list()?,)*))
            }
            #[inline]
            fn map(self) -> Result<Self::Map, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.map()?,)*))
            }
            #[inline]
            fn tuple(self) -> Result<Self::List, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.tuple()?,)*))
            }
            #[inline]
            fn string(self, _value: &str) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.string(_value)?,)*))
            }
            #[inline]
            fn slice<T: Serialize>(self, _value: &[T]) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.slice(_value)?,)*))
            }
            #[inline]
            fn array<T: Serialize, const N: usize>(
                self,
                _value: &[T; N],
            ) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.array(_value)?,)*))
            }
            #[inline]
            fn bytes(self, _value: &[u8]) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.bytes(_value)?,)*))
            }
            #[inline]
            fn structure(self) -> Result<Self::Structure, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.structure()?,)*))
            }
            #[inline]
            fn enumeration(self) -> Result<Self::Enumeration, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.enumeration()?,)*))
            }
        }

        impl<$($t: Map,)*> Map for ($($t,)*)
        where
            $($e: From<$t::Error>,)*
        {
            type Value = ($($t::Value,)*);
            type Error = $e;

            #[inline]
            fn pair<K: Serialize, V: Serialize>(self, _key: K, _value: V) -> Result<Self, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.pair(&_key, &_value)?,)*))
            }
            #[inline]
            fn end(self) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.end()?,)*))
            }
        }

        impl<$($t: List,)*> List for ($($t,)*)
        where
            $($e: From<$t::Error>,)*
        {
            type Value = ($($t::Value,)*);
            type Error = $e;

            #[inline]
            fn item<T: Serialize>(self, _item: T) -> Result<Self, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.item(&_item)?,)*))
            }
            #[inline]
            fn end(self) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.end()?,)*))
            }
        }

        impl<$($t: Structure,)*> Structure for ($($t,)*)
        where
            $($e: From<$t::Error>,)*
        {
            type Value = ($($t::Value,)*);
            type Error = $e;
            type Map = ($($t::Map,)*);
            type List = ($($t::List,)*);

            #[inline]
            fn unit(self) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.unit()?,)*))
            }
            #[inline]
            fn tuple(self) -> Result<Self::List, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.tuple()?,)*))
            }
            #[inline]
            fn map(self) -> Result<Self::Map, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.map()?,)*))
            }
        }

        impl<$($t: Enumeration,)*> Enumeration for ($($t,)*)
        where
            $($e: From<$t::Error>,)*
        {
            type Value = ($($t::Value,)*);
            type Error = $e;
            type Structure = ($($t::Structure,)*);

            #[inline]
            fn never(self) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.never()?,)*))
            }
            #[inline]
            fn variant(
                self,
                _name: &str,
                _index: usize,
            ) -> Result<Self::Structure, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.variant(_name, _index)?,)*))
            }
        }
    };
}

recurse!(tuple);

pub mod state {
    use super::*;

    pub struct State<S, V>(S, V);

    impl<S, V> State<S, V> {
        #[inline]
        pub(super) const fn new(serializer: S, state: V) -> Self {
            Self(serializer, state)
        }
    }

    impl<S: Serializer, V> Serializer for State<S, V> {
        type Value = (S::Value, V);
        type Error = S::Error;
        type Map = State<S::Map, V>;
        type List = State<S::List, V>;
        type Structure = State<S::Structure, V>;
        type Enumeration = State<S::Enumeration, V>;

        #[inline]
        fn unit(self) -> Result<Self::Value, Self::Error> {
            Ok((self.0.unit()?, self.1))
        }
        #[inline]
        fn bool(self, value: bool) -> Result<Self::Value, Self::Error> {
            Ok((self.0.bool(value)?, self.1))
        }
        #[inline]
        fn char(self, value: char) -> Result<Self::Value, Self::Error> {
            Ok((self.0.char(value)?, self.1))
        }
        #[inline]
        fn u8(self, value: u8) -> Result<Self::Value, Self::Error> {
            Ok((self.0.u8(value)?, self.1))
        }
        #[inline]
        fn u16(self, value: u16) -> Result<Self::Value, Self::Error> {
            Ok((self.0.u16(value)?, self.1))
        }
        #[inline]
        fn u32(self, value: u32) -> Result<Self::Value, Self::Error> {
            Ok((self.0.u32(value)?, self.1))
        }
        #[inline]
        fn u64(self, value: u64) -> Result<Self::Value, Self::Error> {
            Ok((self.0.u64(value)?, self.1))
        }
        #[inline]
        fn u128(self, value: u128) -> Result<Self::Value, Self::Error> {
            Ok((self.0.u128(value)?, self.1))
        }
        #[inline]
        fn usize(self, value: usize) -> Result<Self::Value, Self::Error> {
            Ok((self.0.usize(value)?, self.1))
        }
        #[inline]
        fn i8(self, value: i8) -> Result<Self::Value, Self::Error> {
            Ok((self.0.i8(value)?, self.1))
        }
        #[inline]
        fn i16(self, value: i16) -> Result<Self::Value, Self::Error> {
            Ok((self.0.i16(value)?, self.1))
        }
        #[inline]
        fn i32(self, value: i32) -> Result<Self::Value, Self::Error> {
            Ok((self.0.i32(value)?, self.1))
        }
        #[inline]
        fn i64(self, value: i64) -> Result<Self::Value, Self::Error> {
            Ok((self.0.i64(value)?, self.1))
        }
        #[inline]
        fn i128(self, value: i128) -> Result<Self::Value, Self::Error> {
            Ok((self.0.i128(value)?, self.1))
        }
        #[inline]
        fn isize(self, value: isize) -> Result<Self::Value, Self::Error> {
            Ok((self.0.isize(value)?, self.1))
        }
        #[inline]
        fn f32(self, value: f32) -> Result<Self::Value, Self::Error> {
            Ok((self.0.f32(value)?, self.1))
        }
        #[inline]
        fn f64(self, value: f64) -> Result<Self::Value, Self::Error> {
            Ok((self.0.f64(value)?, self.1))
        }
        #[inline]
        fn list(self) -> Result<Self::List, Self::Error> {
            Ok(State::new(self.0.list()?, self.1))
        }
        #[inline]
        fn map(self) -> Result<Self::Map, Self::Error> {
            Ok(State::new(self.0.map()?, self.1))
        }
        #[inline]
        fn tuple(self) -> Result<Self::List, Self::Error> {
            Ok(State::new(self.0.tuple()?, self.1))
        }
        #[inline]
        fn string(self, value: &str) -> Result<Self::Value, Self::Error> {
            Ok((self.0.string(value)?, self.1))
        }
        #[inline]
        fn slice<T: Serialize>(self, value: &[T]) -> Result<Self::Value, Self::Error> {
            Ok((self.0.slice(value)?, self.1))
        }
        #[inline]
        fn array<T: Serialize, const N: usize>(
            self,
            value: &[T; N],
        ) -> Result<Self::Value, Self::Error> {
            Ok((self.0.array(value)?, self.1))
        }
        #[inline]
        fn bytes(self, value: &[u8]) -> Result<Self::Value, Self::Error> {
            Ok((self.0.bytes(value)?, self.1))
        }
        #[inline]
        fn structure(self) -> Result<Self::Structure, Self::Error> {
            Ok(State::new(self.0.structure()?, self.1))
        }
        #[inline]
        fn enumeration(self) -> Result<Self::Enumeration, Self::Error> {
            Ok(State::new(self.0.enumeration()?, self.1))
        }
    }

    impl<S: Structure, V> Structure for State<S, V> {
        type Value = (S::Value, V);
        type Error = S::Error;
        type Map = State<S::Map, V>;
        type List = State<S::List, V>;

        #[inline]
        fn unit(self) -> Result<Self::Value, Self::Error> {
            Ok((self.0.unit()?, self.1))
        }
        #[inline]
        fn tuple(self) -> Result<Self::List, Self::Error> {
            Ok(State::new(self.0.tuple()?, self.1))
        }
        #[inline]
        fn map(self) -> Result<Self::Map, Self::Error> {
            Ok(State::new(self.0.map()?, self.1))
        }
    }

    impl<S: Map, V> Map for State<S, V> {
        type Value = (S::Value, V);
        type Error = S::Error;

        #[inline]
        fn pair<K: Serialize, T: Serialize>(self, key: K, value: T) -> Result<Self, Self::Error> {
            Ok(State::new(self.0.pair(key, value)?, self.1))
        }
        #[inline]
        fn end(self) -> Result<Self::Value, Self::Error> {
            Ok((self.0.end()?, self.1))
        }
        #[inline]
        fn pairs<K: Serialize, T: Serialize, I: IntoIterator<Item = (K, T)>>(
            self,
            pairs: I,
        ) -> Result<Self::Value, Self::Error>
        where
            Self: Sized,
        {
            Ok((self.0.pairs(pairs)?, self.1))
        }
    }

    impl<S: List, V> List for State<S, V> {
        type Value = (S::Value, V);
        type Error = S::Error;

        #[inline]
        fn item<I: Serialize>(self, item: I) -> Result<Self, Self::Error> {
            Ok(State::new(self.0.item(item)?, self.1))
        }
        #[inline]
        fn end(self) -> Result<Self::Value, Self::Error> {
            Ok((self.0.end()?, self.1))
        }
        #[inline]
        fn items<T: Serialize, I: IntoIterator<Item = T>>(
            self,
            items: I,
        ) -> Result<Self::Value, Self::Error>
        where
            Self: Sized,
        {
            Ok((self.0.items(items)?, self.1))
        }
    }

    impl<S: Enumeration, V> Enumeration for State<S, V> {
        type Value = (S::Value, V);
        type Error = S::Error;
        type Structure = State<S::Structure, V>;

        #[inline]
        fn never(self) -> Result<Self::Value, Self::Error> {
            Ok((self.0.never()?, self.1))
        }
        #[inline]
        fn variant(self, name: &str, index: usize) -> Result<Self::Structure, Self::Error> {
            Ok(State::new(self.0.variant(name, index)?, self.1))
        }
    }
}

pub mod adapt {
    use super::*;

    pub struct Adapt<
        S,
        V,
        E,
        F = fn(Result<<S as Serializer>::Value, <S as Serializer>::Error>) -> Result<V, E>,
    >(S, F, PhantomData<(V, E)>);

    impl<S, V, E, F> Adapt<S, V, E, F> {
        #[inline]
        pub(super) const fn new(serializer: S, map: F) -> Self {
            Self(serializer, map, PhantomData)
        }
    }

    impl<
            S: Serializer,
            V,
            E: From<S::Error>,
            F: FnOnce(Result<S::Value, S::Error>) -> Result<V, E>,
        > Serializer for Adapt<S, V, E, F>
    {
        type Value = V;
        type Error = E;
        type Map = Adapt<S::Map, V, E, F>;
        type List = Adapt<S::List, V, E, F>;
        type Structure = Adapt<S::Structure, V, E, F>;
        type Enumeration = Adapt<S::Enumeration, V, E, F>;

        #[inline]
        fn unit(self) -> Result<Self::Value, Self::Error> {
            self.1(self.0.unit())
        }
        #[inline]
        fn bool(self, value: bool) -> Result<Self::Value, Self::Error> {
            self.1(self.0.bool(value))
        }
        #[inline]
        fn char(self, value: char) -> Result<Self::Value, Self::Error> {
            self.1(self.0.char(value))
        }
        #[inline]
        fn u8(self, value: u8) -> Result<Self::Value, Self::Error> {
            self.1(self.0.u8(value))
        }
        #[inline]
        fn u16(self, value: u16) -> Result<Self::Value, Self::Error> {
            self.1(self.0.u16(value))
        }
        #[inline]
        fn u32(self, value: u32) -> Result<Self::Value, Self::Error> {
            self.1(self.0.u32(value))
        }
        #[inline]
        fn u64(self, value: u64) -> Result<Self::Value, Self::Error> {
            self.1(self.0.u64(value))
        }
        #[inline]
        fn u128(self, value: u128) -> Result<Self::Value, Self::Error> {
            self.1(self.0.u128(value))
        }
        #[inline]
        fn usize(self, value: usize) -> Result<Self::Value, Self::Error> {
            self.1(self.0.usize(value))
        }
        #[inline]
        fn i8(self, value: i8) -> Result<Self::Value, Self::Error> {
            self.1(self.0.i8(value))
        }
        #[inline]
        fn i16(self, value: i16) -> Result<Self::Value, Self::Error> {
            self.1(self.0.i16(value))
        }
        #[inline]
        fn i32(self, value: i32) -> Result<Self::Value, Self::Error> {
            self.1(self.0.i32(value))
        }
        #[inline]
        fn i64(self, value: i64) -> Result<Self::Value, Self::Error> {
            self.1(self.0.i64(value))
        }
        #[inline]
        fn i128(self, value: i128) -> Result<Self::Value, Self::Error> {
            self.1(self.0.i128(value))
        }
        #[inline]
        fn isize(self, value: isize) -> Result<Self::Value, Self::Error> {
            self.1(self.0.isize(value))
        }
        #[inline]
        fn f32(self, value: f32) -> Result<Self::Value, Self::Error> {
            self.1(self.0.f32(value))
        }
        #[inline]
        fn f64(self, value: f64) -> Result<Self::Value, Self::Error> {
            self.1(self.0.f64(value))
        }
        #[inline]
        fn list(self) -> Result<Self::List, Self::Error> {
            Ok(Adapt::new(self.0.list()?, self.1))
        }
        #[inline]
        fn map(self) -> Result<Self::Map, Self::Error> {
            Ok(Adapt::new(self.0.map()?, self.1))
        }
        #[inline]
        fn tuple(self) -> Result<Self::List, Self::Error> {
            Ok(Adapt::new(self.0.tuple()?, self.1))
        }
        #[inline]
        fn string(self, value: &str) -> Result<Self::Value, Self::Error> {
            self.1(self.0.string(value))
        }
        #[inline]
        fn slice<T: Serialize>(self, value: &[T]) -> Result<Self::Value, Self::Error> {
            self.1(self.0.slice(value))
        }
        #[inline]
        fn array<T: Serialize, const N: usize>(
            self,
            value: &[T; N],
        ) -> Result<Self::Value, Self::Error> {
            self.1(self.0.array(value))
        }
        #[inline]
        fn bytes(self, value: &[u8]) -> Result<Self::Value, Self::Error> {
            self.1(self.0.bytes(value))
        }
        #[inline]
        fn structure(self) -> Result<Self::Structure, Self::Error> {
            Ok(Adapt::new(self.0.structure()?, self.1))
        }
        #[inline]
        fn enumeration(self) -> Result<Self::Enumeration, Self::Error> {
            Ok(Adapt::new(self.0.enumeration()?, self.1))
        }
    }

    impl<
            S: Structure,
            V,
            E: From<S::Error>,
            F: FnOnce(Result<S::Value, S::Error>) -> Result<V, E>,
        > Structure for Adapt<S, V, E, F>
    {
        type Value = V;
        type Error = E;
        type Map = Adapt<S::Map, V, E, F>;
        type List = Adapt<S::List, V, E, F>;

        #[inline]
        fn unit(self) -> Result<Self::Value, Self::Error> {
            self.1(self.0.unit())
        }
        #[inline]
        fn tuple(self) -> Result<Self::List, Self::Error> {
            Ok(Adapt::new(self.0.tuple()?, self.1))
        }
        #[inline]
        fn map(self) -> Result<Self::Map, Self::Error> {
            Ok(Adapt::new(self.0.map()?, self.1))
        }
    }

    impl<S: Map, V, E: From<S::Error>, F: FnOnce(Result<S::Value, S::Error>) -> Result<V, E>> Map
        for Adapt<S, V, E, F>
    {
        type Value = V;
        type Error = E;

        #[inline]
        fn pair<K: Serialize, T: Serialize>(self, key: K, value: T) -> Result<Self, Self::Error> {
            Ok(Adapt::new(self.0.pair(key, value)?, self.1))
        }
        #[inline]
        fn end(self) -> Result<Self::Value, Self::Error> {
            self.1(self.0.end())
        }
        #[inline]
        fn pairs<K: Serialize, T: Serialize, I: IntoIterator<Item = (K, T)>>(
            self,
            pairs: I,
        ) -> Result<Self::Value, Self::Error>
        where
            Self: Sized,
        {
            self.1(self.0.pairs(pairs))
        }
    }

    impl<S: List, V, E: From<S::Error>, F: FnOnce(Result<S::Value, S::Error>) -> Result<V, E>> List
        for Adapt<S, V, E, F>
    {
        type Value = V;
        type Error = E;

        #[inline]
        fn item<I: Serialize>(self, item: I) -> Result<Self, Self::Error> {
            Ok(Adapt::new(self.0.item(item)?, self.1))
        }
        #[inline]
        fn end(self) -> Result<Self::Value, Self::Error> {
            self.1(self.0.end())
        }
        #[inline]
        fn items<T: Serialize, I: IntoIterator<Item = T>>(
            self,
            items: I,
        ) -> Result<Self::Value, Self::Error>
        where
            Self: Sized,
        {
            self.1(self.0.items(items))
        }
    }

    impl<
            S: Enumeration,
            V,
            E: From<S::Error>,
            F: FnOnce(Result<S::Value, S::Error>) -> Result<V, E>,
        > Enumeration for Adapt<S, V, E, F>
    {
        type Value = V;
        type Error = E;
        type Structure = Adapt<S::Structure, V, E, F>;

        #[inline]
        fn never(self) -> Result<Self::Value, Self::Error> {
            self.1(self.0.never())
        }
        #[inline]
        fn variant(self, name: &str, index: usize) -> Result<Self::Structure, Self::Error> {
            Ok(Adapt::new(self.0.variant(name, index)?, self.1))
        }
    }
}

use self::adapt::*;
use crate::{recurse, serialize::*};
use std::marker::PhantomData;

pub trait Serializer: Sized {
    type Value;
    type Error;
    type Map: Map<Value = Self::Value, Error = Self::Error>;
    type List: List<Value = Self::Value, Error = Self::Error>;
    type Structure: Structure<Value = Self::Value, Error = Self::Error>;
    type Enumeration: Enumeration<Value = Self::Value, Error = Self::Error>;

    fn unit(self) -> Result<Self::Value, Self::Error>;
    fn bool(self, value: bool) -> Result<Self::Value, Self::Error>;
    fn char(self, value: char) -> Result<Self::Value, Self::Error>;
    fn u8(self, value: u8) -> Result<Self::Value, Self::Error>;
    fn u16(self, value: u16) -> Result<Self::Value, Self::Error>;
    fn u32(self, value: u32) -> Result<Self::Value, Self::Error>;
    fn u64(self, value: u64) -> Result<Self::Value, Self::Error>;
    fn usize(self, value: usize) -> Result<Self::Value, Self::Error>;
    fn u128(self, value: u128) -> Result<Self::Value, Self::Error>;
    fn i8(self, value: i8) -> Result<Self::Value, Self::Error>;
    fn i16(self, value: i16) -> Result<Self::Value, Self::Error>;
    fn i32(self, value: i32) -> Result<Self::Value, Self::Error>;
    fn i64(self, value: i64) -> Result<Self::Value, Self::Error>;
    fn isize(self, value: isize) -> Result<Self::Value, Self::Error>;
    fn i128(self, value: i128) -> Result<Self::Value, Self::Error>;
    fn f32(self, value: f32) -> Result<Self::Value, Self::Error>;
    fn f64(self, value: f64) -> Result<Self::Value, Self::Error>;

    fn shared<T: ?Sized>(self, value: &T) -> Result<Self::Value, Self::Error>;
    fn exclusive<T: ?Sized>(self, value: &mut T) -> Result<Self::Value, Self::Error>;
    fn constant<T: ?Sized>(self, value: *const T) -> Result<Self::Value, Self::Error>;
    fn mutable<T: ?Sized>(self, value: *mut T) -> Result<Self::Value, Self::Error>;

    fn list(self, capacity: usize) -> Result<Self::List, Self::Error>;
    fn map(self, capacity: usize) -> Result<Self::Map, Self::Error>;

    #[inline]
    fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error> {
        self.list(N)
    }
    #[inline]
    fn string(self, value: &str) -> Result<Self::Value, Self::Error> {
        self.list(value.len())?.items(value.chars())
    }
    #[inline]
    fn slice<T: Serialize>(self, value: &[T]) -> Result<Self::Value, Self::Error> {
        self.list(value.len())?.items(value)
    }
    #[inline]
    fn array<T: Serialize, const N: usize>(
        self,
        value: &[T; N],
    ) -> Result<Self::Value, Self::Error> {
        self.list(value.len())?.items(value)
    }
    #[inline]
    fn bytes(self, value: &[u8]) -> Result<Self::Value, Self::Error> {
        self.list(value.len())?.items(value)
    }

    fn structure<T: ?Sized>(self) -> Result<Self::Structure, Self::Error>;
    fn enumeration<T: ?Sized>(self) -> Result<Self::Enumeration, Self::Error>;

    #[inline]
    fn adapt<T, E: From<Self::Error>, F: FnOnce(Result<Self::Value, Self::Error>) -> Result<T, E>>(
        self,
        map: F,
    ) -> Adapt<Self, T, E, F>
    where
        Self: Sized,
    {
        Adapt::new(self, map)
    }
}

pub trait Structure {
    type Value;
    type Error;
    type Map: Map<Value = Self::Value, Error = Self::Error>;
    type List: List<Value = Self::Value, Error = Self::Error>;

    fn unit(self) -> Result<Self::Value, Self::Error>;
    fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error>;
    fn map<const N: usize>(self) -> Result<Self::Map, Self::Error>;
}

pub trait Enumeration {
    type Value;
    type Error;
    type Structure: Structure<Value = Self::Value, Error = Self::Error>;

    fn never(self) -> Result<Self::Value, Self::Error>;
    fn variant<const I: usize, const N: usize>(
        self,
        name: &'static str,
    ) -> Result<Self::Structure, Self::Error>;
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
    ) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
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
    ) -> Result<Self::Value, Self::Error>
    where
        Self: Sized,
    {
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
            fn shared<T: ?Sized>(self, _value: &T) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.shared(_value)?,)*))
            }
            #[inline]
            fn exclusive<T: ?Sized>(self, _value: &mut T) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.exclusive(_value)?,)*))
            }
            #[inline]
            fn constant<T: ?Sized>(self, _value: *const T) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.constant(_value)?,)*))
            }
            #[inline]
            fn mutable<T: ?Sized>(self, _value: *mut T) -> Result<Self::Value, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.mutable(_value)?,)*))
            }
            #[inline]
            fn list(self, _capacity: usize) -> Result<Self::List, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.list(_capacity)?,)*))
            }
            #[inline]
            fn map(self, _capacity: usize) -> Result<Self::Map, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.map(_capacity)?,)*))
            }
            #[inline]
            fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.tuple::<N>()?,)*))
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
            fn structure<T: ?Sized>(self) -> Result<Self::Structure, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.structure::<T>()?,)*))
            }
            #[inline]
            fn enumeration<T: ?Sized>(self) -> Result<Self::Enumeration, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.enumeration::<T>()?,)*))
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
            fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.tuple::<N>()?,)*))
            }
            #[inline]
            fn map<const N: usize>(self) -> Result<Self::Map, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.map::<N>()?,)*))
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
            fn variant<const I: usize, const N: usize>(
                self,
                _name: &'static str,
            ) -> Result<Self::Structure, Self::Error> {
                let ($($p,)*) = self;
                Ok(($($p.variant::<I, N>(_name)?,)*))
            }
        }
    };
}

recurse!(tuple);

mod adapt {
    use super::*;

    pub struct Adapt<S, V, E, F>(S, F, PhantomData<(V, E)>);

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
        fn shared<T: ?Sized>(self, value: &T) -> Result<Self::Value, Self::Error> {
            self.1(self.0.shared(value))
        }
        #[inline]
        fn exclusive<T: ?Sized>(self, value: &mut T) -> Result<Self::Value, Self::Error> {
            self.1(self.0.exclusive(value))
        }
        #[inline]
        fn constant<T: ?Sized>(self, value: *const T) -> Result<Self::Value, Self::Error> {
            self.1(self.0.constant(value))
        }
        #[inline]
        fn mutable<T: ?Sized>(self, value: *mut T) -> Result<Self::Value, Self::Error> {
            self.1(self.0.mutable(value))
        }
        #[inline]
        fn list(self, capacity: usize) -> Result<Self::List, Self::Error> {
            Ok(Adapt::new(self.0.list(capacity)?, self.1))
        }
        #[inline]
        fn map(self, capacity: usize) -> Result<Self::Map, Self::Error> {
            Ok(Adapt::new(self.0.map(capacity)?, self.1))
        }
        #[inline]
        fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error> {
            Ok(Adapt::new(self.0.tuple::<N>()?, self.1))
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
        fn structure<T: ?Sized>(self) -> Result<Self::Structure, Self::Error> {
            Ok(Adapt::new(self.0.structure::<T>()?, self.1))
        }
        #[inline]
        fn enumeration<T: ?Sized>(self) -> Result<Self::Enumeration, Self::Error> {
            Ok(Adapt::new(self.0.enumeration::<T>()?, self.1))
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
        fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error> {
            Ok(Adapt::new(self.0.tuple::<N>()?, self.1))
        }
        #[inline]
        fn map<const N: usize>(self) -> Result<Self::Map, Self::Error> {
            Ok(Adapt::new(self.0.map::<N>()?, self.1))
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
        fn variant<const I: usize, const N: usize>(
            self,
            name: &'static str,
        ) -> Result<Self::Structure, Self::Error> {
            Ok(Adapt::new(self.0.variant::<I, N>(name)?, self.1))
        }
    }
}

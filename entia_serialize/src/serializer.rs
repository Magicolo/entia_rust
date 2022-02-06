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
    #[inline]
    fn bool(self, value: bool) -> Result<Self::Value, Self::Error> {
        self.u8(value as _)
    }
    #[inline]
    fn char(self, value: char) -> Result<Self::Value, Self::Error> {
        self.u32(value as _)
    }
    #[inline]
    fn u8(self, value: u8) -> Result<Self::Value, Self::Error> {
        self.u16(value as _)
    }
    #[inline]
    fn u16(self, value: u16) -> Result<Self::Value, Self::Error> {
        self.u32(value as _)
    }
    #[inline]
    fn u32(self, value: u32) -> Result<Self::Value, Self::Error> {
        self.u64(value as _)
    }
    #[inline]
    fn u64(self, value: u64) -> Result<Self::Value, Self::Error> {
        self.u128(value as _)
    }
    #[inline]
    fn usize(self, value: usize) -> Result<Self::Value, Self::Error> {
        self.u128(value as _)
    }
    fn u128(self, value: u128) -> Result<Self::Value, Self::Error>;

    #[inline]
    fn i8(self, value: i8) -> Result<Self::Value, Self::Error> {
        self.i16(value as _)
    }
    #[inline]
    fn i16(self, value: i16) -> Result<Self::Value, Self::Error> {
        self.i32(value as _)
    }
    #[inline]
    fn i32(self, value: i32) -> Result<Self::Value, Self::Error> {
        self.i64(value as _)
    }
    #[inline]
    fn i64(self, value: i64) -> Result<Self::Value, Self::Error> {
        self.i128(value as _)
    }
    #[inline]
    fn isize(self, value: isize) -> Result<Self::Value, Self::Error> {
        self.i128(value as _)
    }
    fn i128(self, value: i128) -> Result<Self::Value, Self::Error>;
    #[inline]
    fn f32(self, value: f32) -> Result<Self::Value, Self::Error> {
        self.f64(value as _)
    }
    fn f64(self, value: f64) -> Result<Self::Value, Self::Error>;

    fn shared<T: ?Sized>(self, value: &T) -> Result<Self::Value, Self::Error>;
    #[inline]
    fn exclusive<T: ?Sized>(self, value: &mut T) -> Result<Self::Value, Self::Error> {
        self.shared(value)
    }

    fn constant<T: ?Sized>(self, value: *const T) -> Result<Self::Value, Self::Error>;
    #[inline]
    fn mutable<T: ?Sized>(self, value: *mut T) -> Result<Self::Value, Self::Error> {
        self.constant(value)
    }

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

    // #[inline]
    // fn map<T, F: FnMut(Self::Value) -> T>(self, map: F) -> map::Map<Self, T, F>
    // where
    //     Self: Sized,
    // {
    //     map::Map::new(self, map)
    // }
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

mod map {
    use super::*;

    pub struct Map<S, T, F>(S, F, PhantomData<T>);

    impl<S, T, F> Map<S, T, F> {
        #[inline]
        pub(super) const fn new(serializer: S, map: F) -> Self {
            Self(serializer, map, PhantomData)
        }
    }

    impl<S: Serializer, T, F: FnMut(S::Value) -> T> Serializer for Map<S, T, F> {
        type Value = T;
        type Error = S::Error;
        type Map = Map<S::Map, T, F>;
        type List = Map<S::List, T, F>;
        type Structure = Map<S::Structure, T, F>;
        type Enumeration = Map<S::Enumeration, T, F>;

        #[inline]
        fn unit(self) -> Result<Self::Value, Self::Error> {
            self.0.unit().map(self.1)
        }
        #[inline]
        fn bool(self, value: bool) -> Result<Self::Value, Self::Error> {
            self.0.bool(value).map(self.1)
        }
        #[inline]
        fn char(self, value: char) -> Result<Self::Value, Self::Error> {
            self.0.char(value).map(self.1)
        }
        #[inline]
        fn u8(self, value: u8) -> Result<Self::Value, Self::Error> {
            self.0.u8(value).map(self.1)
        }
        #[inline]
        fn u16(self, value: u16) -> Result<Self::Value, Self::Error> {
            self.0.u16(value).map(self.1)
        }
        #[inline]
        fn u32(self, value: u32) -> Result<Self::Value, Self::Error> {
            self.0.u32(value).map(self.1)
        }
        #[inline]
        fn u64(self, value: u64) -> Result<Self::Value, Self::Error> {
            self.0.u64(value).map(self.1)
        }
        #[inline]
        fn u128(self, value: u128) -> Result<Self::Value, Self::Error> {
            self.0.u128(value).map(self.1)
        }
        #[inline]
        fn usize(self, value: usize) -> Result<Self::Value, Self::Error> {
            self.0.usize(value).map(self.1)
        }
        #[inline]
        fn i8(self, value: i8) -> Result<Self::Value, Self::Error> {
            self.0.i8(value).map(self.1)
        }
        #[inline]
        fn i16(self, value: i16) -> Result<Self::Value, Self::Error> {
            self.0.i16(value).map(self.1)
        }
        #[inline]
        fn i32(self, value: i32) -> Result<Self::Value, Self::Error> {
            self.0.i32(value).map(self.1)
        }
        #[inline]
        fn i64(self, value: i64) -> Result<Self::Value, Self::Error> {
            self.0.i64(value).map(self.1)
        }
        #[inline]
        fn i128(self, value: i128) -> Result<Self::Value, Self::Error> {
            self.0.i128(value).map(self.1)
        }
        #[inline]
        fn isize(self, value: isize) -> Result<Self::Value, Self::Error> {
            self.0.isize(value).map(self.1)
        }
        #[inline]
        fn f32(self, value: f32) -> Result<Self::Value, Self::Error> {
            self.0.f32(value).map(self.1)
        }
        #[inline]
        fn f64(self, value: f64) -> Result<Self::Value, Self::Error> {
            self.0.f64(value).map(self.1)
        }
        #[inline]
        fn shared<V: ?Sized>(self, value: &V) -> Result<Self::Value, Self::Error> {
            self.0.shared(value).map(self.1)
        }
        #[inline]
        fn exclusive<V: ?Sized>(self, value: &mut V) -> Result<Self::Value, Self::Error> {
            self.0.exclusive(value).map(self.1)
        }
        #[inline]
        fn constant<V: ?Sized>(self, value: *const V) -> Result<Self::Value, Self::Error> {
            self.0.constant(value).map(self.1)
        }
        #[inline]
        fn mutable<V: ?Sized>(self, value: *mut V) -> Result<Self::Value, Self::Error> {
            self.0.mutable(value).map(self.1)
        }
        #[inline]
        fn list(self, capacity: usize) -> Result<Self::List, Self::Error> {
            Ok(Map::new(self.0.list(capacity)?, self.1))
        }
        #[inline]
        fn map(self, capacity: usize) -> Result<Self::Map, Self::Error> {
            Ok(Map::new(self.0.map(capacity)?, self.1))
        }
        #[inline]
        fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error> {
            Ok(Map::new(self.0.tuple::<N>()?, self.1))
        }
        #[inline]
        fn string(self, value: &str) -> Result<Self::Value, Self::Error> {
            self.0.string(value).map(self.1)
        }
        #[inline]
        fn slice<V: Serialize>(self, value: &[V]) -> Result<Self::Value, Self::Error> {
            self.0.slice(value).map(self.1)
        }
        #[inline]
        fn array<V: Serialize, const N: usize>(
            self,
            value: &[V; N],
        ) -> Result<Self::Value, Self::Error> {
            self.0.array(value).map(self.1)
        }
        #[inline]
        fn bytes(self, value: &[u8]) -> Result<Self::Value, Self::Error> {
            self.0.bytes(value).map(self.1)
        }
        #[inline]
        fn structure<V: ?Sized>(self) -> Result<Self::Structure, Self::Error> {
            Ok(Map::new(self.0.structure::<V>()?, self.1))
        }
        #[inline]
        fn enumeration<V: ?Sized>(self) -> Result<Self::Enumeration, Self::Error> {
            Ok(Map::new(self.0.enumeration::<V>()?, self.1))
        }
    }

    impl<S: Structure, T, F: FnMut(S::Value) -> T> Structure for Map<S, T, F> {
        type Value = T;
        type Error = S::Error;
        type Map = Map<S::Map, T, F>;
        type List = Map<S::List, T, F>;

        #[inline]
        fn unit(self) -> Result<Self::Value, Self::Error> {
            self.0.unit().map(self.1)
        }
        #[inline]
        fn tuple<const N: usize>(self) -> Result<Self::List, Self::Error> {
            Ok(Map::new(self.0.tuple::<N>()?, self.1))
        }
        #[inline]
        fn map<const N: usize>(self) -> Result<Self::Map, Self::Error> {
            Ok(Map::new(self.0.map::<N>()?, self.1))
        }
    }

    impl<S: super::Map, T, F: FnMut(S::Value) -> T> super::Map for Map<S, T, F> {
        type Value = T;
        type Error = S::Error;

        #[inline]
        fn pair<K: Serialize, V: Serialize>(self, key: K, value: V) -> Result<Self, Self::Error> {
            Ok(Map::new(self.0.pair(key, value)?, self.1))
        }
        #[inline]
        fn end(self) -> Result<Self::Value, Self::Error> {
            self.0.end().map(self.1)
        }
        #[inline]
        fn pairs<K: Serialize, V: Serialize, I: IntoIterator<Item = (K, V)>>(
            self,
            pairs: I,
        ) -> Result<Self::Value, Self::Error>
        where
            Self: Sized,
        {
            self.0.pairs(pairs).map(self.1)
        }
    }

    impl<S: List, T, F: FnMut(S::Value) -> T> List for Map<S, T, F> {
        type Value = T;
        type Error = S::Error;

        #[inline]
        fn item<I: Serialize>(self, item: I) -> Result<Self, Self::Error> {
            Ok(Map::new(self.0.item(item)?, self.1))
        }
        #[inline]
        fn end(self) -> Result<Self::Value, Self::Error> {
            self.0.end().map(self.1)
        }
        #[inline]
        fn items<V: Serialize, I: IntoIterator<Item = V>>(
            self,
            items: I,
        ) -> Result<Self::Value, Self::Error>
        where
            Self: Sized,
        {
            self.0.items(items).map(self.1)
        }
    }

    impl<S: Enumeration, T, F: FnMut(S::Value) -> T> Enumeration for Map<S, T, F> {
        type Value = T;
        type Error = S::Error;
        type Structure = Map<S::Structure, T, F>;

        #[inline]
        fn never(self) -> Result<Self::Value, Self::Error> {
            self.0.never().map(self.1)
        }
        #[inline]
        fn variant<const I: usize, const N: usize>(
            self,
            name: &'static str,
        ) -> Result<Self::Structure, Self::Error> {
            Ok(Map::new(self.0.variant::<I, N>(name)?, self.1))
        }
    }
}

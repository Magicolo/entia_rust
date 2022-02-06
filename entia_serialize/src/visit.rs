use crate::recurse;
use entia_macro::count;
use std::{
    marker::PhantomData,
    ops::{Bound, Range},
};

pub trait Visit {
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result;
}

pub trait Visitor {
    type Result;
    type Primitive: Primitive<Result = Self::Result>;
    type Structure: Structure<Result = Self::Result>;
    type Enumeration: Enumeration<Result = Self::Result>;
    type Sequence: Sequence<Result = Self::Result>;

    fn primitive(self) -> Self::Primitive;
    fn structure(self, name: &'static str) -> Self::Structure;
    fn enumeration(self, name: &'static str) -> Self::Enumeration;
    fn sequence(self) -> Self::Sequence;
}

pub trait Primitive: Sized {
    type Result;

    fn unit(self) -> Self::Result;
    fn never(self) -> Self::Result;
    fn bool(self, value: bool) -> Self::Result;
    fn char(self, value: char) -> Self::Result;
    fn u8(self, value: u8) -> Self::Result;
    fn u16(self, value: u16) -> Self::Result;
    fn u32(self, value: u32) -> Self::Result;
    fn u64(self, value: u64) -> Self::Result;
    fn u128(self, value: u128) -> Self::Result;
    fn usize(self, value: usize) -> Self::Result;
    fn i8(self, value: i8) -> Self::Result;
    fn i16(self, value: i16) -> Self::Result;
    fn i32(self, value: i32) -> Self::Result;
    fn i64(self, value: i64) -> Self::Result;
    fn i128(self, value: i128) -> Self::Result;
    fn isize(self, value: isize) -> Self::Result;
    fn f32(self, value: f32) -> Self::Result;
    fn f64(self, value: f64) -> Self::Result;

    fn shared<T: ?Sized>(self, value: &T) -> Self::Result;
    #[inline]
    fn exclusive<T: ?Sized>(self, value: &mut T) -> Self::Result {
        self.shared(value)
    }

    fn constant<T: ?Sized>(self, value: *const T) -> Self::Result;
    #[inline]
    fn mutable<T: ?Sized>(self, value: *mut T) -> Self::Result {
        self.constant(value)
    }
}

pub trait Structure {
    type Result;
    type Fields: Fields + Into<Self::Result>;
    type Items: Items + Into<Self::Result>;

    fn unit(self) -> Self::Result;
    fn tuple<const N: usize>(self) -> Self::Items;
    fn map<const N: usize>(self) -> Self::Fields;
}

pub trait Enumeration {
    type Result;
    type Variant: Variant<Result = Self::Result>;

    fn never(self) -> Self::Result;
    fn variant<const N: usize>(self) -> Self::Variant;
}

pub trait Sequence: Sized {
    type Result;
    type Fields: Fields + Into<Self::Result>;
    type Items: Items + Into<Self::Result>;

    fn list(self, capacity: usize) -> Self::Items;
    fn map(self, capacity: usize) -> Self::Fields;

    #[inline]
    fn items<T: Visit, I: IntoIterator<Item = T>>(self, items: I) -> Self::Result {
        let iterator = items.into_iter();
        let (low, _) = iterator.size_hint();
        let items = self.list(low);
        iterator
            .fold(items, |items, value| items.item(value))
            .into()
    }

    #[inline]
    fn fields<K: Visit, V: Visit, I: IntoIterator<Item = (K, V)>>(self, fields: I) -> Self::Result {
        let iterator = fields.into_iter();
        let (low, _) = iterator.size_hint();
        let fields = self.map(low);
        iterator
            .fold(fields, |fields, (key, value)| fields.field(key, value))
            .into()
    }

    #[inline]
    fn tuple<const N: usize>(self) -> Self::Items {
        self.list(N)
    }
    #[inline]
    fn string(self, value: &str) -> Self::Result {
        self.items(value.chars())
    }
    #[inline]
    fn slice<T: Visit>(self, value: &[T]) -> Self::Result {
        self.items(value)
    }
    #[inline]
    fn array<T: Visit, const N: usize>(self, value: &[T; N]) -> Self::Result {
        self.slice(value)
    }
    #[inline]
    fn bytes(self, value: &[u8]) -> Self::Result {
        self.slice(value)
    }
}

pub trait Fields {
    fn field<K: Visit, V: Visit>(self, key: K, value: V) -> Self;
}

pub trait Items {
    fn item<T: Visit>(self, value: T) -> Self;
}

pub trait Variant {
    type Result;
    type Fields: Fields + Into<Self::Result>;
    type Items: Items + Into<Self::Result>;

    fn unit(self, name: &'static str, index: usize) -> Self::Result;
    fn tuple<const N: usize>(self, name: &'static str, index: usize) -> Self::Items;
    fn map<const N: usize>(self, name: &'static str, index: usize) -> Self::Fields;
}

impl<T: Visit + ?Sized> Visit for &T {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        (&**self).visit(visitor)
    }
}

impl<T: Visit + ?Sized> Visit for &mut T {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        (&**self).visit(visitor)
    }
}

impl<T: Visit> Visit for Option<T> {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        let variant = visitor.enumeration("Option").variant::<2>();
        match self {
            None => variant.unit("None", 0),
            Some(value) => variant.tuple::<1>("Some", 1).item(value).into(),
        }
    }
}

impl<T: Visit, E: Visit> Visit for Result<T, E> {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        let variant = visitor.enumeration("Result").variant::<2>();
        match self {
            Ok(value) => variant.tuple::<1>("Ok", 0).item(value).into(),
            Err(error) => variant.tuple::<1>("Err", 1).item(error).into(),
        }
    }
}

impl<T: ?Sized> Visit for PhantomData<T> {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        visitor.structure("PhantomData").unit()
    }
}

impl Visit for str {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        visitor.sequence().string(self)
    }
}

impl Visit for String {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        visitor.sequence().string(self)
    }
}

impl<T: Visit> Visit for Vec<T> {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        visitor.sequence().slice(self)
    }
}

impl<T: Visit> Visit for [T] {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        visitor.sequence().slice(self)
    }
}

impl<T: Visit, const N: usize> Visit for [T; N] {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        visitor.sequence().array(self)
    }
}

impl<T: Visit> Visit for Range<T> {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        visitor
            .structure("Range")
            .map::<2>()
            .field("start", &self.start)
            .field("end", &self.end)
            .into()
    }
}

impl<T: Visit> Visit for Bound<T> {
    #[inline]
    fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
        let variant = visitor.enumeration("Bound").variant::<3>();
        match self {
            Bound::Included(value) => variant.tuple::<1>("Included", 0).item(value).into(),
            Bound::Excluded(value) => variant.tuple::<1>("Excluded", 1).item(value).into(),
            Bound::Unbounded => variant.unit("Unbounded", 2),
        }
    }
}

macro_rules! primitive {
    ($t:ident) => {
        impl Visit for $t {
            #[inline]
            fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
                visitor.primitive().$t(*self)
            }
        }
    };
    ($t:ident) => {
        impl Visit for $t {
            #[inline]
            fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
                visitor.primitive(|primitive| primitive.$t(*self))
            }
        }
    };
    ($($t:ident),*) => {
        $(primitive!($t);)*
    }
}

primitive!(bool, char, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);

macro_rules! tuple {
    () => {
        impl Visit for () {
            #[inline]
            fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
                visitor.primitive().unit()
            }
        }
    };
    ($p:ident, $t:ident $(, $ps:ident, $ts:ident)*) => {
        impl<$t: Visit, $($ts: Visit,)*> Visit for ($t, $($ts,)*) {
            #[inline]
            fn visit<V: Visitor>(&self, visitor: V) -> V::Result {
                let ($p, $($ps,)*) = self;
                visitor.sequence().tuple::<{ count!($t $(,$ts)*) }>().item($p) $(.item($ps))* .into()
            }
        }
    };
}

recurse!(tuple);

pub mod unit {
    use super::*;

    impl Visitor for () {
        type Result = ();
        type Enumeration = ();
        type Primitive = ();
        type Sequence = ();
        type Structure = ();

        #[inline]
        fn primitive(self) -> Self::Primitive {}
        #[inline]
        fn structure(self, _: &'static str) -> Self::Structure {}
        #[inline]
        fn enumeration(self, _: &'static str) -> Self::Enumeration {}
        #[inline]
        fn sequence(self) -> Self::Sequence {}
    }

    impl Enumeration for () {
        type Result = ();
        type Variant = ();

        #[inline]
        fn never(self) -> Self::Result {}
        #[inline]
        fn variant<const N: usize>(self) -> Self::Variant {}
    }

    impl Structure for () {
        type Result = ();
        type Fields = ();
        type Items = ();

        #[inline]
        fn unit(self) -> Self::Result {}
        #[inline]
        fn tuple<const N: usize>(self) -> Self::Items {}
        #[inline]
        fn map<const N: usize>(self) -> Self::Fields {}
    }

    impl Sequence for () {
        type Result = ();
        type Fields = ();
        type Items = ();

        #[inline]
        fn list(self, _: usize) -> Self::Items {}
        #[inline]
        fn map(self, _: usize) -> Self::Fields {}
    }

    impl Primitive for () {
        type Result = ();

        #[inline]
        fn unit(self) -> Self::Result {}
        #[inline]
        fn never(self) -> Self::Result {}
        #[inline]
        fn bool(self, _: bool) -> Self::Result {}
        #[inline]
        fn char(self, _: char) -> Self::Result {}
        #[inline]
        fn u8(self, _: u8) -> Self::Result {}
        #[inline]
        fn u16(self, _: u16) -> Self::Result {}
        #[inline]
        fn u32(self, _: u32) -> Self::Result {}
        #[inline]
        fn u64(self, _: u64) -> Self::Result {}
        #[inline]
        fn u128(self, _: u128) -> Self::Result {}
        #[inline]
        fn usize(self, _: usize) -> Self::Result {}
        #[inline]
        fn i8(self, _: i8) -> Self::Result {}
        #[inline]
        fn i16(self, _: i16) -> Self::Result {}
        #[inline]
        fn i32(self, _: i32) -> Self::Result {}
        #[inline]
        fn i64(self, _: i64) -> Self::Result {}
        #[inline]
        fn i128(self, _: i128) -> Self::Result {}
        #[inline]
        fn isize(self, _: isize) -> Self::Result {}
        #[inline]
        fn f32(self, _: f32) -> Self::Result {}
        #[inline]
        fn f64(self, _: f64) -> Self::Result {}
        #[inline]
        fn shared<T: ?Sized>(self, _: &T) -> Self::Result {}
        #[inline]
        fn exclusive<T: ?Sized>(self, _: &mut T) -> Self::Result {}
        #[inline]
        fn constant<T: ?Sized>(self, _: *const T) -> Self::Result {}
        #[inline]
        fn mutable<T: ?Sized>(self, _: *mut T) -> Self::Result {}
    }

    impl Fields for () {
        #[inline]
        fn field<K: Visit, V: Visit>(self, _: K, _: V) -> Self {
            self
        }
    }

    impl Items for () {
        #[inline]
        fn item<T: Visit>(self, _: T) -> Self {
            self
        }
    }

    impl Variant for () {
        type Fields = ();
        type Items = ();
        type Result = ();

        #[inline]
        fn unit(self, _: &'static str, _: usize) -> Self::Result {}
        #[inline]
        fn tuple<const N: usize>(self, _: &'static str, _: usize) -> Self::Result {}
        #[inline]
        fn map<const N: usize>(self, _: &'static str, _: usize) -> Self::Result {}
    }
}

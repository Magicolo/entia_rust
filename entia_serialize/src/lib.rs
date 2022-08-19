pub mod deserialize;
pub mod deserializer;
pub mod json;
pub mod meta;
pub mod node;
pub mod serialize;
pub mod serializer;

pub(crate) use entia_macro::tuples_16 as tuples;

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

mod example {
    use super::*;
    use crate::{deserialize::*, deserializer::*, meta::Meta};

    impl<T> Meta<T> for Boba {
        fn meta() -> &'static T {
            todo!()
        }
    }

    impl<T> Meta<T> for Fett {
        fn meta() -> &'static T {
            todo!()
        }
    }

    macro_rules! new_map {
        ($f:expr, $l:expr $(,$p:ident, $n:expr, $t:ty)*) => {{
            $(let mut $p = None;)*
            let key = &mut [0 as u8; $l];
            while let Some((key, item)) = $f.pair(&mut key[..])? {
                match &*key {
                    $($n => $p = Some(item.value(New::<$t>::new())?),)*
                    _ => item.excess()?,
                }
            }
            ($(match $p { Some($p) => $p, None => $f.miss(New::<$t>::new())? },)*)
        }};
    }

    macro_rules! use_map {
        ($f:expr, $l:expr $(,$p:ident, $n:expr)*) => {{
            let key = &mut [0 as u8; $l];
            while let Some((key, item)) = $f.pair(&mut key[..])? {
                match &*key {
                    $($n => item.value(&mut *$p)?,)*
                    _ => item.excess()?,
                }
            }
        }};
    }

    impl Deserialize for New<Boba> {
        type Value = Boba;

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            let mut map = deserializer.structure()?.map()?;
            let (a, b, c) = new_map!(map, 1, a, b"a", bool, b, b"b", usize, c, b"c", bool);
            Ok(Boba { a, b, c })
        }
    }

    impl Deserialize for &mut Boba {
        type Value = ();

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            let mut map = deserializer.structure()?.map()?;
            let Boba { a, b, c } = self;
            use_map!(map, 1, a, b"a", b, b"b", c, b"c");
            Ok(())
        }
    }

    impl Deserialize for New<Fett> {
        type Value = Fett;

        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            let key = &mut [0 as u8];
            let (key, variant) = deserializer.enumeration()?.variant(&mut key[..])?;
            match &*key {
                b"A" => {
                    variant.unit("A", 0)?;
                    Ok(Fett::A)
                }
                b"B" => {
                    let mut list = variant.tuple("B", 1)?;
                    let a = value_or_miss(&mut list, New::<bool>::new())?;
                    let b = value_or_miss(&mut list, New::<usize>::new())?;
                    let c = value_or_miss(&mut list, New::<Boba>::new())?;
                    list.drain()?;
                    Ok(Fett::B(a, b, c))
                }
                b"C" => {
                    let mut map = variant.map("C", 2)?;
                    let (a, b, c) = new_map!(map, 1, a, b"a", bool, b, b"b", char, c, b"c", String);
                    Ok(Fett::C { a, b, c })
                }
                _ => variant.miss(self),
            }
        }
    }

    impl Deserialize for &mut Fett {
        type Value = ();

        #[inline]
        fn deserialize<D: Deserializer>(self, deserializer: D) -> Result<Self::Value, D::Error> {
            let key = &mut [0 as u8];
            let (key, variant) = deserializer.enumeration()?.variant(&mut key[..])?;
            match &*key {
                b"A" => {
                    variant.unit("A", 0)?;
                    *self = Fett::A;
                    Ok(())
                }
                b"B" => {
                    let mut list = variant.tuple("B", 1)?;
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
                b"C" => {
                    let mut map = variant.map("C", 2)?;
                    match self {
                        Fett::C { a, b, c } => {
                            use_map!(map, 1, a, b"a", b, b"b", c, b"c");
                        }
                        value => {
                            let (a, b, c) =
                                new_map!(map, 1, a, b"a", bool, b, b"b", char, c, b"c", String);
                            *value = Fett::C { a, b, c };
                        }
                    }
                    Ok(())
                }
                _ => Variant::miss(variant, self),
            }
        }
    }

    #[inline]
    fn value_or_miss<L: List, V: Deserialize>(
        list: &mut L,
        value: V,
    ) -> Result<V::Value, L::Error> {
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
}

pub mod deserialize;
pub mod deserializer;
pub mod json;
pub mod node;
pub mod serialize;
pub mod serializer;

pub(crate) use entia_macro::recurse_16 as recurse;

#[cfg(test)]
mod test;

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
    use crate::{deserialize::*, deserializer::*};
    use std::str;

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

use crate::{
    enumeration::{Enumeration, Variant},
    primitive::Primitive,
    structure::{Field, Structure},
    value::Value,
};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    mem::{size_of, swap},
    ops::Deref,
};

pub struct Index<T: 'static>(pub &'static [T], pub fn(&str) -> Option<usize>);

pub trait Meta<T = Data> {
    fn meta() -> T;
}

#[derive(Clone, Copy)]
pub enum Data {
    Primitive(&'static Primitive),
    Structure(&'static Structure),
    Enumeration(&'static Enumeration),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Access {
    Private,
    Public,
    Crate,
    Super,
}

pub struct Constant {
    pub access: Access,
    pub name: &'static str,
    pub meta: fn() -> Data,
    pub value: &'static (dyn Any + Sync + Send),
    pub attributes: &'static [Attribute],
}

pub struct Static {
    pub access: Access,
    pub name: &'static str,
    pub meta: fn() -> Data,
    pub get: fn() -> &'static dyn Any,
    pub get_mut: Option<unsafe fn() -> &'static mut dyn Any>,
    pub attributes: &'static [Attribute],
}

#[derive(Debug)]
pub struct Attribute {
    pub path: &'static str,
    pub content: &'static str,
}

impl<T: 'static> Index<T> {
    #[inline]
    pub fn get(&self, name: &str) -> Option<&T> {
        Some(&self.0[self.index(name)?])
    }

    #[inline]
    pub fn index(&self, name: &str) -> Option<usize> {
        (self.1)(name)
    }
}

impl<T: 'static> Deref for Index<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Data {
    #[inline]
    pub fn name(self) -> &'static str {
        match self {
            Data::Primitive(primitive) => primitive.name,
            Data::Structure(structure) => structure.name,
            Data::Enumeration(enumeration) => enumeration.name,
        }
    }

    #[inline]
    pub fn identifier(self) -> TypeId {
        match self {
            Data::Primitive(primitive) => primitive.identifier(),
            Data::Structure(structure) => structure.identifier(),
            Data::Enumeration(enumeration) => enumeration.identifier(),
        }
    }

    #[inline]
    pub fn from(self, value: Value) -> Option<Value> {
        match self {
            Data::Primitive(primitive) => primitive.from(value).ok(),
            Data::Structure(structure) => structure.from(value),
            Data::Enumeration(enumeration) => enumeration.from(value),
        }
    }

    #[inline]
    pub fn clone(self, value: &dyn Any) -> Option<Value> {
        match self {
            Data::Primitive(primitive) => primitive.clone(value),
            Data::Structure(structure) => structure.clone(value),
            Data::Enumeration(enumeration) => enumeration.clone(value),
        }
    }
}

impl<T: 'static> Meta for Vec<T> {
    #[inline]
    fn meta() -> Data {
        todo!()
        // Type::Structure(&Structure {
        //     access: Visibility::Public,
        //     name: "Vec",
        //     size: size_of::<Self>(),
        //     identifier: TypeId::of::<Self>,
        //     new: todo!(),
        //     values: todo!(),
        //     generics: Index(
        //         &[
        //         //     generic::Generic::Type(generic::Type {
        //         //     name: "T",
        //         //     constraints: &[constraint::Type::Trait(|| &traits::Meta)],
        //         //     attributes: &[],
        //         // })
        //         ],
        //         |name| match name {
        //             "T" => Some(0),
        //             _ => None,
        //         },
        //     ),
        //     attributes: &[],
        //     fields: Index(&[], |_| None),
        //     functions: Index(&[], |_| None),
        // })
    }
}

impl<T: Meta> Meta for &T {
    #[inline]
    fn meta() -> Data {
        T::meta()
    }
}

impl<T: Meta> Meta for &mut T {
    #[inline]
    fn meta() -> Data {
        T::meta()
    }
}

impl<T: Meta + 'static> Meta for Option<T> {
    #[inline]
    fn meta() -> Data {
        Data::Enumeration(&Enumeration {
            access: Access::Public,
            name: "Option",
            size: size_of::<Self>(),
            identifier: TypeId::of::<Self>,
            index: |value| match value.downcast_ref::<Self>()? {
                None => Some(0),
                Some(_) => Some(1),
            },
            generics: &[
                //     generic::Generic::Type(generic::Type {
                //     name: "T",
                //     constraints: &[constraint::Type::Trait(|| &traits::Meta)],
                //     attributes: &[],
                // })
                ],
            attributes: &[],
            variants: Index(
                &[
                    Variant {
                        name: "None",
                        new: |_| Some(Box::new(Self::None)),
                        values: |instance| match *instance.downcast::<Self>()? {
                            None => Ok([].into()),
                            value => Err(Box::new(value)),
                        },
                        attributes: &[],
                        fields: Index(&[], |_| None),
                    },
                    Variant {
                        name: "Some",
                        new: |values| Some(Box::new(Self::Some(values.next()?.downcast().ok()?))),
                        values: |instance| match *instance.downcast::<Self>()? {
                            Some(value) => Ok([Value::from(value)].into()),
                            value => Err(Box::new(value)),
                        },
                        attributes: &[],
                        fields: Index(
                            &[Field {
                                access: Access::Public,
                                name: "0",
                                meta: T::meta,
                                get: |instance| Some(instance.downcast_ref::<Self>()?),
                                get_mut: |instance| Some(instance.downcast_mut::<Self>()?),
                                set: |instance, value| {
                                    Some(swap(
                                        instance.downcast_mut::<Self>()?.as_mut()?,
                                        value.downcast_mut()?,
                                    ))
                                },
                                attributes: &[],
                            }],
                            |name| match name {
                                "0" => Some(0),
                                "1" => Some(1),
                                _ => None,
                            },
                        ),
                    },
                ],
                |name| match name {
                    "None" | "0" => Some(0),
                    "Some" | "1" => Some(1),
                    _ => None,
                },
            ),
        })
    }
}

impl<T: Meta> Meta for Box<T> {
    #[inline]
    fn meta() -> Data {
        todo!()
    }
}

impl<T1: Meta> Meta for (T1,) {
    #[inline]
    fn meta() -> Data {
        todo!()
    }
}

#[macro_export]
macro_rules! attribute {
    ($n:tt) => {
        $crate::meta::Attribute {
            name: stringify!($n),
            content: "",
        }
    };
    ($n:tt($c:tt)) => {
        $crate::meta::Attribute {
            name: stringify!($n),
            content: stringify!($c),
        }
    };
}

#[macro_export]
macro_rules! structure {
    ($vt:tt $t:ident) => {{
        static META: $crate::meta::Structure = $crate::meta::Structure {
            access: $crate::access!($vt),
            name: stringify!($t),
            size: std::mem::size_of::<$t>(),
            identifier: std::any::TypeId::of::<$t>,
            kind: $crate::meta::Structures::Unit,
            attributes: $crate::meta::Index(&[], |_| None),
            fields: $crate::meta::Index(&[], |_| None),
            generics: $crate::meta::Index(&[], |_| None),
            new: |_| Some(Box::new($t)),
            values: |instance| instance.downcast::<$t>().map(|_| [].into()),
            functions: $crate::meta::Index(&[], |_| None),
        };
        $crate::meta::Type::Structure(&META)
    }};
    ($vt:tt $t:ident { $($vf:tt $k:ident[$i:tt]: $v:ty),* }) => {{
        static META: $crate::meta::Structure = $crate::meta::Structure {
            access: $crate::access!($vt),
            name: stringify!($t),
            size: std::mem::size_of::<$t>(),
            identifier: std::any::TypeId::of::<$t>,
            kind: $crate::meta::Structures::Map,
            attributes: $crate::meta::Index(&[], |_| None),
            fields: $crate::meta::Index(&[$($crate::field!($vf $t($k: $v)),)*], |name| match name { $(stringify!($k) | stringify!($i) => Some($i),)* _ => None, }),
            generics: $crate::meta::Index(&[], |_| None),
            new: |values| Some(Box::new($t { $($k: values.next()?.downcast::<$v>().ok()?,)* })),
            values: |instance| instance.downcast::<$t>().map(|instance| [$($crate::value::Value::from(instance.$k),)*].into()),
            functions: $crate::meta::Index(&[], |_| None),
        };
        $crate::meta::Type::Structure(&META)
    }};
    ($vt:tt $t:ident($($vi:tt $i:tt: $v:ty),*)) => {{
        static META: $crate::meta::Structure = $crate::meta::Structure {
            access: $crate::access!($vt),
            name: stringify!($t),
            size: std::mem::size_of::<$t>(),
            identifier: std::any::TypeId::of::<$t>,
            kind: $crate::meta::Structures::Tuple,
            attributes: $crate::meta::Index(&[], |_| None),
            fields: $crate::meta::Index(&[$($crate::field!($vi $t($i: $v)),)*], |name| match name { $(stringify!($i) => Some($i),)* _ => None, }),
            generics: $crate::meta::Index(&[], |_| None),
            new: |values| Some(Box::new($t($(values.next()?.downcast::<$v>().ok()?,)*))),
            values: |instance| instance.downcast::<$t>().map(|instance| [$($crate::value::Value::from(instance.$i),)*].into()),
            functions: $crate::meta::Index(&[], |_| None),
        };
        $crate::meta::Type::Structure(&META)
    }};
}

#[macro_export]
macro_rules! access {
    (pub(super)) => {
        $crate::meta::Visibility::Super
    };
    (pub(crate)) => {
        $crate::meta::Visibility::Crate
    };
    (pub) => {
        $crate::meta::Visibility::Public
    };
    (priv) => {
        $crate::meta::Visibility::Private
    };
    () => {
        $crate::meta::Visibility::Private
    };
}

#[macro_export]
macro_rules! field {
    ($vis:tt $t:ident($k:tt: $v:ty)) => {
        $crate::meta::Field {
            access: $crate::access!($vis),
            name: stringify!($k),
            attributes: &[],
            meta: <$v>::meta,
            get: |instance| Some(&instance.downcast_ref::<$t>()?.$k),
            get_mut: |instance| Some(&mut instance.downcast_mut::<$t>()?.$k),
            set: |instance, value| {
                Some(std::mem::swap(
                    &mut instance.downcast_mut::<$t>()?.$k,
                    value.downcast_mut()?,
                ))
            },
        }
    };
}

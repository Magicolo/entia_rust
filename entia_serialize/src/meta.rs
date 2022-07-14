use crate::recurse;
use std::{
    marker::PhantomData,
    mem::size_of,
    ops::{Bound, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
};

pub trait Meta<T> {
    fn meta() -> &'static T;
}

pub type Path = &'static [&'static str];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Access {
    Public,
    Private,
    Crate,
    Super,
    In(Path),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Primitive {
    Unit,
    Bool,
    Char,
    U8,
    U16,
    U32,
    U64,
    Usize,
    U128,
    I8,
    I16,
    I32,
    I64,
    Isize,
    I128,
    F32,
    F64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Structures {
    Unit,
    Tuple,
    Map,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Structure {
    pub modules: Path,
    pub name: &'static str,
    pub size: usize,
    pub kind: Structures,
    pub access: Access,
    pub attributes: &'static [Attribute],
    pub generics: &'static [Generic],
    pub fields: &'static [Field],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Enumeration {
    pub modules: &'static [&'static str],
    pub name: &'static str,
    pub size: usize,
    pub access: Access,
    pub attributes: &'static [Attribute],
    pub generics: &'static [Generic],
    pub variants: &'static [Variant],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Variants {
    Unit,
    Tuple,
    Map,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub name: &'static str,
    pub kind: Variants,
    pub attributes: &'static [Attribute],
    pub fields: &'static [Field],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: &'static str,
    pub access: Access,
    pub attributes: &'static [Attribute],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Generics {
    Lifetime,
    Type,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generic {
    pub name: &'static str,
    pub kind: Generics,
    pub attributes: &'static [Attribute],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    pub name: &'static str,
    pub content: &'static str,
}

impl<T> Meta<Enumeration> for Option<T> {
    fn meta() -> &'static Enumeration {
        &Enumeration {
            modules: &["core", "option"],
            name: "Option",
            size: size_of::<Self>(),
            access: Access::Public,
            attributes: &[],
            generics: &[Generic {
                name: "T",
                kind: Generics::Type,
                attributes: &[],
            }],
            variants: &[
                Variant {
                    name: "None",
                    kind: Variants::Unit,
                    attributes: &[],
                    fields: &[],
                },
                Variant {
                    name: "Some",
                    kind: Variants::Tuple,
                    attributes: &[],
                    fields: &[Field {
                        name: "0",
                        access: Access::Public,
                        attributes: &[],
                    }],
                },
            ],
        }
    }
}

impl<T, E> Meta<Enumeration> for Result<T, E> {
    fn meta() -> &'static Enumeration {
        &Enumeration {
            modules: &["core", "result"],
            name: "Result",
            size: size_of::<Self>(),
            access: Access::Public,
            attributes: &[],
            generics: &[
                Generic {
                    name: "T",
                    kind: Generics::Type,
                    attributes: &[],
                },
                Generic {
                    name: "E",
                    kind: Generics::Type,
                    attributes: &[],
                },
            ],
            variants: &[
                Variant {
                    name: "Ok",
                    kind: Variants::Tuple,
                    attributes: &[],
                    fields: &[Field {
                        name: "0",
                        access: Access::Public,
                        attributes: &[],
                    }],
                },
                Variant {
                    name: "Err",
                    kind: Variants::Tuple,
                    attributes: &[],
                    fields: &[Field {
                        name: "0",
                        access: Access::Public,
                        attributes: &[],
                    }],
                },
            ],
        }
    }
}

impl<T: ?Sized> Meta<Structure> for PhantomData<T> {
    fn meta() -> &'static Structure {
        &Structure {
            modules: &["core", "marker"],
            name: "PhantomData",
            size: size_of::<Self>(),
            kind: Structures::Unit,
            access: Access::Public,
            attributes: &[],
            generics: &[Generic {
                name: "T",
                kind: Generics::Type,
                attributes: &[],
            }],
            fields: &[],
        }
    }
}

impl<T> Meta<Structure> for Vec<T> {
    fn meta() -> &'static Structure {
        &Structure {
            modules: &["alloc", "vec"],
            name: "Vec",
            size: size_of::<Self>(),
            kind: Structures::Map,
            access: Access::Public,
            attributes: &[],
            generics: &[Generic {
                name: "T",
                kind: Generics::Type,
                attributes: &[],
            }],
            fields: &[
                Field {
                    name: "buf",
                    access: Access::Private,
                    attributes: &[],
                },
                Field {
                    name: "len",
                    access: Access::Private,
                    attributes: &[],
                },
            ],
        }
    }
}

impl<Idx> Meta<Structure> for Range<Idx> {
    fn meta() -> &'static Structure {
        &Structure {
            modules: &["core", "ops", "range"],
            name: "Range",
            size: size_of::<Self>(),
            kind: Structures::Map,
            access: Access::Public,
            attributes: &[],
            generics: &[Generic {
                name: "Idx",
                kind: Generics::Type,
                attributes: &[],
            }],
            fields: &[
                Field {
                    name: "start",
                    access: Access::Public,
                    attributes: &[],
                },
                Field {
                    name: "end",
                    access: Access::Public,
                    attributes: &[],
                },
            ],
        }
    }
}

impl<Idx> Meta<Structure> for RangeInclusive<Idx> {
    fn meta() -> &'static Structure {
        &Structure {
            modules: &["core", "ops", "range"],
            name: "RangeInclusive",
            size: size_of::<Self>(),
            kind: Structures::Map,
            access: Access::Public,
            attributes: &[],
            generics: &[Generic {
                name: "Idx",
                kind: Generics::Type,
                attributes: &[],
            }],
            fields: &[
                Field {
                    name: "start",
                    access: Access::Crate,
                    attributes: &[],
                },
                Field {
                    name: "end",
                    access: Access::Crate,
                    attributes: &[],
                },
                Field {
                    name: "exhausted",
                    access: Access::Crate,
                    attributes: &[],
                },
            ],
        }
    }
}

impl<Idx> Meta<Structure> for RangeTo<Idx> {
    fn meta() -> &'static Structure {
        &Structure {
            modules: &["core", "ops", "range"],
            name: "RangeTo",
            size: size_of::<Self>(),
            kind: Structures::Map,
            access: Access::Public,
            attributes: &[],
            generics: &[Generic {
                name: "Idx",
                kind: Generics::Type,
                attributes: &[],
            }],
            fields: &[Field {
                name: "end",
                access: Access::Public,
                attributes: &[],
            }],
        }
    }
}

impl<Idx> Meta<Structure> for RangeToInclusive<Idx> {
    fn meta() -> &'static Structure {
        &Structure {
            modules: &["core", "ops", "range"],
            name: "RangeToInclusive",
            size: size_of::<Self>(),
            kind: Structures::Map,
            access: Access::Public,
            attributes: &[],
            generics: &[Generic {
                name: "Idx",
                kind: Generics::Type,
                attributes: &[],
            }],
            fields: &[Field {
                name: "end",
                access: Access::Public,
                attributes: &[],
            }],
        }
    }
}

impl<Idx> Meta<Structure> for RangeFrom<Idx> {
    fn meta() -> &'static Structure {
        &Structure {
            modules: &["core", "ops", "range"],
            name: "RangeFrom",
            size: size_of::<Self>(),
            kind: Structures::Map,
            access: Access::Public,
            attributes: &[],
            generics: &[Generic {
                name: "Idx",
                kind: Generics::Type,
                attributes: &[],
            }],
            fields: &[Field {
                name: "start",
                access: Access::Public,
                attributes: &[],
            }],
        }
    }
}

impl Meta<Structure> for RangeFull {
    fn meta() -> &'static Structure {
        &Structure {
            modules: &["core", "ops", "range"],
            name: "RangeFull",
            size: size_of::<Self>(),
            kind: Structures::Unit,
            access: Access::Public,
            attributes: &[],
            generics: &[],
            fields: &[],
        }
    }
}

impl<T> Meta<Enumeration> for Bound<T> {
    fn meta() -> &'static Enumeration {
        &Enumeration {
            modules: &["core", "ops", "range"],
            name: "Bound",
            size: size_of::<Self>(),
            access: Access::Public,
            attributes: &[],
            generics: &[Generic {
                name: "T",
                kind: Generics::Type,
                attributes: &[],
            }],
            variants: &[
                Variant {
                    name: "Included",
                    kind: Variants::Tuple,
                    attributes: &[],
                    fields: &[Field {
                        name: "0",
                        access: Access::Public,
                        attributes: &[],
                    }],
                },
                Variant {
                    name: "Excluded",
                    kind: Variants::Tuple,
                    attributes: &[],
                    fields: &[Field {
                        name: "0",
                        access: Access::Public,
                        attributes: &[],
                    }],
                },
                Variant {
                    name: "Unbounded",
                    kind: Variants::Unit,
                    attributes: &[],
                    fields: &[],
                },
            ],
        }
    }
}

macro_rules! primitive {
    ($t:ty, $p:ident) => {
        impl Meta<Primitive> for $t {
            fn meta() -> &'static Primitive {
                &Primitive::$p
            }
        }
    };
    ($($t:ident, $p:ident),*) => { $(primitive!($t, $p);)* };
}

primitive!(
    bool, Bool, char, Char, u8, U8, u16, U16, u32, U32, u64, U64, usize, Usize, u128, U128, i8, I8,
    i16, I16, i32, I32, i64, I64, isize, Isize, i128, I128, f32, F32, f64, F64
);

macro_rules! tuple {
    () => {
        impl Meta<Primitive> for () {
            fn meta() -> &'static Primitive {
                &Primitive::Unit
            }
        }
    };
    ($($p:ident, $t:ident),*) => {
        impl<$($t,)*> Meta<Structure> for ($($t,)*) {
            fn meta() -> &'static Structure {
                &Structure {
                    modules: &[],
                    name: stringify!(($($t,)*)),
                    size: size_of::<Self>(),
                    kind: Structures::Tuple,
                    access: Access::Public,
                    attributes: &[],
                    generics: &[$(Generic { name: stringify!($t), kind: Generics::Type, attributes: &[] },)*],
                    fields: &[$(Field { name: stringify!($p), access: Access::Public, attributes: &[] },)*],
                }
            }
        }
    };
}

recurse!(tuple);

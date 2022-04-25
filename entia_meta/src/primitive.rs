use std::{
    any::{Any, TypeId},
    mem::size_of,
};

use crate::{
    function::Function,
    meta::{Access, Constant, Data, Index, Meta},
    value::Value,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Primitives {
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

pub struct Primitive {
    pub kind: Primitives,
    pub access: Access,
    pub name: &'static str,
    pub size: usize,
    pub identifier: fn() -> TypeId,
    pub from: fn(Value) -> Result<Value, Value>,
    pub default: fn() -> Value,
    pub constants: Index<Constant>,
    pub functions: Index<Function>,
}

impl Primitive {
    #[inline]
    pub fn from(&self, value: Value) -> Result<Value, Value> {
        (self.from)(value)
    }

    #[inline]
    pub fn default(&self) -> Value {
        (self.default)()
    }

    #[inline]
    pub fn identifier(&self) -> TypeId {
        (self.identifier)()
    }

    #[inline]
    pub fn clone(&self, value: &dyn Any) -> Option<Value> {
        Some(match self.kind {
            Primitives::Unit => Value::Unit(*value.downcast_ref()?),
            Primitives::Bool => Value::Bool(*value.downcast_ref()?),
            Primitives::Char => Value::Char(*value.downcast_ref()?),
            Primitives::U8 => Value::U8(*value.downcast_ref()?),
            Primitives::U16 => Value::U16(*value.downcast_ref()?),
            Primitives::U32 => Value::U32(*value.downcast_ref()?),
            Primitives::U64 => Value::U64(*value.downcast_ref()?),
            Primitives::Usize => Value::Usize(*value.downcast_ref()?),
            Primitives::U128 => Value::U128(*value.downcast_ref()?),
            Primitives::I8 => Value::I8(*value.downcast_ref()?),
            Primitives::I16 => Value::I16(*value.downcast_ref()?),
            Primitives::I32 => Value::I32(*value.downcast_ref()?),
            Primitives::I64 => Value::I64(*value.downcast_ref()?),
            Primitives::Isize => Value::Isize(*value.downcast_ref()?),
            Primitives::I128 => Value::I128(*value.downcast_ref()?),
            Primitives::F32 => Value::F32(*value.downcast_ref()?),
            Primitives::F64 => Value::F64(*value.downcast_ref()?),
        })
    }
}

macro_rules! from {
    ($v:ident($t:ident)) => {
        impl From<$t> for Value {
            #[inline]
            fn from(value: $t) -> Self {
                Self::$v(value)
            }
        }
    };
}

macro_rules! primitive {
    ($v:ident($t:ident)) => {
        from!($v($t));

        impl Meta<&'static Primitive> for $t {
            #[inline]
            fn meta() -> &'static Primitive {
                &Primitive {
                    access: Access::Public,
                    kind: Primitives::$v,
                    name: stringify!($t),
                    size: size_of::<$t>(),
                    identifier: TypeId::of::<$t>,
                    from: |value| match value {
                        Value::Unit(_) => Ok(Value::$v($t::default())),
                        Value::Bool(value) => Ok(Value::$v(value as u8 as $t)),
                        Value::Char(value) => Ok(Value::$v(value as u32 as $t)),
                        Value::U8(value) => Ok(Value::$v(value as $t)),
                        Value::U16(value) => Ok(Value::$v(value as $t)),
                        Value::U32(value) => Ok(Value::$v(value as $t)),
                        Value::U64(value) => Ok(Value::$v(value as $t)),
                        Value::Usize(value) => Ok(Value::$v(value as $t)),
                        Value::U128(value) => Ok(Value::$v(value as $t)),
                        Value::I8(value) => Ok(Value::$v(value as $t)),
                        Value::I16(value) => Ok(Value::$v(value as $t)),
                        Value::I32(value) => Ok(Value::$v(value as $t)),
                        Value::I64(value) => Ok(Value::$v(value as $t)),
                        Value::Isize(value) => Ok(Value::$v(value as $t)),
                        Value::I128(value) => Ok(Value::$v(value as $t)),
                        Value::F32(value) => Ok(Value::$v(value as $t)),
                        Value::F64(value) => Ok(Value::$v(value as $t)),
                        Value::Structure(..) | Value::Variant(..) => Err(value),
                    },
                    default: || Value::$v($t::default()),
                    constants: Index(
                        &[
                            Constant {
                                access: Access::Public,
                                name: "MAX",
                                meta: $t::meta,
                                value: &$t::MAX,
                                attributes: &[],
                            },
                            Constant {
                                access: Access::Public,
                                name: "MIN",
                                meta: $t::meta,
                                value: &$t::MIN,
                                attributes: &[],
                            },
                        ],
                        |name| match name { "MAX" => Some(0), "MIN" => Some(1), _ => None }),
                    functions: Index(&[], |_| None),
                }
            }
        }

        impl Meta<Data> for $t {
            #[inline]
            fn meta() -> Data {
                Data::Primitive(<$t as Meta<&'static Primitive>>::meta())
            }
        }
    };
    ($($v:ident($t:ident)),*) => { $(primitive!($v($t));)* }
}

primitive!(
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Usize(usize),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Isize(isize),
    I128(i128),
    F32(f32),
    F64(f64)
);

impl Meta for bool {
    #[inline]
    fn meta() -> Data {
        todo!()
    }
}
from!(Bool(bool));

impl Meta for char {
    #[inline]
    fn meta() -> Data {
        todo!()
    }
}
from!(Char(char));

impl Meta for () {
    #[inline]
    fn meta() -> Data {
        Data::Primitive(&Primitive {
            kind: Primitives::Unit,
            access: Access::Public,
            name: "()",
            size: size_of::<()>(),
            identifier: TypeId::of::<()>,
            from: |_| Ok(Value::Unit(())),
            default: || Value::Unit(()),
            constants: Index(&[], |_| None),
            functions: Index(
                &[
                    // Function {
                    //     access: Access::Public,
                    //     signature: Signature {
                    //         name: "clamp",
                    //         meta: <()>::meta,
                    //         generics: Index(&[], |_| None),
                    //         parameters: Index(
                    //             &[Parameter {
                    //                 kind: Parameters::Owned,
                    //                 name: "self",
                    //                 meta: <()>::meta,
                    //                 attributes: &[],
                    //             }],
                    //             |name| match name {
                    //                 "self" => Some(0),
                    //                 _ => None,
                    //             },
                    //         ),
                    //         attributes: &[],
                    //     },
                    //     invoke: |values| {
                    //         Some(Argument::Owned(Value::from(<()>::clamp(
                    //             values.next()?.owned().ok()?,
                    //             values.next()?.owned().ok()?,
                    //             values.next()?.owned().ok()?,
                    //         ))))
                    //     },
                    // },
                    // Function {
                    //     access: Access::Public,
                    //     signature: Signature {
                    //         name: "clone",
                    //         meta: <()>::meta,
                    //         generics: Index(&[], |_| None),
                    //         parameters: Index(
                    //             &[Parameter {
                    //                 kind: Parameters::Shared,
                    //                 name: "self",
                    //                 meta: <()>::meta,
                    //                 attributes: &[],
                    //             }],
                    //             |name| match name {
                    //                 "self" => Some(0),
                    //                 _ => None,
                    //             },
                    //         ),
                    //         attributes: &[],
                    //     },
                    //     invoke: |values| {
                    //         Some(Argument::Owned(Value::from(<()>::clone(
                    //             values.next()?.shared()?,
                    //         ))))
                    //     },
                    // },
                ],
                |name| match name {
                    "clamp" => Some(0),
                    "clone" => Some(1),
                    _ => None,
                },
            ),
        })
    }
}

impl From<()> for Value {
    #[inline]
    fn from(value: ()) -> Self {
        Value::Unit(value)
    }
}

use crate::meta::{Enumeration, Meta, Primitives, Structure, Type, Variant};
use std::{
    any::{Any, TypeId},
    mem::{forget, transmute_copy},
};

#[derive(Debug)]
pub enum Value {
    Unit(()),
    Bool(bool),
    Char(char),
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
    F64(f64),
    Structure(Box<dyn Any>, &'static Structure),
    Variant(Box<dyn Any>, &'static Enumeration, &'static Variant),
}

impl Value {
    pub fn from<T: Meta>(value: T) -> Self {
        match T::meta() {
            Type::Primitive(primitive) => match primitive.kind {
                Primitives::Unit => Self::Unit(unsafe { transmute_copy(&value) }),
                Primitives::Bool => Self::Bool(unsafe { transmute_copy(&value) }),
                Primitives::Char => Self::Char(unsafe { transmute_copy(&value) }),
                Primitives::U8 => Self::U8(unsafe { transmute_copy(&value) }),
                Primitives::U16 => Self::U16(unsafe { transmute_copy(&value) }),
                Primitives::U32 => Self::U32(unsafe { transmute_copy(&value) }),
                Primitives::U64 => Self::U64(unsafe { transmute_copy(&value) }),
                Primitives::Usize => Self::Usize(unsafe { transmute_copy(&value) }),
                Primitives::U128 => Self::U128(unsafe { transmute_copy(&value) }),
                Primitives::I8 => Self::I8(unsafe { transmute_copy(&value) }),
                Primitives::I16 => Self::I16(unsafe { transmute_copy(&value) }),
                Primitives::I32 => Self::I32(unsafe { transmute_copy(&value) }),
                Primitives::I64 => Self::I64(unsafe { transmute_copy(&value) }),
                Primitives::Isize => Self::Isize(unsafe { transmute_copy(&value) }),
                Primitives::I128 => Self::I128(unsafe { transmute_copy(&value) }),
                Primitives::F32 => Self::F32(unsafe { transmute_copy(&value) }),
                Primitives::F64 => Self::F64(unsafe { transmute_copy(&value) }),
            },
            Type::Structure(structure) => Self::Structure(Box::new(value), structure),
            Type::Enumeration(enumeration) => match enumeration.variant_of(&value) {
                Some((_, variant)) => Self::Variant(Box::new(value), enumeration, variant),
                None => unreachable!(),
            },
        }
    }

    pub fn into<T: Meta>(self) -> Option<T> {
        T::meta().from(self)?.downcast().ok()
    }

    pub fn meta(&self) -> Type {
        match self {
            Self::Unit(_) => <()>::meta(),
            Self::Bool(_) => bool::meta(),
            Self::Char(_) => char::meta(),
            Self::U8(_) => u8::meta(),
            Self::U16(_) => u16::meta(),
            Self::U32(_) => u32::meta(),
            Self::U64(_) => u64::meta(),
            Self::Usize(_) => usize::meta(),
            Self::U128(_) => u128::meta(),
            Self::I8(_) => i8::meta(),
            Self::I16(_) => i16::meta(),
            Self::I32(_) => i32::meta(),
            Self::I64(_) => i64::meta(),
            Self::Isize(_) => isize::meta(),
            Self::I128(_) => i128::meta(),
            Self::F32(_) => f32::meta(),
            Self::F64(_) => f64::meta(),
            Self::Structure(_, structure) => Type::Structure(structure),
            Self::Variant(_, enumeration, _) => Type::Enumeration(enumeration),
        }
    }

    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        #[inline]
        fn cast<S: Copy + 'static, T: 'static>(source: S) -> Result<T, S> {
            if source.type_id() == TypeId::of::<T>() {
                // SAFETY: Since 'T' == 'U', they will have the same size.
                let target = unsafe { transmute_copy(&source) };
                // Prevent double drop of 'source' since there now exists a reinterpret version of it that will take care of dropping.
                forget(source);
                Ok(target)
            } else {
                Err(source)
            }
        }

        match self {
            Self::Unit(value) => cast(value).map_err(Self::Unit),
            Self::Bool(value) => cast(value).map_err(Self::Bool),
            Self::Char(value) => cast(value).map_err(Self::Char),
            Self::U8(value) => cast(value).map_err(Self::U8),
            Self::U16(value) => cast(value).map_err(Self::U16),
            Self::U32(value) => cast(value).map_err(Self::U32),
            Self::U64(value) => cast(value).map_err(Self::U64),
            Self::Usize(value) => cast(value).map_err(Self::Usize),
            Self::U128(value) => cast(value).map_err(Self::U128),
            Self::I8(value) => cast(value).map_err(Self::I8),
            Self::I16(value) => cast(value).map_err(Self::I16),
            Self::I32(value) => cast(value).map_err(Self::I32),
            Self::I64(value) => cast(value).map_err(Self::I64),
            Self::Isize(value) => cast(value).map_err(Self::Isize),
            Self::I128(value) => cast(value).map_err(Self::I128),
            Self::F32(value) => cast(value).map_err(Self::F32),
            Self::F64(value) => cast(value).map_err(Self::F64),
            Self::Structure(value, structure) => match value.downcast() {
                Ok(value) => Ok(*value),
                Err(value) => Err(Self::Structure(value, structure)),
            },
            Self::Variant(value, enumeration, variant) => match value.downcast() {
                Ok(value) => Ok(*value),
                Err(value) => Err(Self::Variant(value, enumeration, variant)),
            },
        }
    }

    #[inline]
    pub fn upcast(self) -> Box<dyn Any> {
        match self {
            Self::Unit(value) => Box::new(value),
            Self::Bool(value) => Box::new(value),
            Self::Char(value) => Box::new(value),
            Self::U8(value) => Box::new(value),
            Self::U16(value) => Box::new(value),
            Self::U32(value) => Box::new(value),
            Self::U64(value) => Box::new(value),
            Self::Usize(value) => Box::new(value),
            Self::U128(value) => Box::new(value),
            Self::I8(value) => Box::new(value),
            Self::I16(value) => Box::new(value),
            Self::I32(value) => Box::new(value),
            Self::I64(value) => Box::new(value),
            Self::Isize(value) => Box::new(value),
            Self::I128(value) => Box::new(value),
            Self::F32(value) => Box::new(value),
            Self::F64(value) => Box::new(value),
            Self::Structure(value, _) => value,
            Self::Variant(value, _, _) => value,
        }
    }

    #[inline]
    pub fn clone(&self) -> Option<Self> {
        self.meta().clone(self)
    }
}

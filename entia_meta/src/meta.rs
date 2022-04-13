use crate::value::Value;
use std::{
    any::{type_name, Any, TypeId},
    fmt::{self, Debug, Formatter},
    mem::size_of,
    ops::{Deref, DerefMut},
};

pub trait Meta: 'static {
    fn meta() -> Type;
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Structures {
    Unit,
    Tuple,
    Map,
}

#[derive(Debug, Clone, Copy)]
pub enum Type {
    Primitive(&'static Primitive),
    Structure(&'static Structure),
    Enumeration(&'static Enumeration),
}

pub struct Primitive {
    pub kind: Primitives,
    pub module: &'static str,
    pub name: &'static str,
    pub path: fn() -> &'static str,
    pub size: usize,
    pub identifier: fn() -> TypeId,
    pub from: fn(Value) -> Result<Value, Value>,
    pub default: fn() -> Value,
}

pub struct Structure {
    pub kind: Structures,
    pub module: &'static str,
    pub name: &'static str,
    pub path: fn() -> &'static str,
    pub size: usize,
    pub file: &'static str,
    pub identifier: fn() -> TypeId,
    pub new: fn(&mut dyn Iterator<Item = Value>) -> Option<Box<dyn Any>>,
    pub drop: unsafe fn(&mut dyn Any) -> bool,
    pub index: fn(&str) -> Option<usize>,
    pub values: fn(Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>>,
    pub parameters: &'static [Type],
    pub attributes: &'static [Attribute],
    pub fields: &'static [Field],
}

pub struct Enumeration {
    pub module: &'static str,
    pub name: &'static str,
    pub path: fn() -> &'static str,
    pub size: usize,
    pub file: &'static str,
    pub identifier: fn() -> TypeId,
    pub drop: unsafe fn(&mut dyn Any) -> bool,
    pub index: fn(&str) -> Option<usize>,
    pub index_of: fn(&dyn Any) -> Option<usize>,
    pub attributes: &'static [Attribute],
    pub variants: &'static [Variant],
}

pub struct Field {
    pub name: &'static str,
    pub parent: fn() -> Type,
    pub meta: fn() -> Type,
    pub get: fn(instance: &dyn Any) -> Option<&dyn Any>,
    pub get_mut: fn(instance: &mut dyn Any) -> Option<&mut dyn Any>,
    pub set: fn(instance: &mut dyn Any, value: Value) -> bool,
    pub attributes: &'static [Attribute],
}

pub struct Variant {
    pub kind: Structures,
    pub name: &'static str,
    pub new: fn(&mut dyn Iterator<Item = Value>) -> Option<Box<dyn Any>>,
    pub index: fn(&str) -> Option<usize>,
    pub parent: fn() -> &'static Enumeration,
    pub values: fn(Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>>,
    pub attributes: &'static [Attribute],
    pub fields: &'static [Field],
}

#[derive(Debug)]
pub struct Attribute {
    pub name: &'static str,
    pub content: &'static str,
}

impl Type {
    #[inline]
    pub fn name(self) -> &'static str {
        match self {
            Type::Primitive(primitive) => primitive.name,
            Type::Structure(structure) => structure.name,
            Type::Enumeration(enumeration) => enumeration.name,
        }
    }

    #[inline]
    pub fn path(self) -> &'static str {
        match self {
            Type::Primitive(primitive) => primitive.path(),
            Type::Structure(structure) => structure.path(),
            Type::Enumeration(enumeration) => enumeration.path(),
        }
    }

    #[inline]
    pub fn identifier(self) -> TypeId {
        match self {
            Type::Primitive(primitive) => primitive.identifier(),
            Type::Structure(structure) => structure.identifier(),
            Type::Enumeration(enumeration) => enumeration.identifier(),
        }
    }

    #[inline]
    pub fn from(self, value: Value) -> Option<Value> {
        match self {
            Type::Primitive(primitive) => primitive.from(value).ok(),
            Type::Structure(structure) => structure.from(value),
            Type::Enumeration(enumeration) => enumeration.from(value),
        }
    }

    #[inline]
    pub fn clone(self, value: &dyn Any) -> Option<Value> {
        match self {
            Type::Primitive(primitive) => primitive.clone(value),
            Type::Structure(structure) => structure.clone(value),
            Type::Enumeration(enumeration) => enumeration.clone(value),
        }
    }
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
    pub fn path(&self) -> &'static str {
        (self.path)()
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

impl Structure {
    #[inline]
    pub fn field(&self, name: &str) -> Option<(usize, &Field)> {
        let index = (self.index)(name)?;
        Some((index, &self.fields[index]))
    }

    #[inline]
    pub fn new<I: IntoIterator<Item = Value>>(&self, parameters: I) -> Option<Box<dyn Any>> {
        (self.new)(&mut parameters.into_iter())
    }

    #[inline]
    pub fn identifier(&self) -> TypeId {
        (self.identifier)()
    }

    #[inline]
    pub fn path(&self) -> &'static str {
        (self.path)()
    }

    #[inline]
    pub fn values(&self, instance: Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>> {
        (self.values)(instance)
    }

    #[inline]
    pub fn from(&'static self, value: Value) -> Option<Value> {
        match (self.kind, value) {
            (Structures::Unit, _) => self.new([]).map(|value| Value::Structure(value, self)),
            (_, Value::Structure(value, source)) => {
                let mut values = source.values(value).ok()?.into_vec();
                for (i, field) in source.fields.iter().enumerate() {
                    values.swap(i, self.field(field.name)?.0);
                }
                Some(Value::Structure(self.new(values)?, self))
            }
            (_, Value::Variant(value, _, source)) => {
                let mut values = source.values(value).ok()?.into_vec();
                for (i, field) in source.fields.iter().enumerate() {
                    values.swap(i, self.field(field.name)?.0);
                }
                Some(Value::Structure(self.new(values)?, self))
            }
            _ => None,
        }
    }

    #[inline]
    pub fn clone(&'static self, value: &dyn Any) -> Option<Value> {
        if self.identifier() == value.type_id() {
            let values = self
                .fields
                .iter()
                .filter_map(|field| field.meta().clone(field.get(value)?));
            Some(Value::Structure(self.new(values)?, self))
        } else {
            None
        }
    }
}

impl Enumeration {
    #[inline]
    pub fn identifier(&self) -> TypeId {
        (self.identifier)()
    }

    #[inline]
    pub fn path(&self) -> &'static str {
        (self.path)()
    }

    #[inline]
    pub fn variant(&self, name: &str) -> Option<(usize, &Variant)> {
        let index = (self.index)(name)?;
        Some((index, &self.variants[index]))
    }

    #[inline]
    pub fn variant_of(&self, value: &dyn Any) -> Option<(usize, &Variant)> {
        let index = (self.index_of)(value)?;
        Some((index, &self.variants[index]))
    }

    #[inline]
    pub fn clone(&'static self, value: &dyn Any) -> Option<Value> {
        self.variant_of(value)?.1.clone(value)
    }

    #[inline]
    pub fn from(&'static self, value: Value) -> Option<Value> {
        match value {
            Value::Structure(value, source) => {
                let (_, variant) = self.variant(source.name)?;
                let mut values = source.values(value).ok()?.into_vec();
                for (i, field) in source.fields.iter().enumerate() {
                    values.swap(i, variant.field(field.name)?.0);
                }
                Some(Value::Variant(variant.new(values)?, self, variant))
            }
            Value::Variant(value, _, source) => {
                let (_, variant) = self.variant(source.name)?;
                let mut values = source.values(value).ok()?.into_vec();
                for (i, field) in source.fields.iter().enumerate() {
                    values.swap(i, variant.field(field.name)?.0);
                }
                Some(Value::Variant(variant.new(values)?, self, variant))
            }
            _ => None,
        }
    }
}

impl Variant {
    #[inline]
    pub fn new<I: IntoIterator<Item = Value>>(&self, parameters: I) -> Option<Box<dyn Any>> {
        (self.new)(&mut parameters.into_iter())
    }

    #[inline]
    pub fn field(&self, name: &str) -> Option<(usize, &Field)> {
        let index = (self.index)(name)?;
        Some((index, &self.fields[index]))
    }

    #[inline]
    pub fn values(&self, instance: Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>> {
        (self.values)(instance)
    }

    #[inline]
    pub fn parent(&self) -> &'static Enumeration {
        (self.parent)()
    }

    #[inline]
    pub fn clone(&'static self, value: &dyn Any) -> Option<Value> {
        let enumeration = self.parent();
        if enumeration.identifier() == value.type_id() {
            let values = self
                .fields
                .iter()
                .filter_map(|field| field.meta().clone(field.get(value)?));
            Some(Value::Variant(self.new(values)?, enumeration, self))
        } else {
            None
        }
    }
}

impl Field {
    #[inline]
    pub fn meta(&self) -> Type {
        (self.meta)()
    }

    #[inline]
    pub fn parent(&self) -> Type {
        (self.parent)()
    }

    #[inline]
    pub fn get<'a>(&self, instance: &'a dyn Any) -> Option<&'a dyn Any> {
        (self.get)(instance)
    }

    #[inline]
    pub fn get_mut<'a>(&self, instance: &'a mut dyn Any) -> Option<&'a mut dyn Any> {
        (self.get_mut)(instance)
    }

    #[inline]
    pub fn set<'a>(&self, instance: &'a mut dyn Any, value: Value) -> bool {
        (self.set)(instance, value)
    }
}

impl Deref for Value {
    type Target = dyn Any;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            Value::Unit(value) => value,
            Value::Bool(value) => value,
            Value::Char(value) => value,
            Value::U8(value) => value,
            Value::U16(value) => value,
            Value::U32(value) => value,
            Value::U64(value) => value,
            Value::Usize(value) => value,
            Value::U128(value) => value,
            Value::I8(value) => value,
            Value::I16(value) => value,
            Value::I32(value) => value,
            Value::I64(value) => value,
            Value::Isize(value) => value,
            Value::I128(value) => value,
            Value::F32(value) => value,
            Value::F64(value) => value,
            Value::Structure(value, _) => value,
            Value::Variant(value, _, _) => value,
        }
    }
}

impl DerefMut for Value {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Value::Unit(value) => value,
            Value::Bool(value) => value,
            Value::Char(value) => value,
            Value::U8(value) => value,
            Value::U16(value) => value,
            Value::U32(value) => value,
            Value::U64(value) => value,
            Value::Usize(value) => value,
            Value::U128(value) => value,
            Value::I8(value) => value,
            Value::I16(value) => value,
            Value::I32(value) => value,
            Value::I64(value) => value,
            Value::Isize(value) => value,
            Value::I128(value) => value,
            Value::F32(value) => value,
            Value::F64(value) => value,
            Value::Structure(value, _) => value,
            Value::Variant(value, _, _) => value,
        }
    }
}

impl Debug for Primitive {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Primitive")
            .field("kind", &self.kind)
            .field("module", &self.module)
            .field("name", &self.name)
            .field("path", &self.path())
            .field("size", &self.size)
            .field("identifier", &self.identifier())
            .finish()
    }
}

impl Debug for Structure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Structure")
            .field("kind", &self.kind)
            .field("module", &self.module)
            .field("name", &self.name)
            .field("path", &self.path())
            .field("size", &self.size)
            .field("file", &self.file)
            .field("identifier", &self.identifier())
            .field("attributes", &self.attributes)
            .field("fields", &self.fields)
            .finish()
    }
}

impl Debug for Enumeration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Enumeration")
            .field("module", &self.module)
            .field("name", &self.name)
            .field("path", &self.path())
            .field("size", &self.size)
            .field("file", &self.file)
            .field("identifier", &self.identifier())
            .field("attributes", &self.attributes)
            .field("variants", &self.variants)
            .finish()
    }
}

impl Debug for Field {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Field")
            .field("name", &self.name)
            .field("parent", &self.parent().path())
            .field("meta", &self.meta().path())
            .field("attributes", &self.attributes)
            .finish()
    }
}

impl Debug for Variant {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Variant")
            .field("kind", &self.kind)
            .field("name", &self.name)
            .field("parent", &self.parent().path())
            .field("values", &self.values)
            .field("attributes", &self.attributes)
            .field("fields", &self.fields)
            .finish()
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

        impl Meta for $t {
            #[inline]
            fn meta() -> Type {
                Type::Primitive(&Primitive {
                    kind: Primitives::$v,
                    module: stringify!(std::$t),
                    name: stringify!($t),
                    path: type_name::<$t>,
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
                })
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
    fn meta() -> Type {
        todo!()
    }
}
from!(Bool(bool));

impl Meta for char {
    #[inline]
    fn meta() -> Type {
        todo!()
    }
}
from!(Char(char));

impl Meta for () {
    #[inline]
    fn meta() -> Type {
        Type::Primitive(&Primitive {
            kind: Primitives::Unit,
            module: "",
            name: "()",
            path: type_name::<()>,
            size: size_of::<()>(),
            identifier: TypeId::of::<()>,
            from: |_| Ok(Value::Unit(())),
            default: || Value::Unit(()),
        })
    }
}
impl From<()> for Value {
    #[inline]
    fn from(value: ()) -> Self {
        Value::Unit(value)
    }
}

impl<T: Meta> Meta for Vec<T> {
    #[inline]
    fn meta() -> Type {
        Type::Structure(&Structure {
            kind: Structures::Map,
            module: "std::vec",
            name: "Vec",
            file: file!(),
            path: type_name::<Self>,
            size: size_of::<Self>(),
            identifier: TypeId::of::<Self>,
            parameters: &[T::meta()],
            attributes: &[],
            fields: &[],
            new: todo!(),
            drop: todo!(),
            index: todo!(),
            values: todo!(),
        })
    }
}

impl<T: Meta> Meta for Box<T> {
    #[inline]
    fn meta() -> Type {
        todo!()
    }
}

impl<T1: Meta> Meta for (T1,) {
    #[inline]
    fn meta() -> Type {
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
    ($t:ident [$($a:expr),*]) => {{
        static META: $crate::meta::Structure = $crate::meta::Structure {
            module: module_path!(),
            name: stringify!($t),
            path: std::any::type_name::<$t>,
            size: std::mem::size_of::<$t>(),
            file: file!(),
            identifier: std::any::TypeId::of::<$t>,
            kind: $crate::meta::Structures::Unit,
            attributes: &[$($a),*],
            fields: &[],
            parameters: &[],
            new: |_| Some(Box::new($t)),
            drop: |instance| instance.downcast_mut::<$t>().map(drop).is_some(),
            index: |_| None,
            values: |instance| instance.downcast::<$t>().map(|_| [].into()),
        };
        $crate::meta::Type::Structure(&META)
    }};
    ($t:ident { $($k:ident[$i:tt]: $v:ty),* } [$($a:expr),*]) => {{
        static META: $crate::meta::Structure = $crate::meta::Structure {
            module: module_path!(),
            name: stringify!($t),
            path: std::any::type_name::<$t>,
            size: std::mem::size_of::<$t>(),
            file: file!(),
            identifier: std::any::TypeId::of::<$t>,
            kind: $crate::meta::Structures::Map,
            attributes: &[$($a),*],
            fields: &[$($crate::field!($t($k: $v)),)*],
            parameters: &[],
            new: |values| Some(Box::new($t { $($k: values.next()?.into::<$v>()?,)* })),
            drop: |instance| instance.downcast_mut::<$t>().map(drop).is_some(),
            index: |name| match name { $(stringify!($k) | stringify!($i) => Some($i),)* _ => None, },
            values: |instance| instance.downcast::<$t>().map(|instance| [$($crate::value::Value::from(instance.$k),)*].into()),
        };
        $crate::meta::Type::Structure(&META)
    }};
    ($t:ident($($i:tt: $v:ty),*) [$($a:expr),*]) => {{
        static META: $crate::meta::Structure = $crate::meta::Structure {
            module: module_path!(),
            name: stringify!($t),
            path: std::any::type_name::<$t>,
            size: std::mem::size_of::<$t>(),
            file: file!(),
            identifier: std::any::TypeId::of::<$t>,
            kind: $crate::meta::Structures::Tuple,
            attributes: &[$($a),*],
            fields: &[$($crate::field!($t($i: $v)),)*],
            parameters: &[],
            new: |values| Some(Box::new($t($(values.next()?.into::<$v>()?,)*))),
            drop: |instance| instance.downcast_mut::<$t>().map(drop).is_some(),
            index: |name| match name { $(stringify!($i) => Some($i),)* _ => None, },
            values: |instance| instance.downcast::<$t>().map(|instance| [$($crate::value::Value::from(instance.$i),)*].into()),
        };
        $crate::meta::Type::Structure(&META)
    }};
}

#[macro_export]
macro_rules! field {
    ($t:ident($k:tt: $v:ty)) => {
        $crate::meta::Field {
            name: stringify!($k),
            attributes: &[],
            parent: <$t>::meta,
            meta: <$v>::meta,
            get: |instance| Some(&instance.downcast_ref::<$t>()?.$k),
            get_mut: |instance| Some(&mut instance.downcast_mut::<$t>()?.$k),
            set: |instance, value| match instance.downcast_mut::<$t>() {
                Some(instance) => match value.into::<$v>() {
                    Some(value) => {
                        instance.$k = value;
                        true
                    }
                    None => false,
                },
                None => false,
            },
        }
    };
}

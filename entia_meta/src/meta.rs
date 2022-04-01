use crate::instance::Instance;
use std::any::{type_name, Any, TypeId};

pub trait Meta {
    fn meta() -> &'static Type;
}

#[derive(Clone)]
pub struct Boba {
    a: usize,
    b: Vec<bool>,
    c: Fett,
}

#[derive(Clone)]
pub enum Fett {
    A(usize),
    B { b: Vec<bool> },
    C,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Structures {
    Unit,
    Tuple,
    Map,
}

pub enum Type {
    Primitive(Primitive),
    Structure(Structure),
    Enumeration(Enumeration),
}

pub struct Primitive {
    pub kind: Primitives,
    pub name: &'static str,
    pub full_name: fn() -> &'static str,
    pub identifier: fn() -> TypeId,
    pub from: fn(&dyn Instance) -> Option<Box<dyn Instance>>,
    pub default: fn() -> Box<dyn Instance>,
}

pub struct Structure {
    pub kind: Structures,
    pub name: &'static str,
    pub full_name: fn() -> &'static str,
    pub identifier: fn() -> TypeId,
    pub new: fn(&mut dyn Iterator<Item = Box<dyn Instance>>) -> Option<Box<dyn Instance>>,
    pub index: fn(&str) -> Option<usize>,
    pub attributes: &'static [Attribute],
    pub fields: &'static [Field],
}

pub struct Enumeration {
    pub name: &'static str,
    pub full_name: fn() -> &'static str,
    pub identifier: fn() -> TypeId,
    pub index: fn(&str) -> Option<usize>,
    pub attributes: &'static [Attribute],
    pub variants: &'static [Variant],
}

pub struct Field {
    pub name: &'static str,
    pub meta: fn() -> &'static Type,
    pub get: fn(instance: &dyn Instance) -> Option<&dyn Instance>,
    pub get_mut: fn(instance: &mut dyn Instance) -> Option<&mut dyn Instance>,
    pub set: fn(instance: &mut dyn Instance, value: Box<dyn Any>) -> Result<(), Box<dyn Any>>,
    pub attributes: &'static [Attribute],
}

pub struct Variant {
    pub name: &'static str,
    pub kind: Structures,
    pub new: fn(&mut dyn Iterator<Item = Box<dyn Instance>>) -> Option<Box<dyn Instance>>,
    pub index: fn(&str) -> Option<usize>,
    pub attributes: &'static [Attribute],
    pub fields: &'static [Field],
}

pub struct Attribute {
    pub name: &'static str,
    pub content: &'static str,
}

impl Primitive {
    #[inline]
    pub fn from(&self, instance: &dyn Instance) -> Option<Box<dyn Instance>> {
        (self.from)(instance)
    }

    #[inline]
    pub fn default(&self) -> Box<dyn Instance> {
        (self.default)()
    }
}

impl Structure {
    #[inline]
    pub fn field(&self, name: &str) -> Option<(usize, &Field)> {
        let index = (self.index)(name)?;
        Some((index, &self.fields[index]))
    }

    #[inline]
    pub fn new<I: IntoIterator<Item = Box<dyn Instance>>>(
        &self,
        parameters: I,
    ) -> Option<Box<dyn Instance>> {
        (self.new)(&mut parameters.into_iter())
    }
}

impl Enumeration {
    #[inline]
    pub fn variant(&self, name: &str) -> Option<(usize, &Variant)> {
        let index = (self.index)(name)?;
        Some((index, &self.variants[index]))
    }
}

impl Variant {
    #[inline]
    pub fn new<I: IntoIterator<Item = Box<dyn Instance>>>(
        &self,
        parameters: I,
    ) -> Option<Box<dyn Instance>> {
        (self.new)(&mut parameters.into_iter())
    }

    #[inline]
    pub fn field(&self, name: &str) -> Option<(usize, &Field)> {
        let index = (self.index)(name)?;
        Some((index, &self.fields[index]))
    }
}

impl Field {
    #[inline]
    pub fn value(&self) -> &'static Type {
        (self.meta)()
    }

    #[inline]
    pub fn get<'a>(&self, instance: &'a dyn Instance) -> Option<&'a dyn Instance> {
        (self.get)(instance)
    }

    #[inline]
    pub fn get_mut<'a>(&self, instance: &'a mut dyn Instance) -> Option<&'a mut dyn Instance> {
        (self.get_mut)(instance)
    }

    #[inline]
    pub fn set<'a>(
        &self,
        instance: &'a mut dyn Instance,
        value: Box<dyn Any>,
    ) -> Result<(), Box<dyn Any>> {
        (self.set)(instance, value)
    }
}

macro_rules! primitive {
    ($t:ident) => {
        impl Meta for $t {
            #[inline]
            fn meta() -> &'static Type {
                &Type::Primitive(Primitive {
                    name: stringify!($t),
                    full_name: type_name::<$t>,
                    identifier: TypeId::of::<$t>,
                    kind: Primitives::Usize,
                    from: |value| match value.get_meta() {
                        Type::Primitive(primitive) => Some(match primitive.kind {
                            Primitives::Unit => Box::new($t::default()),
                            Primitives::Bool => Box::new(*value.cast_ref::<bool>()? as u8 as $t),
                            Primitives::Char => Box::new(*value.cast_ref::<char>()? as u32 as $t),
                            Primitives::U8 => Box::new(*value.cast_ref::<u8>()? as $t),
                            Primitives::U16 => Box::new(*value.cast_ref::<u16>()? as $t),
                            Primitives::U32 => Box::new(*value.cast_ref::<u32>()? as $t),
                            Primitives::U64 => Box::new(*value.cast_ref::<u64>()? as $t),
                            Primitives::Usize => Box::new(*value.cast_ref::<usize>()? as $t),
                            Primitives::U128 => Box::new(*value.cast_ref::<u128>()? as $t),
                            Primitives::I8 => Box::new(*value.cast_ref::<i8>()? as $t),
                            Primitives::I16 => Box::new(*value.cast_ref::<i16>()? as $t),
                            Primitives::I32 => Box::new(*value.cast_ref::<i32>()? as $t),
                            Primitives::I64 => Box::new(*value.cast_ref::<i64>()? as $t),
                            Primitives::Isize => Box::new(*value.cast_ref::<isize>()? as $t),
                            Primitives::I128 => Box::new(*value.cast_ref::<i128>()? as $t),
                            Primitives::F32 => Box::new(*value.cast_ref::<f32>()? as $t),
                            Primitives::F64 => Box::new(*value.cast_ref::<f64>()? as $t),
                        }),
                        _ => None,
                    },
                    default: || Box::new(usize::default()),
                })
            }
        }
    };
    ($($t:ident),*) => { $(primitive!($t);)* }
}

primitive!(u8, u16, u32, u64, usize, u128, i8, i16, i32, i64, isize, i128, f32, f64);

impl Meta for bool {
    #[inline]
    fn meta() -> &'static Type {
        todo!()
    }
}

impl Meta for char {
    #[inline]
    fn meta() -> &'static Type {
        todo!()
    }
}

impl Meta for () {
    #[inline]
    fn meta() -> &'static Type {
        &Type::Primitive(Primitive {
            name: "()",
            full_name: type_name::<()>,
            identifier: TypeId::of::<()>,
            kind: Primitives::Unit,
            from: |_| None,
            default: || Box::new(()),
        })
    }
}

impl<T: Meta> Meta for Vec<T> {
    #[inline]
    fn meta() -> &'static Type {
        todo!()
    }
}

impl<T: Meta> Meta for Box<T> {
    #[inline]
    fn meta() -> &'static Type {
        todo!()
    }
}

impl<T1: Meta> Meta for (T1,) {
    #[inline]
    fn meta() -> &'static Type {
        todo!()
    }
}

impl Meta for Boba {
    #[inline]
    fn meta() -> &'static Type {
        &Type::Structure(Structure {
            name: "Boba",
            full_name: type_name::<Boba>,
            identifier: TypeId::of::<Boba>,
            kind: Structures::Map,
            attributes: &[Attribute {
                name: "derive",
                content: "Clone",
            }],
            fields: &[
                Field {
                    name: "a",
                    attributes: &[],
                    meta: usize::meta,
                    get: |instance| Some(&instance.cast_ref::<Boba>()?.a),
                    get_mut: |instance| Some(&mut instance.cast_mut::<Boba>()?.a),
                    set: |instance, value| match instance.cast_mut::<Boba>() {
                        Some(instance) => {
                            instance.a = *value.downcast()?;
                            Ok(())
                        }
                        None => Err(value),
                    },
                },
                Field {
                    name: "",
                    attributes: &[],
                    meta: Vec::<bool>::meta,
                    get: |instance| Some(&instance.cast_ref::<Boba>()?.b),
                    get_mut: |instance| Some(&mut instance.cast_mut::<Boba>()?.b),
                    set: |instance, value| match instance.cast_mut::<Boba>() {
                        Some(instance) => {
                            instance.b = *value.downcast()?;
                            Ok(())
                        }
                        None => Err(value),
                    },
                },
                Field {
                    name: "c",
                    attributes: &[],
                    meta: Fett::meta,
                    get: |instance| Some(&instance.cast_ref::<Boba>()?.c),
                    get_mut: |instance| Some(&mut instance.cast_mut::<Boba>()?.c),
                    set: |instance, value| match instance.cast_mut::<Boba>() {
                        Some(instance) => {
                            instance.c = *value.downcast()?;
                            Ok(())
                        }
                        None => Err(value),
                    },
                },
            ],
            new: |values| {
                Some(Box::new(Boba {
                    a: *values.next()?.cast().ok()?,
                    b: *values.next()?.cast().ok()?,
                    c: *values.next()?.cast().ok()?,
                }))
            },
            index: |name| match name {
                "a" | "0" => Some(0),
                "" | "1" => Some(1),
                "c" | "2" => Some(2),
                _ => None,
            },
        })
    }
}

impl Meta for Fett {
    fn meta() -> &'static Type {
        &Type::Enumeration(Enumeration {
            name: "Fett",
            full_name: type_name::<Fett>,
            identifier: TypeId::of::<Fett>,
            attributes: &[Attribute {
                name: "derive",
                content: "Clone",
            }],
            variants: &[
                Variant {
                    name: "A",
                    kind: Structures::Tuple,
                    attributes: &[],
                    fields: &[Field {
                        name: "0",
                        attributes: &[],
                        meta: usize::meta,
                        get: |instance| match instance.cast_ref() {
                            Some(Fett::A(a)) => Some(a),
                            _ => None,
                        },
                        get_mut: |instance| match instance.cast_mut() {
                            Some(Fett::A(a)) => Some(a),
                            _ => None,
                        },
                        set: |instance, value| match instance.cast_mut() {
                            Some(Fett::A(a)) => {
                                *a = *value.downcast()?;
                                Ok(())
                            }
                            _ => Err(value),
                        },
                    }],
                    index: |name| match name {
                        "0" => Some(0),
                        _ => None,
                    },
                    new: |values| Some(Box::new(Fett::A(*values.next()?.cast().ok()?))),
                },
                Variant {
                    name: "B",
                    kind: Structures::Map,
                    attributes: &[],
                    fields: &[],
                    new: |values| {
                        Some(Box::new(Fett::B {
                            b: *values.next()?.cast().ok()?,
                        }))
                    },
                    index: |name| match name {
                        "" | "0" => Some(0),
                        _ => None,
                    },
                },
                Variant {
                    name: "C",
                    kind: Structures::Unit,
                    attributes: &[],
                    fields: &[],
                    new: |_| Some(Box::new(Fett::C)),
                    index: |_| None,
                },
            ],
            index: |name| match name {
                "A" => Some(0),
                "B" => Some(1),
                "C" => Some(2),
                _ => None,
            },
        })
    }
}

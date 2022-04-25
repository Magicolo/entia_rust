use std::any::Any;

use crate::{Attribute, Primitive, Value};

pub struct Type {
    pub name: &'static str,
    pub default: Option<fn() -> Type>,
    pub attributes: &'static [Attribute],
}

pub struct Lifetime {
    pub name: &'static str,
    pub attributes: &'static [Attribute],
}

pub struct Constant {
    pub name: &'static str,
    pub default: Option<Value<Box<dyn Any + Sync + Send>>>,
    pub meta: fn() -> &'static Primitive,
    pub attributes: &'static [Attribute],
}

pub enum Generic {
    Type(Type),
    Lifetime(Lifetime),
    Constant(Constant),
}

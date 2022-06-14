use crate::meta::{Access, Attribute, Data};
use std::any::Any;

pub struct Field {
    pub access: Access,
    pub name: &'static str,
    pub meta: Option<fn() -> Data>,
    pub get: Option<fn(instance: &dyn Any) -> Option<&dyn Any>>,
    pub get_mut: Option<fn(instance: &mut dyn Any) -> Option<&mut dyn Any>>,
    pub set: Option<fn(instance: &mut dyn Any, value: &mut dyn Any) -> Option<()>>,
    pub attributes: &'static [Attribute],
}

impl Field {
    #[inline]
    pub fn meta(&self) -> Option<Data> {
        Some((self.meta?)())
    }

    #[inline]
    pub fn get<'a>(&self, instance: &'a dyn Any) -> Option<&'a dyn Any> {
        (self.get?)(instance)
    }

    #[inline]
    pub fn get_mut<'a>(&self, instance: &'a mut dyn Any) -> Option<&'a mut dyn Any> {
        (self.get_mut?)(instance)
    }

    #[inline]
    pub fn set(&self, instance: &mut dyn Any, value: &mut dyn Any) -> Option<()> {
        (self.set?)(instance, value)
    }
}

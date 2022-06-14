use crate::{
    field::Field,
    meta::{Attribute, Index},
    value::Value,
};
use std::any::Any;

pub struct Variant {
    pub name: &'static str,
    pub new: Option<fn(&mut dyn Iterator<Item = Value>) -> Option<Box<dyn Any>>>,
    pub values: Option<fn(Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>>>,
    pub attributes: &'static [Attribute],
    pub fields: Index<Field>,
}

impl Variant {
    #[inline]
    pub fn new<I: IntoIterator<Item = Value>>(&self, parameters: I) -> Option<Box<dyn Any>> {
        (self.new?)(&mut parameters.into_iter())
    }

    #[inline]
    pub fn values(&self, instance: Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>> {
        match self.values {
            Some(values) => values(instance),
            None => Err(instance),
        }
    }
}

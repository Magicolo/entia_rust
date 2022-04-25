use crate::{
    generic::Generic,
    meta::{Access, Attribute, Data, Index},
    value::Value,
};
use std::any::{Any, TypeId};

pub struct Structure {
    pub access: Access,
    pub name: &'static str,
    pub size: usize,
    pub identifier: fn() -> TypeId,
    pub new: fn(&mut dyn Iterator<Item = Value>) -> Option<Box<dyn Any>>,
    pub values: fn(Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>>,
    pub attributes: &'static [Attribute],
    pub generics: &'static [Generic],
    pub fields: Index<Field>,
}

pub struct Field {
    pub access: Access,
    pub name: &'static str,
    pub meta: fn() -> Data,
    pub get: fn(instance: &dyn Any) -> Option<&dyn Any>,
    pub get_mut: fn(instance: &mut dyn Any) -> Option<&mut dyn Any>,
    pub set: fn(instance: &mut dyn Any, value: &mut dyn Any) -> Option<()>,
    pub attributes: &'static [Attribute],
}

impl Structure {
    #[inline]
    pub fn new<I: IntoIterator<Item = Value>>(&self, parameters: I) -> Option<Box<dyn Any>> {
        (self.new)(&mut parameters.into_iter())
    }

    #[inline]
    pub fn identifier(&self) -> TypeId {
        (self.identifier)()
    }

    #[inline]
    pub fn values(&self, instance: Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>> {
        (self.values)(instance)
    }

    #[inline]
    pub fn from(&'static self, value: Value) -> Option<Value> {
        match value {
            _ if self.fields.len() == 0 => Some(Value::Structure(self.new([])?, self)),
            Value::Structure(value, source) => {
                let mut values = source.values(value).ok()?.into_vec();
                for (i, field) in source.fields.iter().enumerate() {
                    values.swap(i, self.fields.index(field.name)?);
                }
                Some(Value::Structure(self.new(values)?, self))
            }
            Value::Variant(value, _, source) => {
                let mut values = source.values(value).ok()?.into_vec();
                for (i, field) in source.fields.iter().enumerate() {
                    values.swap(i, self.fields.index(field.name)?);
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

impl Field {
    #[inline]
    pub fn meta(&self) -> Data {
        (self.meta)()
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
    pub fn set(&self, instance: &mut dyn Any, value: &mut dyn Any) -> Option<()> {
        (self.set)(instance, value)
    }
}

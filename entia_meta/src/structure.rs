use crate::{
    field::Field,
    generic::Generic,
    meta::{Access, Attribute, Index},
    value::Value,
};
use std::any::{Any, TypeId};

pub struct Structure {
    pub access: Access,
    pub name: &'static str,
    pub size: Option<usize>,
    pub identifier: Option<fn() -> TypeId>,
    pub new: Option<fn(&mut dyn Iterator<Item = Value>) -> Option<Box<dyn Any>>>,
    pub values: Option<fn(Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>>>,
    pub attributes: &'static [Attribute],
    pub generics: &'static [Generic],
    pub fields: Index<Field>,
}

impl Structure {
    #[inline]
    pub fn new<I: IntoIterator<Item = Value>>(&self, parameters: I) -> Option<Box<dyn Any>> {
        (self.new?)(&mut parameters.into_iter())
    }

    #[inline]
    pub fn identifier(&self) -> Option<TypeId> {
        Some((self.identifier?)())
    }

    #[inline]
    pub fn values(&self, instance: Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>> {
        match self.values {
            Some(values) => values(instance),
            None => Err(instance),
        }
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
        if self.identifier() == Some(value.type_id()) {
            let values = self
                .fields
                .iter()
                .filter_map(|field| field.meta()?.clone(field.get(value)?));
            Some(Value::Structure(self.new(values)?, self))
        } else {
            None
        }
    }
}

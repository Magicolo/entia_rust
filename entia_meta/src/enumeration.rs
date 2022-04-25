use std::any::{Any, TypeId};

use crate::{
    generic::Generic,
    meta::{Access, Attribute, Index},
    structure::Field,
    value::Value,
};

pub struct Enumeration {
    pub access: Access,
    pub name: &'static str,
    pub size: usize,
    pub identifier: fn() -> TypeId,
    pub generics: &'static [Generic],
    pub attributes: &'static [Attribute],
    pub variants: Index<Variant>,
    pub index: fn(&dyn Any) -> Option<usize>,
}

pub struct Variant {
    pub name: &'static str,
    pub new: fn(&mut dyn Iterator<Item = Value>) -> Option<Box<dyn Any>>,
    pub values: fn(Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>>,
    pub attributes: &'static [Attribute],
    pub fields: Index<Field>,
}

impl Enumeration {
    #[inline]
    pub fn identifier(&self) -> TypeId {
        (self.identifier)()
    }

    #[inline]
    pub fn variant_of(&self, value: &dyn Any) -> Option<(usize, &Variant)> {
        let index = (self.index)(value)?;
        Some((index, &self.variants[index]))
    }

    #[inline]
    pub fn clone(&'static self, value: &dyn Any) -> Option<Value> {
        if self.identifier() == value.type_id() {
            let (_, variant) = self.variant_of(value)?;
            let values = variant
                .fields
                .iter()
                .filter_map(|field| field.meta().clone(field.get(value)?));
            Some(Value::Variant(variant.new(values)?, self, variant))
        } else {
            None
        }
    }

    #[inline]
    pub fn from(&'static self, value: Value) -> Option<Value> {
        match value {
            Value::Structure(value, source) => {
                let variant = self.variants.get(source.name)?;
                let mut values = source.values(value).ok()?.into_vec();
                for (i, field) in source.fields.iter().enumerate() {
                    values.swap(i, variant.fields.index(field.name)?);
                }
                Some(Value::Variant(variant.new(values)?, self, variant))
            }
            Value::Variant(value, _, source) => {
                let variant = self.variants.get(source.name)?;
                let mut values = source.values(value).ok()?.into_vec();
                for (i, field) in source.fields.iter().enumerate() {
                    values.swap(i, variant.fields.index(field.name)?);
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
    pub fn values(&self, instance: Box<dyn Any>) -> Result<Box<[Value]>, Box<dyn Any>> {
        (self.values)(instance)
    }
}

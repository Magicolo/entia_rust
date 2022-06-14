use std::any::Any;

use crate::{
    generic::Generic,
    meta::{Access, Attribute, Data},
    value::Value,
};

#[repr(u8)]
pub enum Modifiers {
    Asyncronous = 1 << 0,
    Constant = 1 << 1,
    Unsafe = 1 << 2,
}

pub enum Argument<'a> {
    Owned(Value),
    Shared(&'a dyn Any),
    Exclusive(&'a mut dyn Any),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Borrow {
    Owned,
    Shared,
    Exclusive,
}

pub struct Parameter {
    pub borrow: Borrow,
    pub name: &'static str,
    pub meta: Option<fn() -> Data>,
    pub attributes: &'static [Attribute],
}

pub struct Function {
    pub access: Access,
    pub modifiers: u8,
    pub name: &'static str,
    pub meta: Option<fn() -> Data>,
    pub generics: &'static [Generic],
    pub parameters: &'static [Parameter],
    pub invoke: Option<Invoke>,
    pub attributes: &'static [Attribute],
}

pub enum Invoke {
    Safe(for<'a> fn(&mut dyn Iterator<Item = Argument<'a>>) -> Option<Argument<'a>>),
    Unsafe(for<'a> unsafe fn(&mut dyn Iterator<Item = Argument<'a>>) -> Option<Argument<'a>>),
}

impl<'a> Argument<'a> {
    #[inline]
    pub fn owned<T: 'static>(self) -> Result<T, Self> {
        match self {
            Argument::Owned(value) => match value.downcast() {
                Ok(value) => Ok(value),
                Err(value) => Err(Argument::Owned(value)),
            },
            _ => Err(self),
        }
    }

    #[inline]
    pub fn shared<T: 'static>(self) -> Option<&'a T> {
        match self {
            Argument::Shared(value) => value.downcast_ref(),
            Argument::Exclusive(value) => value.downcast_ref(),
            _ => None,
        }
    }

    #[inline]
    pub fn exclusive<T: 'static>(self) -> Option<&'a mut T> {
        match self {
            Argument::Exclusive(value) => value.downcast_mut(),
            _ => None,
        }
    }
}

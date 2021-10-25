use crate::world::World;
use std::any::{type_name, TypeId};

/// SAFETY: This trait is unsafe since a wrong implementation may lead to undefined behavior. Every
/// implementor must declare all necessary dependencies in order to properly inform a scheduler of what it
/// it allowed to do.
pub unsafe trait Depend {
    fn depend(&self, world: &World) -> Vec<Dependency>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    All,
    Inner,
    Outer,
}

#[derive(Debug, Clone)]
pub enum Dependency {
    Unknown,
    Read(TypeId, &'static str),
    Write(TypeId, &'static str),
    Defer(TypeId, &'static str),
    At(usize, Box<Dependency>),
    Ignore(Scope, Box<Dependency>),
}

impl Dependency {
    #[inline]
    pub fn read<T: 'static>() -> Self {
        Self::Read(TypeId::of::<T>(), type_name::<T>())
    }

    #[inline]
    pub fn write<T: 'static>() -> Self {
        Self::Write(TypeId::of::<T>(), type_name::<T>())
    }

    #[inline]
    pub fn defer<T: 'static>() -> Self {
        Self::Defer(TypeId::of::<T>(), type_name::<T>())
    }

    #[inline]
    pub fn at(self, index: usize) -> Self {
        Self::At(index, self.into())
    }

    #[inline]
    pub fn ignore(self, scope: Scope) -> Self {
        Self::Ignore(scope, self.into())
    }
}

unsafe impl<D: Depend> Depend for Option<D> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        match self {
            Some(depend) => depend.depend(world),
            None => Vec::new(),
        }
    }
}

macro_rules! depend {
    ($($p:ident, $t:ident),*) => {
        unsafe impl<'a, $($t: Depend,)*> Depend for ($($t,)*) {
            fn depend(&self, _world: &World) -> Vec<Dependency> {
                let ($($p,)*) = self;
                let mut _dependencies = Vec::new();
                $(_dependencies.append(&mut $p.depend(_world));)*
                _dependencies
            }
        }
    };
}

entia_macro::recurse_32!(depend);

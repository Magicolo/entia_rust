use entia_core::{utility::short_type_name, Bits};

use crate::{system::Error, world::World};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

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
    Read(TypeId, String),
    Write(TypeId, String),
    Defer(TypeId, String),
    At(usize, Box<Dependency>),
    Ignore(Scope, Box<Dependency>),
}

impl Dependency {
    #[inline]
    pub fn read<T: 'static>() -> Self {
        Self::Read(TypeId::of::<T>(), short_type_name::<T>())
    }

    #[inline]
    pub fn write<T: 'static>() -> Self {
        Self::Write(TypeId::of::<T>(), short_type_name::<T>())
    }

    #[inline]
    pub fn defer<T: 'static>() -> Self {
        Self::Defer(TypeId::of::<T>(), short_type_name::<T>())
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

use Scope::*;

#[derive(Debug)]
pub enum Has {
    All,
    None,
    Indices(Bits),
}

#[derive(Debug, Default)]
pub struct Conflict {
    unknown: bool,
    reads: HashMap<TypeId, Has>,
    writes: HashMap<TypeId, Has>,
    defers: HashMap<TypeId, Has>,
}

impl Has {
    pub fn add(&mut self, index: usize) -> bool {
        match self {
            Self::All => false,
            Self::None => {
                *self = Has::Indices(Bits::new());
                self.add(index)
            }
            Self::Indices(bits) => {
                if bits.has(index) {
                    false
                } else {
                    bits.set(index, true);
                    true
                }
            }
        }
    }

    pub fn has(&self, index: usize) -> bool {
        match self {
            Self::All => true,
            Self::None => false,
            Self::Indices(bits) => bits.has(index),
        }
    }
}

impl Default for Has {
    fn default() -> Self {
        Self::None
    }
}

impl Conflict {
    pub fn detect(&mut self, scope: Scope, dependencies: &Vec<Dependency>) -> Result<(), Error> {
        let mut errors = Vec::new();
        if scope == Outer && self.unknown {
            errors.push(Error::UnknownConflict);
        }

        for dependency in dependencies {
            match self.conflict(scope, None, dependency.clone()) {
                Ok(_) => {}
                Err(error) => errors.push(error),
            }
        }

        let mut set = HashSet::new();
        errors.retain(move |error| set.insert(error.clone()));
        Error::All(errors).flatten(true).map(Err).unwrap_or(Ok(()))
    }

    pub fn clear(&mut self) {
        self.unknown = false;
        self.reads.clear();
        self.writes.clear();
        self.defers.clear();
    }

    fn conflict(
        &mut self,
        scope: Scope,
        index: Option<usize>,
        dependency: Dependency,
    ) -> Result<(), Error> {
        match (index, dependency) {
            (_, Dependency::Unknown) => {
                self.unknown = true;
                if scope == Outer {
                    Err(Error::UnknownConflict)
                } else {
                    Ok(())
                }
            }
            (_, Dependency::At(index, dependency)) => {
                self.conflict(scope, Some(index), *dependency)
            }
            (index, Dependency::Ignore(inner, dependency)) => {
                if scope == inner || inner == All {
                    self.conflict(scope, index, *dependency)
                } else {
                    Ok(())
                }
            }
            (Some(index), Dependency::Read(identifier, name)) => {
                if has(&self.writes, identifier, index) {
                    Err(Error::ReadWriteConflict(name, Some(index)))
                } else if scope == Outer && has(&self.defers, identifier, index) {
                    Err(Error::ReadDeferConflict(name, Some(index)))
                } else {
                    add(&mut self.reads, identifier, index);
                    Ok(())
                }
            }

            (Some(index), Dependency::Write(identifier, name)) => {
                if has(&self.reads, identifier, index) {
                    Err(Error::ReadWriteConflict(name, Some(index)))
                } else if has(&self.writes, identifier, index) {
                    Err(Error::WriteWriteConflict(name, Some(index)))
                } else if scope == Outer && has(&self.defers, identifier, index) {
                    Err(Error::WriteDeferConflict(name, Some(index)))
                } else {
                    add(&mut self.writes, identifier, index);
                    Ok(())
                }
            }
            (Some(index), Dependency::Defer(identifier, _)) => {
                add(&mut self.defers, identifier, index);
                Ok(())
            }
            (None, Dependency::Read(identifier, name)) => {
                if has_any(&self.writes, identifier) {
                    Err(Error::ReadWriteConflict(name, None))
                } else if scope == Outer && has_any(&self.defers, identifier) {
                    Err(Error::ReadDeferConflict(name, None))
                } else {
                    add_all(&mut self.reads, identifier);
                    Ok(())
                }
            }
            (None, Dependency::Write(identifier, name)) => {
                if has_any(&self.reads, identifier) {
                    Err(Error::ReadWriteConflict(name, None))
                } else if has_any(&self.writes, identifier) {
                    Err(Error::WriteWriteConflict(name, None))
                } else if scope == Outer && has_any(&self.defers, identifier) {
                    Err(Error::WriteDeferConflict(name, None))
                } else {
                    add_all(&mut self.writes, identifier);
                    Ok(())
                }
            }
            (None, Dependency::Defer(identifier, _)) => {
                add_all(&mut self.defers, identifier);
                Ok(())
            }
        }
    }
}

fn add(map: &mut HashMap<TypeId, Has>, identifier: TypeId, index: usize) -> bool {
    map.entry(identifier).or_default().add(index)
}

fn add_all(map: &mut HashMap<TypeId, Has>, identifier: TypeId) {
    *map.entry(identifier).or_default() = Has::All;
}

fn has(map: &HashMap<TypeId, Has>, identifier: TypeId, index: usize) -> bool {
    map.get(&identifier)
        .map(|has| has.has(index))
        .unwrap_or(false)
}

fn has_any(map: &HashMap<TypeId, Has>, identifier: TypeId) -> bool {
    has(map, identifier, usize::MAX)
}

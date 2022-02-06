use crate::{error, recurse, world::World};
use entia_core::utility::short_type_name;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

/*
TODO: There should be a way to opt out of dependency checking.
    - Every system or system group should be allowed to change its dependency behavior.
    - Dependency rules fall into 3 categories: Inner, Outer, Coherence.
    - Inner rules are local to a system and ensure that no undefined behavior occurs. These are always active.
    - Outer rules ensure that there is no undefined behavior between parallel systems. Since there are scenarios where the
    user can ensure safety, these rules can be disabled although not recommended.
    - Coherence rules maintain all 'happens before' relationships. Since a user may know better than these heuristics, these rules
    can be disabled without any risk of undefined behavior.
        Example of coherence:
        - Let system A have a 'Create<Entity>' and let system B have a 'Query<Entity>'
        - While they could run in parallel since 'Create' has been made safe, the query will never see newly created entities
        because they are only 'commited' to the segment at 'resolve' time.
        - The coherence rules would impose a synchronization point between the 2 systems such that system B always observes
        system A's created entities. Such ordering would respect the declaration order of the systems.
        - Note that if system B and system A swapped their declaration order, there would not be any coherence issue; the query
        would simply never observe the created entities.
*/

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

#[derive(Debug)]
pub enum Has {
    All,
    None,
    Indices(HashSet<usize>),
}

#[derive(Debug, Default)]
pub struct Conflict {
    unknown: bool,
    reads: HashMap<TypeId, Has>,
    writes: HashMap<TypeId, Has>,
    defers: HashMap<TypeId, Has>,
}

#[derive(Debug)]
pub enum Error {
    UnknownConflict(Scope),
    ReadWriteConflict(Scope, String, Option<usize>),
    WriteWriteConflict(Scope, String, Option<usize>),
    ReadDeferConflict(Scope, String, Option<usize>),
    WriteDeferConflict(Scope, String, Option<usize>),
}

error::error!(Error, error::Error::Depend);

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

unsafe impl<T> Depend for PhantomData<T> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        ().depend(world)
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

recurse!(depend);

impl Has {
    pub fn add(&mut self, index: usize) -> bool {
        match self {
            Self::All => false,
            Self::None => {
                *self = Has::Indices(HashSet::new());
                self.add(index)
            }
            Self::Indices(indices) => indices.insert(index),
        }
    }

    pub fn has(&self, index: usize) -> bool {
        match self {
            Self::All => true,
            Self::None => false,
            Self::Indices(indices) => indices.contains(&index),
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
        if scope == Scope::Outer && self.unknown {
            return Err(Error::UnknownConflict(scope));
        }

        for dependency in dependencies {
            self.conflict(scope, None, dependency.clone())?;
        }

        Ok(())
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
        use self::{Dependency::*, Error::*, Scope::*};

        match (index, dependency) {
            (_, Unknown) => {
                self.unknown = true;
                if scope == Outer {
                    Err(UnknownConflict(scope))
                } else {
                    Ok(())
                }
            }
            (_, At(index, dependency)) => self.conflict(scope, Some(index), *dependency),
            (index, Ignore(inner, dependency)) => {
                if scope == inner || inner == All {
                    self.conflict(scope, index, *dependency)
                } else {
                    Ok(())
                }
            }
            (Some(index), Read(identifier, name)) => {
                if has(&self.writes, identifier, index) {
                    Err(ReadWriteConflict(scope, name, Some(index)))
                } else if scope == Outer && has(&self.defers, identifier, index) {
                    Err(ReadDeferConflict(scope, name, Some(index)))
                } else {
                    add(&mut self.reads, identifier, index);
                    Ok(())
                }
            }
            (Some(index), Write(identifier, name)) => {
                if has(&self.reads, identifier, index) {
                    Err(ReadWriteConflict(scope, name, Some(index)))
                } else if has(&self.writes, identifier, index) {
                    Err(WriteWriteConflict(scope, name, Some(index)))
                } else if scope == Outer && has(&self.defers, identifier, index) {
                    Err(WriteDeferConflict(scope, name, Some(index)))
                } else {
                    add(&mut self.writes, identifier, index);
                    Ok(())
                }
            }
            (Some(index), Defer(identifier, _)) => {
                add(&mut self.defers, identifier, index);
                Ok(())
            }
            (None, Read(identifier, name)) => {
                if has_any(&self.writes, identifier) {
                    Err(ReadWriteConflict(scope, name, None))
                } else if scope == Outer && has_any(&self.defers, identifier) {
                    Err(ReadDeferConflict(scope, name, None))
                } else {
                    add_all(&mut self.reads, identifier);
                    Ok(())
                }
            }
            (None, Write(identifier, name)) => {
                if has_any(&self.reads, identifier) {
                    Err(ReadWriteConflict(scope, name, None))
                } else if has_any(&self.writes, identifier) {
                    Err(WriteWriteConflict(scope, name, None))
                } else if scope == Outer && has_any(&self.defers, identifier) {
                    Err(WriteDeferConflict(scope, name, None))
                } else {
                    add_all(&mut self.writes, identifier);
                    Ok(())
                }
            }
            (None, Defer(identifier, _)) => {
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
    map.get(&identifier).map_or(false, |has| has.has(index))
}

fn has_any(map: &HashMap<TypeId, Has>, identifier: TypeId) -> bool {
    has(map, identifier, usize::MAX)
}

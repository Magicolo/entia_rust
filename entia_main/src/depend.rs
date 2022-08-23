use crate::error;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scope {
    All,
    Inner,
    Outer,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dependency {
    Unknown,
    Read(Identifier),
    Write(Identifier),
    Defer(Identifier),
    At(usize, Box<Dependency>),
    Ignore(Scope, Box<Dependency>),
}

#[derive(Debug, Default)]
pub struct Conflict {
    unknown: bool,
    reads: HashMap<Identifier, Has>,
    writes: HashMap<Identifier, Has>,
    defers: HashMap<Identifier, Has>,
}

#[derive(Debug, Clone)]
pub enum Error {
    UnknownConflict(Scope),
    ReadWriteConflict(Scope, Option<usize>),
    WriteWriteConflict(Scope, Option<usize>),
    ReadDeferConflict(Scope, Option<usize>),
    WriteDeferConflict(Scope, Option<usize>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Identifier {
    At(usize),
    Type(TypeId),
}

#[derive(Debug)]
enum Has {
    All,
    None,
    Indices(HashSet<usize>),
}

error::error!(Error, error::Error::Depend);

impl Dependency {
    #[inline]
    pub fn read_at(identifier: usize) -> Self {
        Self::Read(Identifier::At(identifier))
    }

    #[inline]
    pub fn read<T: 'static>() -> Self {
        Self::Read(Identifier::Type(TypeId::of::<T>()))
    }

    #[inline]
    pub fn write_at(identifier: usize) -> Self {
        Self::Write(Identifier::At(identifier))
    }

    #[inline]
    pub fn write<T: 'static>() -> Self {
        Self::Write(Identifier::Type(TypeId::of::<T>()))
    }
}

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
    pub fn detect(
        &mut self,
        scope: Scope,
        dependencies: &[Dependency],
        fill: bool,
    ) -> Result<(), Error> {
        if scope == Scope::Outer && self.unknown {
            return Err(Error::UnknownConflict(scope));
        }

        for dependency in dependencies {
            self.conflict(scope, None, dependency, fill)?;
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
        at: Option<usize>,
        dependency: &Dependency,
        fill: bool,
    ) -> Result<(), Error> {
        use self::{Dependency::*, Error::*, Scope::*};

        match (at, dependency) {
            (_, Unknown) => {
                self.unknown = true;
                if scope == Outer {
                    Err(UnknownConflict(scope))
                } else {
                    Ok(())
                }
            }
            (_, At(index, dependency)) => self.conflict(scope, Some(*index), dependency, fill),
            (at, Ignore(inner, dependency)) => {
                if scope == *inner || *inner == All {
                    self.conflict(scope, at, dependency, fill)
                } else {
                    Ok(())
                }
            }
            (Some(at), Read(identifier)) => {
                if has(&self.writes, *identifier, at) {
                    Err(ReadWriteConflict(scope, Some(at)))
                } else if scope == Outer && has(&self.defers, *identifier, at) {
                    Err(ReadDeferConflict(scope, Some(at)))
                } else if fill {
                    add(&mut self.reads, *identifier, at);
                    Ok(())
                } else {
                    Ok(())
                }
            }
            (Some(at), Write(identifier)) => {
                if has(&self.reads, *identifier, at) {
                    Err(ReadWriteConflict(scope, Some(at)))
                } else if has(&self.writes, *identifier, at) {
                    Err(WriteWriteConflict(scope, Some(at)))
                } else if scope == Outer && has(&self.defers, *identifier, at) {
                    Err(WriteDeferConflict(scope, Some(at)))
                } else if fill {
                    add(&mut self.writes, *identifier, at);
                    Ok(())
                } else {
                    Ok(())
                }
            }
            (Some(at), Defer(identifier)) => {
                if fill {
                    add(&mut self.defers, *identifier, at);
                }
                Ok(())
            }
            (None, Read(identifier)) => {
                if has_any(&self.writes, *identifier) {
                    Err(ReadWriteConflict(scope, None))
                } else if scope == Outer && has_any(&self.defers, *identifier) {
                    Err(ReadDeferConflict(scope, None))
                } else if fill {
                    add_all(&mut self.reads, *identifier);
                    Ok(())
                } else {
                    Ok(())
                }
            }
            (None, Write(identifier)) => {
                if has_any(&self.reads, *identifier) {
                    Err(ReadWriteConflict(scope, None))
                } else if has_any(&self.writes, *identifier) {
                    Err(WriteWriteConflict(scope, None))
                } else if scope == Outer && has_any(&self.defers, *identifier) {
                    Err(WriteDeferConflict(scope, None))
                } else if fill {
                    add_all(&mut self.writes, *identifier);
                    Ok(())
                } else {
                    Ok(())
                }
            }
            (None, Defer(identifier)) => {
                if fill {
                    add_all(&mut self.defers, *identifier);
                }
                Ok(())
            }
        }
    }
}

fn add(map: &mut HashMap<Identifier, Has>, identifier: Identifier, index: usize) -> bool {
    map.entry(identifier).or_default().add(index)
}

fn add_all(map: &mut HashMap<Identifier, Has>, identifier: Identifier) {
    *map.entry(identifier).or_default() = Has::All;
}

fn has(map: &HashMap<Identifier, Has>, identifier: Identifier, index: usize) -> bool {
    map.get(&identifier).map_or(false, |has| has.has(index))
}

fn has_any(map: &HashMap<Identifier, Has>, identifier: Identifier) -> bool {
    has(map, identifier, usize::MAX)
}

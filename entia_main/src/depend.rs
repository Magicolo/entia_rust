use crate::error;
use std::{any::TypeId, collections::HashSet};

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
}

#[derive(Debug, Default)]
pub struct Conflict {
    unknown: bool,
    reads: HashSet<Identifier>,
    writes: HashSet<Identifier>,
}

#[derive(Debug, Clone)]
pub enum Error {
    UnknownConflict(Scope),
    ReadWriteConflict(Scope),
    WriteWriteConflict(Scope),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Identifier {
    At(usize),
    Type(TypeId),
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
            self.conflict(scope, dependency, fill)?;
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.unknown = false;
        self.reads.clear();
        self.writes.clear();
    }

    fn conflict(&mut self, scope: Scope, dependency: &Dependency, fill: bool) -> Result<(), Error> {
        use self::{Dependency::*, Error::*, Scope::*};

        match dependency {
            Unknown => {
                if fill {
                    self.unknown = true;
                }
                if scope == Outer {
                    Err(UnknownConflict(scope))
                } else {
                    Ok(())
                }
            }
            Read(identifier) if self.writes.contains(identifier) => Err(ReadWriteConflict(scope)),
            Read(identifier) if fill && self.reads.insert(*identifier) => Ok(()),
            Read(_) => Ok(()),
            Write(identifier) if self.reads.contains(identifier) => Err(ReadWriteConflict(scope)),
            Write(identifier) if fill && self.writes.insert(*identifier) => Ok(()),
            Write(identifier) if self.writes.contains(identifier) => Err(WriteWriteConflict(scope)),
            Write(_) => Ok(()),
        }
    }
}

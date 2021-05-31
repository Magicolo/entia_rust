use std::any::TypeId;

use crate::world::World;

pub trait Depend {
    fn depend(&self, world: &World) -> Vec<Dependency>;
}

#[derive(Debug, Clone)]
pub enum Dependency {
    Unknown,
    Read(usize, TypeId),
    Write(usize, TypeId),
    Defer(usize, TypeId),
}

impl<D: Depend> Depend for Option<D> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        match self {
            Some(depend) => depend.depend(world),
            None => Vec::new(),
        }
    }
}

macro_rules! depend {
    ($($p:ident, $t:ident),*) => {
        impl<'a, $($t: Depend,)*> Depend for ($($t,)*) {
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

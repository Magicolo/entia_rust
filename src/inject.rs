use crate::system::*;
use crate::world::*;

pub trait Inject {
    type State: for<'a> Get<'a> + 'static;
    fn initialize(world: &mut World) -> Option<Self::State>;
    fn update(_: &mut Self::State, _: &mut World) {}
    fn resolve(_: &mut Self::State, _: &mut World) {}
    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Unknown]
    }
}

pub trait Get<'a> {
    type Item;
    fn get(&'a mut self, world: &World) -> Self::Item;
}

macro_rules! inject {
    ($($p:ident, $t:ident),*) => {
        impl<'a, $($t: Inject,)*> Inject for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(_world: &mut World) -> Option<Self::State> {
                Some(($($t::initialize(_world)?,)*))
            }

            fn update(($($p,)*): &mut Self::State, _world: &mut World) {
                $($t::update($p, _world);)*
            }

            fn resolve(($($p,)*): &mut Self::State, _world: &mut World) {
                $($t::resolve($p, _world);)*
            }

            fn depend(($($p,)*): &Self::State, _world: &World) -> Vec<Dependency> {
                let mut _dependencies = Vec::new();
                $(_dependencies.append(&mut $t::depend($p, _world));)*
                _dependencies
            }
        }

        impl<'a, $($t: Get<'a>,)*> Get<'a> for ($($t,)*) {
            type Item = ($($t::Item,)*);

            #[inline]
            fn get(&'a mut self, _world: &World) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.get(_world),)*)
            }
        }
    };
}

crate::recurse!(
    inject, inject0, I0, inject1, I1, inject2, I2, inject3, I3, inject4, I4, inject5, I5, inject6,
    I6, inject7, I7, inject8, I8, inject9, I9, inject10, I10, inject11, I11, inject12, I12,
    inject13, I13, inject14, I14, inject15, I15
);

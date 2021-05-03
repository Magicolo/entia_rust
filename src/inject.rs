use crate::system::*;
use crate::world::*;

pub trait Inject {
    type State;
    fn initialize(world: &mut World) -> Option<Self::State>;
    fn update(state: &mut Self::State, world: &mut World) -> Vec<Dependency>;
    fn resolve(state: &Self::State, world: &mut World);
    fn inject(state: &Self::State, world: &World) -> Self;
}

impl Inject for () {
    type State = ();

    fn initialize(_: &mut World) -> Option<Self::State> {
        Some(())
    }

    fn update(_: &mut Self::State, _: &mut World) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State, _: &mut World) {}

    #[inline]
    fn inject(_: &Self::State, _: &World) -> Self {
        ()
    }
}

macro_rules! inject {
    ($($p:ident, $t:ident),+) => {
        impl<$($t: Inject),+> Inject for ($($t),+,) {
            type State = ($($t::State),+,);

            fn initialize(world: &mut World) -> Option<Self::State> {
                Some(($($t::initialize(world)?),+,))
            }

            fn update(($($p),+,): &mut Self::State, world: &mut World) -> Vec<Dependency> {
                let mut dependencies = Vec::new();
                $(dependencies.append(&mut $t::update($p, world)));+;
                dependencies
            }

            fn resolve(($($p),+,): &Self::State, world: &mut World) {
                $($t::resolve($p, world));+;
            }

            #[inline]
            fn inject(($($p),+,): &Self::State, world: &World) -> Self {
                ($($t::inject($p, world)),+,)
            }
        }
    };
}

crate::recurse!(
    inject, inject1, I1, inject2, I2, inject3, I3, inject4, I4, inject5, I5, inject6, I6, inject7,
    I7, inject8, I8, inject9, I9, inject10, I10, inject11, I11, inject12, I12
);

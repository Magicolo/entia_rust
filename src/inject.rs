use crate::system::*;
use crate::world::*;
use std::marker::PhantomData;

pub trait Inject<'a>: 'a {
    type State: 'a;
    // type State: for<'a> State<'a>;
    fn initialize(world: &'i World) -> Option<Self::State>;
    fn inject(state: &Self::State) -> Self;
    fn update(_: &mut Self::State) {}
    fn resolve(_: &mut Self::State) {}
    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        vec![Dependency::Unknown]
    }
}

pub trait State<'a> {
    type Item;
    fn get(&'a self) -> Self::Item;
    // fn update(&mut self) {}
    // fn resolve(&mut self) {}
    // fn dependencies(&self) -> Vec<Dependency> {
    //     vec![Dependency::Unknown]
    // }
}

impl<'a, T: 'a> Inject<'a> for PhantomData<T> {
    type State = ();

    fn initialize(_: &World) -> Option<Self::State> {
        Some(())
    }

    fn inject(_: &Self::State) -> Self {
        PhantomData
    }

    fn dependencies(_: &Self::State) -> Vec<Dependency> {
        Vec::new()
    }
}

macro_rules! inject {
    ($($p:ident, $t:ident),*) => {
        impl<'a, $($t: Inject<'a>,)*> Inject<'a> for ($($t,)*) {
            type State = ($($t::State,)*);

            fn initialize(_world: &'a World) -> Option<Self::State> {
                Some(($($t::initialize(_world)?,)*))
            }

            fn inject(($($p,)*): &Self::State) -> Self {
                ($($t::inject($p),)*)
            }

            fn update(($($p,)*): &mut Self::State) {
                $($t::update($p);)*
            }

            fn resolve(($($p,)*): &mut Self::State) {
                $($t::resolve($p);)*
            }

            fn dependencies(($($p,)*): &Self::State) -> Vec<Dependency> {
                let mut _dependencies = Vec::new();
                $(_dependencies.append(&mut $t::dependencies($p));)*
                _dependencies
            }
        }

        impl<'a, $($t: State<'a>,)*> State<'a> for ($($t,)*) {
            type Item = ($($t::Item,)*);

            fn get(&'a self) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.get(),)*)
            }
        }
    };
}

crate::recurse!(
    inject, inject0, I0, inject1, I1, inject2, I2, inject3, I3, inject4, I4, inject5, I5, inject6,
    I6, inject7, I7, inject8, I8, inject9, I9, inject10, I10, inject11, I11, inject12, I12,
    inject13, I13, inject14, I14, inject15, I15
);

use std::any::type_name;

use crate::{depend::Depend, world::World};

pub struct InjectContext<'a> {
    identifier: usize,
    world: &'a mut World,
}

pub unsafe trait Inject {
    type Input;
    type State: for<'a> Get<'a> + Depend;

    fn name() -> String {
        type_name::<Self>().into()
    }
    fn initialize(input: Self::Input, context: InjectContext) -> Option<Self::State>;
    #[inline]
    fn update(_: &mut Self::State, _: InjectContext) {}
    #[inline]
    fn resolve(_: &mut Self::State, _: InjectContext) {}
}

/// SAFETY: The implementations of the 'get' method must ensure that no reference to the 'World' are kept within 'self'
/// because it would violate this crate's lifetime requirements. In principle, this is prevented by the fact that the
/// trait is 'static and as such, it is not marked as unsafe. This note serves to prevent any unseen sneaky way to
/// retain a reference to the 'World' that lives outside of 'Item'.
pub trait Get<'a>: 'static {
    type Item;
    fn get(&'a mut self, world: &'a World) -> Self::Item;
}

pub struct Injector<I: Inject = ()>(pub I::Input);

impl<'a> InjectContext<'a> {
    #[inline]
    pub fn new(identifier: usize, world: &'a mut World) -> Self {
        Self { identifier, world }
    }

    pub fn identifier(&self) -> usize {
        self.identifier
    }

    pub fn world(&mut self) -> &mut World {
        self.world
    }

    pub fn owned(&mut self) -> InjectContext {
        InjectContext::new(self.identifier, self.world)
    }
}

macro_rules! inject {
    ($($p:ident, $t:ident),*) => {
        unsafe impl<'a, $($t: Inject,)*> Inject for ($($t,)*) {
            type Input = ($($t::Input,)*);
            type State = ($($t::State,)*);

            fn initialize(($($p,)*): Self::Input, mut _context: InjectContext) -> Option<Self::State> {
                Some(($($t::initialize($p, _context.owned())?,)*))
            }

            #[inline]
            fn update(($($p,)*): &mut Self::State, mut _context: InjectContext) {
                $($t::update($p, _context.owned());)*
            }

            #[inline]
            fn resolve(($($p,)*): &mut Self::State, mut _context: InjectContext) {
                $($t::resolve($p, _context.owned());)*
            }
        }

        impl<'a, $($t: Get<'a>,)*> Get<'a> for ($($t,)*) {
            type Item = ($($t::Item,)*);

            #[inline]
            fn get(&'a mut self, _world: &'a World) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.get(_world),)*)
            }
        }
    };
}

entia_macro::recurse_32!(inject);

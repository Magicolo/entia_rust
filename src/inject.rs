use crate::{depend::Depend, world::World};

pub struct Context {
    pub identifier: usize,
}

pub trait Inject {
    type Input;
    type State: for<'a> Get<'a> + Depend;
    fn initialize(input: Self::Input, context: &Context, world: &mut World) -> Option<Self::State>;
    #[inline]
    fn update(_: &mut Self::State, _: &mut World) {}
    #[inline]
    fn resolve(_: &mut Self::State, _: &mut World) {}
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

impl Context {
    #[inline]
    pub const fn new(identifier: usize) -> Self {
        Self { identifier }
    }
}

macro_rules! inject {
    ($($p:ident, $t:ident),*) => {
        impl<'a, $($t: Inject,)*> Inject for ($($t,)*) {
            type Input = ($($t::Input,)*);
            type State = ($($t::State,)*);

            fn initialize(($($p,)*): Self::Input, _context: &Context, _world: &mut World) -> Option<Self::State> {
                Some(($($t::initialize($p, _context, _world)?,)*))
            }

            #[inline]
            fn update(($($p,)*): &mut Self::State, _world: &mut World) {
                $($t::update($p, _world);)*
            }

            #[inline]
            fn resolve(($($p,)*): &mut Self::State, _world: &mut World) {
                $($t::resolve($p, _world);)*
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

use crate::core::call::*;
use crate::core::prepend::*;
use crate::schedule::*;
use crate::system::*;
use crate::world::*;

pub trait Inject {
    type Input;
    type State: for<'a> Get<'a>;
    fn initialize(input: Self::Input, world: &mut World) -> Option<Self::State>;
    #[inline]
    fn update(_: &mut Self::State, _: &mut World) {}
    #[inline]
    fn resolve(_: &mut Self::State, _: &mut World) {}
    fn depend(_: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Unknown]
    }
}

// SAFETY: The implementations of the 'get' method must ensure that no reference the 'World' are kept within 'self'
// because it would violate this crate's lifetime requirements. In principle, this is prevented by the fact that the
// trait is 'static and as such, it is not marked as unsafe. This note serves to prevent any unseen sneaky way to
// retain a reference to the 'World' that lives outside of 'Item'.
pub trait Get<'a>: 'static {
    type Item;
    fn get(&'a mut self, world: &'a World) -> Self::Item;
}

pub struct Injector<'a, I: Inject = ()> {
    pub(crate) input: I::Input,
    pub(crate) scheduler: Scheduler<'a>,
}

impl<'a, I: Inject> Injector<'a, I> {
    pub fn inject<T: Inject + Prepend<I>>(self) -> Injector<'a, <T as Prepend<I>>::Target>
    where
        T::Input: Default,
        <T as Prepend<I>>::Target: Inject,
        T::Input: Prepend<I::Input, Target = <<T as Prepend<I>>::Target as Inject>::Input>,
    {
        self.inject_with::<T>(T::Input::default())
    }

    pub fn inject_with<T: Inject + Prepend<I>>(
        self,
        input: T::Input,
    ) -> Injector<'a, <T as Prepend<I>>::Target>
    where
        <T as Prepend<I>>::Target: Inject,
        T::Input: Prepend<I::Input, Target = <<T as Prepend<I>>::Target as Inject>::Input>,
    {
        Injector {
            input: input.prepend(self.input),
            scheduler: self.scheduler,
        }
    }

    pub fn schedule<C: Call<I, ()> + Call<<I::State as Get<'a>>::Item, ()> + 'static>(
        self,
        schedule: C,
    ) -> Scheduler<'a> {
        <(I::Input, C) as Schedule<'a, [I; 1]>>::schedule((self.input, schedule), self.scheduler)
    }
}

macro_rules! inject {
    ($($p:ident, $t:ident),*) => {
        impl<'a, $($t: Inject,)*> Inject for ($($t,)*) {
            type Input = ($($t::Input,)*);
            type State = ($($t::State,)*);

            fn initialize(($($p,)*): Self::Input, _world: &mut World) -> Option<Self::State> {
                Some(($($t::initialize($p, _world)?,)*))
            }

            #[inline]
            fn update(($($p,)*): &mut Self::State, _world: &mut World) {
                $($t::update($p, _world);)*
            }

            #[inline]
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
            fn get(&'a mut self, _world: &'a World) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.get(_world),)*)
            }
        }
    };
}

entia_macro::recurse_32!(inject);

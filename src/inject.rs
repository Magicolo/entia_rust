use crate::{
    depend::{Conflict, Depend, Dependency, Scope},
    system::Error,
    world::World,
};
use entia_core::utility::short_type_name;
use std::marker::PhantomData;

pub struct Injector<I: Inject> {
    identifier: usize,
    name: String,
    world: usize,
    version: usize,
    state: I::State,
    dependencies: Vec<Dependency>,
    _marker: PhantomData<I>,
}

pub struct Guard<'a, I: Inject> {
    identifier: usize,
    state: &'a mut I::State,
    world: &'a mut World,
}

pub struct Context<'a> {
    identifier: usize,
    world: &'a mut World,
}

pub trait Inject {
    type Input;
    type State: for<'a> Get<'a> + Depend;

    fn name() -> String {
        short_type_name::<Self>()
    }
    fn initialize(input: Self::Input, context: Context) -> Option<Self::State>;
    #[inline]
    fn update(_: &mut Self::State, _: Context) {}
    #[inline]
    fn resolve(_: &mut Self::State, _: Context) {}
}

/// SAFETY: The implementations of the 'get' method must ensure that no reference to the 'World' are kept within 'self'
/// because it would violate this crate's lifetime requirements. In principle, this is prevented by the fact that the
/// trait is 'static and as such, it is not marked as unsafe. This note serves to prevent any unseen sneaky way to
/// retain a reference to the 'World' that lives outside of 'Item'.
pub trait Get<'a> {
    type Item;
    fn get(&'a mut self, world: &'a World) -> Self::Item;
}

impl<'a> Context<'a> {
    #[inline]
    pub fn new(identifier: usize, world: &'a mut World) -> Self {
        Self { identifier, world }
    }

    #[inline]
    pub const fn identifier(&self) -> usize {
        self.identifier
    }

    #[inline]
    pub fn world(&mut self) -> &mut World {
        self.world
    }

    #[inline]
    pub fn owned(&mut self) -> Context {
        Context::new(self.identifier, self.world)
    }
}

macro_rules! inject {
    ($($p:ident, $t:ident),*) => {
        impl<'a, $($t: Inject,)*> Inject for ($($t,)*) {
            type Input = ($($t::Input,)*);
            type State = ($($t::State,)*);

            fn initialize(($($p,)*): Self::Input, mut _context: Context) -> Option<Self::State> {
                Some(($($t::initialize($p, _context.owned())?,)*))
            }

            #[inline]
            fn update(($($p,)*): &mut Self::State, mut _context: Context) {
                $($t::update($p, _context.owned());)*
            }

            #[inline]
            fn resolve(($($p,)*): &mut Self::State, mut _context: Context) {
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

impl World {
    pub fn injector<I: Inject>(&mut self) -> Result<Injector<I>, Error>
    where
        I::Input: Default,
    {
        self.injector_with(I::Input::default())
    }

    pub fn injector_with<I: Inject>(&mut self, input: I::Input) -> Result<Injector<I>, Error> {
        let identifier = World::reserve();
        let state =
            I::initialize(input, Context::new(identifier, self)).ok_or(Error::FailedToInject)?;
        Ok(Injector {
            identifier,
            name: I::name(),
            world: self.identifier(),
            version: 0,
            state,
            dependencies: Vec::new(),
            _marker: PhantomData,
        })
    }
}

impl<I: Inject> Injector<I> {
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn version(&self) -> usize {
        self.version
    }

    pub fn update<'a>(&mut self, world: &mut World) -> Result<(), Error> {
        if self.world != world.identifier() {
            Err(Error::WrongWorld)
        } else if self.version != world.version() {
            I::update(&mut self.state, Context::new(self.identifier, world));
            self.dependencies = self.state.depend(world);
            let mut conflict = Conflict::default();
            match conflict.detect(Scope::Inner, &self.dependencies) {
                Ok(_) => Ok(()),
                Err(error) => Err(Error::InnerConflict(self.name().into(), error.into())),
            }
        } else {
            Ok(())
        }
    }

    pub fn guard<'a>(&'a mut self, world: &'a mut World) -> Result<Guard<'a, I>, Error> {
        self.update(world)?;
        Ok(Guard {
            identifier: self.identifier,
            state: &mut self.state,
            world,
        })
    }
}

impl<'a, I: Inject> Guard<'a, I> {
    pub fn inject(&mut self) -> <I::State as Get<'_>>::Item {
        self.state.get(self.world)
    }
}

impl<I: Inject> Drop for Guard<'_, I> {
    fn drop(&mut self) {
        I::resolve(self.state, Context::new(self.identifier, self.world));
    }
}

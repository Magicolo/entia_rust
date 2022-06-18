use crate::{
    depend::{Conflict, Depend, Dependency, Scope},
    error::{Error, Result},
    recurse,
    world::World,
};
use entia_core::{utility::short_type_name, Change};
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

pub struct Context<'a> {
    identifier: usize,
    world: &'a mut World,
}

pub trait Inject {
    type Input;
    type State: for<'a> Get<'a> + Depend;

    #[inline]
    fn name() -> String {
        short_type_name::<Self>()
    }

    fn initialize(input: Self::Input, context: Context) -> Result<Self::State>;

    #[inline]
    fn update(_: &mut Self::State, _: Context) -> Result {
        Ok(())
    }

    #[inline]
    fn resolve(_: &mut Self::State, _: Context) -> Result {
        Ok(())
    }
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

impl World {
    pub fn injector<I: Inject>(&mut self) -> Result<Injector<I>>
    where
        I::Input: Default,
    {
        self.injector_with(I::Input::default())
    }

    pub fn injector_with<I: Inject>(&mut self, input: I::Input) -> Result<Injector<I>> {
        let identifier = World::reserve();
        let state = I::initialize(input, Context::new(identifier, self))?;
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

    pub fn update<'a>(&mut self, world: &mut World) -> Result {
        if self.world != world.identifier() {
            return Err(Error::WrongWorld);
        }

        let mut version = self.version;
        while version.change(world.version()) {
            I::update(&mut self.state, Context::new(self.identifier, world))?;
        }
        // Commit the version only if the 'I::update' is a success such that it may continue to fail on the next call if it failed on this call.
        self.version = version;

        self.dependencies = self.state.depend(world);
        let mut conflict = Conflict::default();
        conflict
            .detect(Scope::Inner, &self.dependencies)
            .map_err(Error::Depend)
    }

    pub fn resolve(&mut self, world: &mut World) -> Result {
        if self.world != world.identifier() {
            Err(Error::WrongWorld)
        } else {
            I::resolve(&mut self.state, Context::new(self.identifier, world))
        }
    }

    pub fn run<T, F: FnOnce(<I::State as Get<'_>>::Item) -> T>(
        &mut self,
        world: &mut World,
        run: F,
    ) -> Result<T> {
        self.update(world)?;
        let value = run(self.state.get(world));
        self.resolve(world)?;
        Ok(value)
    }
}

impl<I: Inject, const N: usize> Inject for [I; N] {
    type Input = [I::Input; N];
    type State = [I::State; N];

    fn initialize(input: Self::Input, mut context: Context) -> Result<Self::State> {
        let mut items = [(); N].map(|_| None);
        for (i, input) in input.into_iter().enumerate() {
            items[i] = Some(I::initialize(input, context.owned())?);
        }
        Ok(items.map(Option::unwrap))
    }

    #[inline]
    fn update(state: &mut Self::State, mut context: Context) -> Result {
        for state in state {
            I::update(state, context.owned())?;
        }
        Ok(())
    }

    #[inline]
    fn resolve(state: &mut Self::State, mut context: Context) -> Result {
        for state in state {
            I::resolve(state, context.owned())?;
        }
        Ok(())
    }
}

impl<'a, T: Get<'a>, const N: usize> Get<'a> for [T; N] {
    type Item = [T::Item; N];

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        let mut iterator = self.iter_mut();
        [(); N].map(|_| iterator.next().unwrap().get(world))
    }
}

unsafe impl<T: Depend, const N: usize> Depend for [T; N] {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for item in self {
            dependencies.append(&mut item.depend(world));
        }
        dependencies
    }
}

impl<T> Inject for PhantomData<T> {
    type Input = <() as Inject>::Input;
    type State = <() as Inject>::State;
    fn initialize(input: Self::Input, context: Context) -> Result<Self::State> {
        <()>::initialize(input, context)
    }
    fn update(state: &mut Self::State, context: Context) -> Result {
        <()>::update(state, context)
    }
    fn resolve(state: &mut Self::State, context: Context) -> Result {
        <()>::resolve(state, context)
    }
}

macro_rules! inject {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Inject,)*> Inject for ($($t,)*) {
            type Input = ($($t::Input,)*);
            type State = ($($t::State,)*);

            fn initialize(($($p,)*): Self::Input, mut _context: Context) -> Result<Self::State> {
                Ok(($($t::initialize($p, _context.owned())?,)*))
            }

            fn update(($($p,)*): &mut Self::State, mut _context: Context) -> Result {
                $($t::update($p, _context.owned())?;)*
                Ok(())
            }

            #[inline]
            fn resolve(($($p,)*): &mut Self::State, mut _context: Context) -> Result {
                $($t::resolve($p, _context.owned())?;)*
                Ok(())
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

recurse!(inject);

use crate::{
    depend::{Depend, Dependency},
    error::{Error, Result},
    inject::{Context, Get, Inject},
    output::IntoOutput,
    world::World,
};
use entia_core::{utility::short_type_name, Call};
use std::{
    any::Any,
    fmt::{self},
    result,
};

pub struct System {
    identifier: usize,
    pub(crate) name: String,
    state: Box<dyn Any + Send>,
    run: Box<dyn Fn(&mut (dyn Any + Send), usize, &World) -> Result + Send + Sync>,
    update: Box<dyn Fn(&mut dyn Any, usize, &mut World) -> Result>,
    resolve: Box<dyn Fn(&mut dyn Any, usize, &mut World) -> Result>,
    depend: Box<dyn Fn(&dyn Any, usize, &World) -> Vec<Dependency>>,
}

pub trait IntoSystem<'a, M = ()> {
    type Input;
    fn system(self, input: Self::Input, world: &mut World) -> Result<System>;
}

impl System {
    pub fn new<'a, T: Send + Sync + 'static>(
        name: String,
        initialize: impl FnOnce(usize) -> Result<T>,
        run: impl Fn(&'a mut T, usize, &'a World) -> Result + Send + Sync + 'static,
        update: impl Fn(&mut T, usize, &mut World) -> Result + 'static,
        resolve: impl Fn(&mut T, usize, &mut World) -> Result + 'static,
        depend: impl Fn(&T, usize, &World) -> Vec<Dependency> + 'static,
    ) -> Result<Self> {
        let identifier = World::reserve();
        Ok(Self {
            name,
            identifier,
            state: Box::new(initialize(identifier)?),
            run: Box::new(move |state, identifier, world| {
                let state = unsafe { state.downcast_mut::<T>().unwrap_unchecked() };
                unsafe { run(&mut *(state as *mut _), identifier, &*(world as *const _)) }
            }),
            update: Box::new(move |state, identifier, world| {
                let state = unsafe { state.downcast_mut().unwrap_unchecked() };
                update(state, identifier, world)
            }),
            resolve: Box::new(move |state, identifier, world| {
                let state = unsafe { state.downcast_mut().unwrap_unchecked() };
                resolve(state, identifier, world)
            }),
            depend: Box::new(move |state, identifier, world| {
                let state = unsafe { state.downcast_ref().unwrap_unchecked() };
                depend(state, identifier, world)
            }),
        })
    }

    #[inline]
    pub const fn identifier(&self) -> usize {
        self.identifier
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn run(&mut self, world: &World) -> Result {
        (self.run)(&mut self.state, self.identifier, world)
    }

    #[inline]
    pub fn update(&mut self, world: &mut World) -> Result {
        (self.update)(&mut self.state, self.identifier, world)
    }

    #[inline]
    pub fn resolve(&mut self, world: &mut World) -> Result {
        (self.resolve)(&mut self.state, self.identifier, world)
    }

    #[inline]
    pub fn depend(&mut self, world: &World) -> Vec<Dependency> {
        (self.depend)(&self.state, self.identifier, world)
    }
}

impl fmt::Debug for System {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&short_type_name::<Self>())
            .field(&self.name())
            .finish()
    }
}

impl<I, S> IntoSystem<'_> for (I, S)
where
    (I, S): Into<System>,
{
    type Input = ();

    fn system(self, _: Self::Input, _: &mut World) -> Result<System> {
        Ok(self.into())
    }
}

impl<'a, M, S: IntoSystem<'a, M>> IntoSystem<'a, (M, S)> for Option<S> {
    type Input = S::Input;

    fn system(self, input: Self::Input, world: &mut World) -> Result<System> {
        match self {
            Some(system) => system.system(input, world),
            None => Err(Error::MissingSystem),
        }
    }
}

impl<'a, M, S: IntoSystem<'a, M>, E: Into<Error>> IntoSystem<'a, (M, S)> for result::Result<S, E> {
    type Input = S::Input;

    fn system(self, input: Self::Input, world: &mut World) -> Result<System> {
        match self {
            Ok(system) => system.system(input, world),
            Err(error) => Err(error.into()),
        }
    }
}

impl<
        'a,
        I: Inject,
        O: IntoOutput,
        C: Call<I, O> + Call<<I::State as Get<'a>>::Item, O> + Send + Sync + 'static,
    > IntoSystem<'a, (I, O, C)> for C
where
    I::State: Send + Sync + 'static,
{
    type Input = I::Input;

    fn system(self, input: I::Input, world: &mut World) -> Result<System> {
        System::new(
            I::name(),
            |identifier| Ok((self, I::initialize(input, Context::new(identifier, world))?)),
            |(run, state), _, world| run.call(state.get(world)).output(),
            |(_, state), identifier, world| I::update(state, Context::new(identifier, world)),
            |(_, state), identifier, world| I::resolve(state, Context::new(identifier, world)),
            |(_, state), _, world| state.depend(world),
        )
    }
}

pub struct Barrier;

impl IntoSystem<'_> for Barrier {
    type Input = ();

    fn system(self, _: Self::Input, _: &mut World) -> Result<System> {
        System::new(
            "barrier".into(),
            |_| Ok(()),
            |_, _, _| Ok(()),
            |_, _, _| Ok(()),
            |_, _, _| Ok(()),
            |_, _, _| vec![Dependency::Unknown],
        )
    }
}

use crate::{
    depend::{Depend, Dependency},
    error::{Error, Result},
    identify,
    inject::{Get, Inject},
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
    run: Box<dyn Fn(&mut (dyn Any + Send)) -> Result + Send + Sync>,
    update: Box<dyn Fn(&mut dyn Any, &mut World) -> Result>,
    resolve: Box<dyn Fn(&mut dyn Any) -> Result>,
    depend: Box<dyn Fn(&dyn Any) -> Vec<Dependency>>,
}

pub trait IntoSystem<M = ()> {
    type Input;
    fn system(self, input: Self::Input, world: &mut World) -> Result<System>;
}

impl System {
    pub fn new<'a, T: Send + Sync + 'static>(
        name: String,
        initialize: impl FnOnce(usize) -> Result<T>,
        run: impl Fn(&'a mut T) -> Result + Send + Sync + 'static,
        update: impl Fn(&mut T, &mut World) -> Result + 'static,
        resolve: impl Fn(&mut T) -> Result + 'static,
        depend: impl Fn(&T) -> Vec<Dependency> + 'static,
    ) -> Result<Self> {
        let identifier = identify();
        Ok(Self {
            name,
            identifier,
            state: Box::new(initialize(identifier)?),
            run: Box::new(move |state| {
                // let state = unsafe { state.downcast_mut::<T>().unwrap_unchecked() };
                unsafe { run(&mut *(state as *mut _ as *mut T)) }
            }),
            update: Box::new(move |state, world| {
                update(unsafe { state.downcast_mut().unwrap_unchecked() }, world)
            }),
            resolve: Box::new(move |state| {
                resolve(unsafe { state.downcast_mut().unwrap_unchecked() })
            }),
            depend: Box::new(move |state| {
                depend(unsafe { state.downcast_ref().unwrap_unchecked() })
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
    pub fn run(&mut self) -> Result {
        (self.run)(&mut self.state)
    }

    #[inline]
    pub fn update(&mut self, world: &mut World) -> Result {
        (self.update)(&mut self.state, world)
    }

    #[inline]
    pub fn resolve(&mut self) -> Result {
        (self.resolve)(&mut self.state)
    }

    #[inline]
    pub fn depend(&mut self) -> Vec<Dependency> {
        (self.depend)(&self.state)
    }
}

impl fmt::Debug for System {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&short_type_name::<Self>())
            .field(&self.name())
            .finish()
    }
}

impl<I, S> IntoSystem for (I, S)
where
    (I, S): Into<System>,
{
    type Input = ();

    fn system(self, _: Self::Input, _: &mut World) -> Result<System> {
        Ok(self.into())
    }
}

impl<M, S: IntoSystem<M>> IntoSystem<(M, S)> for Option<S> {
    type Input = S::Input;

    fn system(self, input: Self::Input, world: &mut World) -> Result<System> {
        match self {
            Some(system) => system.system(input, world),
            None => Err(Error::MissingSystem),
        }
    }
}

impl<M, S: IntoSystem<M>, E: Into<Error>> IntoSystem<(M, S)> for result::Result<S, E> {
    type Input = S::Input;

    fn system(self, input: Self::Input, world: &mut World) -> Result<System> {
        match self {
            Ok(system) => system.system(input, world),
            Err(error) => Err(error.into()),
        }
    }
}

impl<'a, I: Inject, O: IntoOutput, C: Call<I, O> + Send + Sync + 'static> IntoSystem<(I, O, C)>
    for C
where
    I::State: Get<'a, Item = I> + Send + Sync + 'static,
{
    type Input = I::Input;

    fn system(self, input: I::Input, world: &mut World) -> Result<System> {
        System::new(
            I::name(),
            |identifier| {
                let state = I::initialize(input, identifier, world)?;
                world.modify();
                Ok((self, state))
            },
            |(run, state)| run.call(unsafe { state.get() }).output(),
            |(_, state), world| I::update(state, world),
            |(_, state)| I::resolve(state),
            |(_, state)| state.depend(),
        )
    }
}

pub struct Barrier;

impl IntoSystem for Barrier {
    type Input = ();

    fn system(self, _: Self::Input, _: &mut World) -> Result<System> {
        System::new(
            "barrier".into(),
            |_| Ok(()),
            |_| Ok(()),
            |_, _| Ok(()),
            |_| Ok(()),
            |_| vec![Dependency::Unknown],
        )
    }
}

use crate::{
    depend::{Depend, Dependency},
    error::{Error, Result},
    inject::{Context, Get, Inject},
    output::IntoOutput,
    world::World,
};
use entia_core::{utility::short_type_name, Call};
use std::{
    fmt::{self},
    result,
    sync::Arc,
};

pub struct System {
    identifier: usize,
    pub(crate) name: String,
    pub(crate) run: Box<dyn Fn(&World) -> Result + Send>,
    pub(crate) update: Box<dyn Fn(&mut World) -> Result + Send>,
    pub(crate) resolve: Box<dyn Fn(&mut World) -> Result + Send>,
    pub(crate) depend: Box<dyn Fn(&World) -> Vec<Dependency> + Send>,
}

pub trait IntoSystem<'a, M = ()> {
    type Input;
    fn system(self, input: Self::Input, world: &mut World) -> Result<System>;
}

impl<S: Into<System>> IntoSystem<'_> for S {
    type Input = ();

    fn system(self, _: Self::Input, _: &mut World) -> Result<System> {
        Ok(self.into())
    }
}

impl<I, S> IntoSystem<'_, (I, S)> for (I, S)
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

impl System {
    #[inline]
    pub fn new<'a, T: Send + Sync + 'static, F: FnOnce(usize) -> Result<T>>(
        name: String,
        initialize: F,
        run: fn(&'a mut T, usize, &'a World) -> Result,
        update: fn(&mut T, usize, &mut World) -> Result,
        resolve: fn(&mut T, usize, &mut World) -> Result,
        depend: fn(&T, usize, &World) -> Vec<Dependency>,
    ) -> Result<Self> {
        let identifier = World::reserve();
        let state = Arc::new(initialize(identifier)?);

        #[inline]
        fn get<'a, T>(state: &Arc<T>) -> &'a mut T {
            unsafe { &mut *(Arc::as_ptr(state) as *mut _) }
        }

        // SAFETY: The scheduler and runner will ensure that none of a given system's functions are called in parallel.
        Ok(Self {
            name,
            identifier,
            run: unsafe {
                let state = state.clone();
                Box::new(move |world| run(get(&state), identifier, &*(world as *const _)))
            },
            update: unsafe {
                let state = state.clone();
                Box::new(move |world| update(get(&state), identifier, &mut *(world as *mut _)))
            },
            resolve: unsafe {
                let state = state.clone();
                Box::new(move |world| resolve(get(&state), identifier, &mut *(world as *mut _)))
            },
            depend: unsafe {
                let state = state.clone();
                Box::new(move |world| depend(get(&state), identifier, &*(world as *const _)))
            },
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
}

impl fmt::Debug for System {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&short_type_name::<Self>())
            .field(&self.name())
            .finish()
    }
}

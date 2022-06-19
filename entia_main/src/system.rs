use crate::{
    depend::{Depend, Dependency},
    error::{Error, Result},
    inject::{Context, Get, Inject},
    output::IntoOutput,
    world::World,
};
use entia_core::{utility::short_type_name, Call};
use std::{
    cell::UnsafeCell,
    fmt::{self},
    result,
    sync::Arc,
};

pub struct System {
    identifier: usize,
    pub(crate) name: String,
    pub(crate) run: Box<dyn FnMut(&World) -> Result>,
    pub(crate) update: Box<dyn FnMut(&mut World) -> Result>,
    pub(crate) resolve: Box<dyn FnMut(&mut World) -> Result>,
    pub(crate) depend: Box<dyn Fn(&World) -> Vec<Dependency>>,
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
        C: Call<I, O> + Call<<I::State as Get<'a>>::Item, O> + Send + 'static,
    > IntoSystem<'a, (I, O, C)> for C
where
    I::State: Send + 'static,
{
    type Input = I::Input;

    fn system(self, input: I::Input, world: &mut World) -> Result<System> {
        let identifier = World::reserve();
        let state = I::initialize(input, Context::new(identifier, world))?;
        let system = unsafe {
            System::new(
                Some(identifier),
                I::name(),
                (self, state, identifier),
                |(run, state, _), world| run.call(state.get(world)).output(),
                |(_, state, identifier), world| I::update(state, Context::new(*identifier, world)),
                |(_, state, identifier), world| I::resolve(state, Context::new(*identifier, world)),
                |(_, state, _), world| state.depend(world),
            )
        };
        Ok(system)
    }
}

impl System {
    pub unsafe fn new<'a, T: 'static>(
        identifier: Option<usize>,
        name: String,
        state: T,
        run: fn(&'a mut T, &'a World) -> Result,
        update: fn(&'a mut T, &'a mut World) -> Result,
        resolve: fn(&'a mut T, &'a mut World) -> Result,
        depend: fn(&'a T, &'a World) -> Vec<Dependency>,
    ) -> Self {
        struct State<T>(Arc<UnsafeCell<T>>);
        unsafe impl<T> Send for State<T> {}
        impl<T> State<T> {
            #[inline]
            pub unsafe fn get<'a>(&self) -> &'a mut T {
                &mut *self.0.get()
            }
        }

        // SAFETY: Since this crate controls the execution of the system's functions, it can guarantee
        // that they are not run in parallel which would allow for races.

        // SAFETY: The 'new' function is declared as unsafe because the user must guarantee that no reference
        // to the 'World' outlives the call of the function pointers. Normally this could be enforced by Rust but
        // there seem to be a limitation in the expressivity of the type system to be able to express the desired
        // intention.

        let identifier = identifier.unwrap_or_else(World::reserve);
        let state = Arc::new(UnsafeCell::new(state));
        Self {
            name,
            identifier,
            run: {
                let state = State(state.clone());
                Box::new(move |world| run(state.get(), &*(world as *const _)))
            },
            update: {
                let state = State(state.clone());
                Box::new(move |world| update(state.get(), &mut *(world as *mut _)))
            },
            resolve: {
                let state = State(state.clone());
                Box::new(move |world| resolve(state.get(), &mut *(world as *mut _)))
            },
            depend: {
                let state = State(state.clone());
                Box::new(move |world| depend(state.get(), &*(world as *const _)))
            },
        }
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

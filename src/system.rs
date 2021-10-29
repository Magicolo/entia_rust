use self::runner::*;
use crate::{
    depend::{Conflict, Depend, Dependency, Scope},
    inject::{Context, Get, Inject},
    world::World,
};
use entia_core::{utility::short_type_name, Call};
use std::{
    any::type_name,
    cell::UnsafeCell,
    error,
    fmt::{self, Display},
    mem::replace,
    mem::swap,
    sync::Arc,
};

pub struct System {
    identifier: usize,
    name: String, // TODO: Replace with 'Lazy<String>'
    pub(crate) run: Box<dyn FnMut(&World)>,
    pub(crate) update: Box<dyn FnMut(&mut World)>,
    pub(crate) resolve: Box<dyn FnMut(&mut World)>,
    pub(crate) depend: Box<dyn Fn(&World) -> Vec<Dependency>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Error {
    WrongWorld,
    MissingSystem,
    FailedToInject,
    InnerConflict(String, Box<Error>),
    OuterConflict(String, Box<Error>),
    UnknownConflict,
    ReadWriteConflict(String, Option<usize>),
    WriteWriteConflict(String, Option<usize>),
    ReadDeferConflict(String, Option<usize>),
    WriteDeferConflict(String, Option<usize>),
    All(Vec<Error>),
}

pub trait IntoSystem<I, M = ()> {
    fn into_system(self, input: I, world: &mut World) -> Result<System, Error>;
}

impl<S: Into<System>> IntoSystem<()> for S {
    fn into_system(self, _: (), _: &mut World) -> Result<System, Error> {
        Ok(self.into())
    }
}

impl<I, S> IntoSystem<(), (I, S)> for (I, S)
where
    (I, S): Into<System>,
{
    fn into_system(self, _: (), _: &mut World) -> Result<System, Error> {
        Ok(self.into())
    }
}

impl<I, M, S: IntoSystem<I, M>> IntoSystem<I, (I, M, S)> for Option<S> {
    fn into_system(self, input: I, world: &mut World) -> Result<System, Error> {
        match self {
            Some(system) => system.into_system(input, world),
            None => Err(Error::MissingSystem),
        }
    }
}

impl<I, M, S: IntoSystem<I, M>, E: Into<Error>> IntoSystem<I, (I, M, S)> for Result<S, E> {
    fn into_system(self, input: I, world: &mut World) -> Result<System, Error> {
        match self {
            Ok(system) => system.into_system(input, world),
            Err(error) => Err(error.into()),
        }
    }
}

impl<'a, I: Inject, O, C: Call<I, O> + Call<<I::State as Get<'a>>::Item, O> + 'static>
    IntoSystem<I::Input, (I, O, C)> for C
{
    fn into_system(self, input: I::Input, world: &mut World) -> Result<System, Error> {
        let identifier = World::reserve();
        let state =
            I::initialize(input, Context::new(identifier, world)).ok_or(Error::FailedToInject)?;
        let system = unsafe {
            System::new(
                Some(identifier),
                I::name(),
                (self, state, identifier),
                |(run, state, _), world| {
                    run.call(state.get(world));
                },
                |(_, state, identifier), world| I::update(state, Context::new(*identifier, world)),
                |(_, state, identifier), world| I::resolve(state, Context::new(*identifier, world)),
                |(_, state, _), world| state.depend(world),
            )
        };
        Ok(system)
    }
}

impl Error {
    pub fn merge(self, error: Self) -> Self {
        match (self, error) {
            (Error::All(mut left), Error::All(mut right)) => {
                left.append(&mut right);
                Error::All(left)
            }
            (Error::All(mut left), right) => {
                left.push(right);
                Error::All(left)
            }
            (left, Error::All(mut right)) => {
                right.insert(0, left);
                Error::All(right)
            }
            (left, right) => Error::All(vec![left, right]),
        }
    }

    pub fn flatten(self, recursive: bool) -> Option<Self> {
        fn descend(error: Error, errors: &mut Vec<Error>, recursive: bool) {
            match error {
                Error::All(mut inner) => {
                    if recursive {
                        for error in inner {
                            descend(error, errors, recursive);
                        }
                    } else {
                        errors.append(&mut inner);
                    }
                }
                error => errors.push(error),
            }
        }

        let mut errors = Vec::new();
        descend(self, &mut errors, recursive);

        if errors.len() == 0 {
            None
        } else if errors.len() == 1 {
            Some(errors.into_iter().next().unwrap())
        } else {
            Some(Error::All(errors))
        }
    }
}

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl System {
    #[inline]
    pub unsafe fn new<'a, T: 'static>(
        identifier: Option<usize>,
        name: String,
        state: T,
        run: fn(&'a mut T, &'a World),
        update: fn(&'a mut T, &'a mut World),
        resolve: fn(&'a mut T, &'a mut World),
        depend: fn(&'a T, &'a World) -> Vec<Dependency>,
    ) -> Self {
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
                let state = state.clone();
                Box::new(move |world| unsafe { run(&mut *state.get(), &*(world as *const _)) })
            },
            update: {
                let state = state.clone();
                Box::new(move |world| unsafe { update(&mut *state.get(), &mut *(world as *mut _)) })
            },
            resolve: {
                let state = state.clone();
                Box::new(move |world| unsafe {
                    resolve(&mut *state.get(), &mut *(world as *mut _))
                })
            },
            depend: {
                let state = state.clone();
                Box::new(move |world| unsafe { depend(&*state.get(), &*(world as *const _)) })
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

pub mod runner {
    use super::*;

    pub struct Runner {
        identifier: usize,
        version: usize,
        blocks: Vec<Block>,
    }

    #[derive(Default)]
    struct Block {
        systems: Vec<System>,
        dependencies: Vec<Dependency>,
        error: Option<Error>,
    }

    unsafe impl Send for System {}

    impl Runner {
        pub fn new(identifier: usize, systems: impl IntoIterator<Item = System>) -> Self {
            Self {
                identifier,
                version: 0,
                blocks: systems.into_iter().map(Into::into).collect(),
            }
        }

        pub fn update(&mut self, world: &mut World) -> Result<(), Error> {
            if self.identifier != world.identifier() {
                return Err(Error::WrongWorld);
            }

            if self.version != world.version() {
                // This will retry scheduling on each call as long as there are dependency errors.
                self.schedule(world)?;
                self.version = world.version();
            }

            Ok(())
        }

        pub fn run(&mut self, world: &mut World) -> Result<(), Error> {
            self.update(world)?;

            for block in self.blocks.iter_mut() {
                // If segments have been added to the world, this may mean that the dependencies used to schedule the systems
                // are not up to date, therefore it is not safe to run the systems in parallel.
                if self.version == world.version() {
                    if block.systems.len() > 1 {
                        use rayon::prelude::*;
                        block
                            .systems
                            .par_iter_mut()
                            .for_each(|system| (system.run)(world));
                    } else {
                        for system in block.systems.iter_mut() {
                            (system.run)(world);
                        }
                    }
                } else {
                    for system in block.systems.iter_mut() {
                        (system.update)(world);
                        (system.run)(world);
                    }
                }

                for system in block.systems.iter_mut() {
                    (system.resolve)(world);
                }
            }

            Ok(())
        }

        fn schedule(&mut self, world: &mut World) -> Result<(), Error> {
            let mut blocks = Vec::new();
            let mut block = Block::default();
            let mut inner = Conflict::default();
            let mut outer = Conflict::default();
            let mut errors = Vec::new();

            for mut system in self.blocks.drain(..).flat_map(|block| block.systems) {
                (system.update)(world);

                let mut dependencies = (system.depend)(world);
                match inner.detect(Scope::Inner, &dependencies) {
                    Ok(()) => {}
                    Err(error) => {
                        errors.push(Error::InnerConflict(system.name().into(), error.into()))
                    }
                }

                match outer.detect(Scope::Outer, &dependencies) {
                    Ok(_) => {}
                    Err(error) => {
                        // TODO: When 'outer_conflicts' are detected, can later systems be still included in the block if they do not
                        // have 'outer_conflicts'? Dependencies would need to be accumulated even for conflicting systems and a system
                        // that has a 'Dependency::Unknown' should never be crossed.
                        if block.systems.len() > 0 {
                            block.error =
                                Some(Error::OuterConflict(system.name().into(), error.into()));
                            blocks.push(replace(&mut block, Block::default()));
                        }
                        swap(&mut inner, &mut outer);
                    }
                }

                block.systems.push(system);
                block.dependencies.append(&mut dependencies);
                inner.clear();
            }

            if block.systems.len() > 0 {
                blocks.push(block);
            }

            self.blocks = blocks;
            Error::All(errors).flatten(true).map(Err).unwrap_or(Ok(()))
        }
    }

    impl Into<Block> for System {
        fn into(self) -> Block {
            let mut block = Block::default();
            block.systems.push(self);
            block
        }
    }
}

pub mod schedule {
    use super::*;

    pub struct Scheduler<'a> {
        pub(crate) prefix: String,
        pub(crate) systems: Vec<Result<System, Error>>,
        pub(crate) world: &'a mut World,
    }

    impl World {
        pub fn scheduler(&mut self) -> Scheduler {
            Scheduler {
                prefix: String::new(),
                systems: Vec::new(),
                world: self,
            }
        }
    }

    impl<'a> Scheduler<'a> {
        pub fn pipe<F: FnOnce(Self) -> Self>(self, schedule: F) -> Self {
            self.with_prefix::<F, _>(schedule)
        }

        pub fn add<I: Default, M, S: IntoSystem<I, M>>(self, system: S) -> Self {
            self.add_with(I::default(), system)
        }

        pub fn add_with<I, M, S: IntoSystem<I, M>>(self, input: I, system: S) -> Self {
            self.with_prefix::<S, _>(|mut scheduler| {
                let system = system
                    .into_system(input, scheduler.world)
                    .map(|mut system| {
                        system.name.insert_str(0, &scheduler.prefix);
                        system
                    });
                scheduler.systems.push(system);
                scheduler
            })
        }

        pub fn barrier(self) -> Self {
            self.add(unsafe {
                System::new(
                    None,
                    "barrier".into(),
                    (),
                    |_, _| {},
                    |_, _| {},
                    |_, _| {},
                    |_, _| vec![Dependency::Unknown],
                )
            })
        }

        pub fn schedule(self) -> Result<Runner, Error> {
            let mut systems = Vec::new();
            let mut errors = Vec::new();

            for system in self.systems {
                match system {
                    Ok(system) => systems.push(system),
                    Err(error) => errors.push(error),
                }
            }

            match Error::All(errors).flatten(true) {
                Some(error) => Err(error),
                None => Ok(Runner::new(self.world.identifier(), systems)),
            }
        }

        fn with_prefix<T, F: FnOnce(Self) -> Self>(mut self, with: F) -> Self {
            let count = self.prefix.len();
            let prefix = if count == 0 {
                format!("{}::", type_name::<T>())
            } else {
                format!("{}::", short_type_name::<T>())
            };
            self.prefix.push_str(&prefix);
            self = with(self);
            self.prefix.truncate(count);
            self
        }
    }
}

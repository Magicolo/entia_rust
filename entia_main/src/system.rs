use self::{output::*, runner::*};
use crate::{
    depend::{Conflict, Depend, Dependency, Scope},
    error::{Error, Result},
    inject::{Context, Get, Inject},
    recurse,
    world::World,
};
use entia_core::{utility::short_type_name, Call};
use std::{
    any::type_name,
    cell::UnsafeCell,
    fmt::{self},
    result,
    sync::Arc,
};

pub struct System {
    identifier: usize,
    name: String, // TODO: Replace with 'Lazy<String>'
    pub(crate) run: Box<dyn FnMut(&World) -> Result + Send>,
    pub(crate) update: Box<dyn FnMut(&mut World) -> Result + Send>,
    pub(crate) resolve: Box<dyn FnMut(&mut World) -> Result + Send>,
    pub(crate) depend: Box<dyn Fn(&World) -> Vec<Dependency> + Send>,
}

pub trait IntoSystem<'a, I, M = ()> {
    fn system(self, input: I, world: &mut World) -> Result<System>;
}

impl<S: Into<System>> IntoSystem<'_, ()> for S {
    fn system(self, _: (), _: &mut World) -> Result<System> {
        Ok(self.into())
    }
}

impl<I, S> IntoSystem<'_, (), (I, S)> for (I, S)
where
    (I, S): Into<System>,
{
    fn system(self, _: (), _: &mut World) -> Result<System> {
        Ok(self.into())
    }
}

impl<'a, I, M, S: IntoSystem<'a, I, M>> IntoSystem<'a, I, (I, M, S)> for Option<S> {
    fn system(self, input: I, world: &mut World) -> Result<System> {
        match self {
            Some(system) => system.system(input, world),
            None => Err(Error::MissingSystem),
        }
    }
}

impl<'a, I, M, S: IntoSystem<'a, I, M>, E: Into<Error>> IntoSystem<'a, I, (I, M, S)>
    for result::Result<S, E>
{
    fn system(self, input: I, world: &mut World) -> Result<System> {
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
    > IntoSystem<'a, I::Input, (I, O, C)> for C
where
    I::State: Send + 'static,
{
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
    pub unsafe fn new<'a, T: Send + 'static>(
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
        let state = State(Arc::new(UnsafeCell::new(state)));
        Self {
            name,
            identifier,
            run: {
                let state = State(state.0.clone());
                Box::new(move |world| unsafe { run(state.get(), &*(world as *const _)) })
            },
            update: {
                let state = State(state.0.clone());
                Box::new(move |world| unsafe { update(state.get(), &mut *(world as *mut _)) })
            },
            resolve: {
                let state = State(state.0.clone());
                Box::new(move |world| unsafe { resolve(state.get(), &mut *(world as *mut _)) })
            },
            depend: {
                let state = State(state.0.clone());
                Box::new(move |world| unsafe { depend(state.get(), &*(world as *const _)) })
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
    use std::{mem::replace, sync::Mutex};

    use super::*;

    pub struct Runner {
        identifier: usize,
        version: usize,
        systems: Vec<System>,
        blocks: Vec<Block>,
        conflict: Conflict,
        result: Mutex<Result>,
    }

    #[derive(Default, Clone)]
    struct Block {
        runs: Vec<usize>,
        resolves: Vec<usize>,
        dependencies: Vec<Dependency>,
    }

    impl Runner {
        pub fn new(identifier: usize, systems: impl IntoIterator<Item = System>) -> Self {
            Self {
                identifier,
                version: 0,
                systems: systems.into_iter().collect(),
                blocks: Vec::new(),
                conflict: Conflict::default(),
                result: Mutex::new(Ok(())),
            }
        }

        #[inline]
        pub fn systems(&self) -> &[System] {
            &self.systems
        }

        pub fn update(&mut self, world: &mut World) -> Result {
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

        pub fn run(&mut self, world: &mut World) -> Result {
            self.update(world)?;

            for block in self.blocks.iter_mut() {
                // If world's version has changed, this may mean that the dependencies used to schedule the systems
                // are not up to date, therefore it is not safe to run the systems in parallel.
                if self.version == world.version() {
                    if block.runs.len() > 1 {
                        use rayon::prelude::*;
                        let result = &mut self.result;
                        let systems = self.systems.as_mut_ptr() as usize;
                        block.runs.par_iter().for_each(|&index| {
                            // SAFETY: The indices stored in 'runs' are guaranteed to be unique and ordered. See 'Runner::schedule'.
                            let system = unsafe { &mut *(systems as *mut System).add(index) };
                            if let Err(error) = (system.run)(world) {
                                if let Ok(mut guard) = result.lock() {
                                    *guard = Err(error);
                                }
                            }
                        });
                        result
                            .get_mut()
                            .map_or(Err(Error::MutexPoison), |result| replace(result, Ok(())))?;
                    } else {
                        for &index in block.runs.iter() {
                            (self.systems[index].run)(world)?;
                        }
                    }
                } else {
                    for &index in block.runs.iter() {
                        let system = &mut self.systems[index];
                        (system.update)(world)?;
                        (system.run)(world)?;
                    }
                }

                for &index in block.resolves.iter() {
                    (self.systems[index].resolve)(world)?;
                }
            }

            Ok(())
        }

        /// Batches the systems in blocks that can be executed in parallel using the dependencies produces by each system
        /// to ensure safety. The indices stored in the blocks are guaranteed to be unique and ordered within each block.
        /// Note that systems may be reordered if their dependencies allow it.
        fn schedule(&mut self, world: &mut World) -> Result {
            fn next(blocks: &mut [Block], conflict: &mut Conflict) -> Result {
                if let Some((head, rest)) = blocks.split_first_mut() {
                    conflict
                        .detect(Scope::Inner, &head.dependencies)
                        .map_err(Error::Depend)?;

                    for tail in rest.iter_mut() {
                        match conflict.detect(Scope::Outer, &tail.dependencies) {
                            Ok(_) => {
                                head.runs.append(&mut tail.runs);
                                head.dependencies.append(&mut tail.dependencies);
                            }
                            _ => {}
                        }
                    }

                    conflict.clear();
                    next(rest, conflict)?;

                    match rest {
                        [tail, ..] if tail.runs.len() == 0 && tail.dependencies.len() == 0 => {
                            head.resolves.append(&mut tail.resolves)
                        }
                        _ => {}
                    }
                }

                Ok(())
            }

            self.blocks.clear();
            self.conflict.clear();

            for (index, system) in self.systems.iter_mut().enumerate() {
                (system.update)(world)?;
                self.blocks.push(Block {
                    runs: vec![index],
                    resolves: vec![index],
                    dependencies: (system.depend)(world),
                });
            }

            next(&mut self.blocks, &mut self.conflict)?;
            self.blocks.retain(|block| {
                block.runs.len() > 0 || block.resolves.len() > 0 || block.dependencies.len() > 0
            });
            Ok(())
        }
    }
}

pub mod schedule {
    use super::*;

    pub struct Scheduler<'a> {
        pub(crate) prefix: String,
        pub(crate) systems: Vec<Result<System>>,
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

        pub fn run<I: Default, M, S: for<'a> IntoSystem<'a, I, M>>(&mut self, system: S) -> Result {
            self.scheduler().add(system).schedule()?.run(self)
        }

        pub fn run_with<I, M, S: for<'a> IntoSystem<'a, I, M>>(
            &mut self,
            input: I,
            system: S,
        ) -> Result {
            self.scheduler()
                .add_with(input, system)
                .schedule()?
                .run(self)
        }
    }

    impl<'a> Scheduler<'a> {
        pub fn pipe<F: FnOnce(Self) -> Self>(self, schedule: F) -> Self {
            self.with_prefix::<F, _>(schedule)
        }

        pub fn add<I: Default, M, S: IntoSystem<'a, I, M>>(self, system: S) -> Self {
            self.add_with(I::default(), system)
        }

        pub fn add_with<I, M, S: IntoSystem<'a, I, M>>(self, input: I, system: S) -> Self {
            self.with_prefix::<S, _>(|mut scheduler| {
                let system = system.system(input, scheduler.world).map(|mut system| {
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
                    |_, _| Ok(()),
                    |_, _| Ok(()),
                    |_, _| Ok(()),
                    |_, _| vec![Dependency::Unknown],
                )
            })
        }

        pub fn schedule(self) -> Result<Runner> {
            let mut systems = Vec::new();
            let mut errors = Vec::new();

            for system in self.systems {
                match system {
                    Ok(system) => systems.push(system),
                    Err(error) => errors.push(error),
                }
            }

            Error::All(errors)
                .flatten(true)
                .map_or(Ok(Runner::new(self.world.identifier(), systems)), Err)
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

pub mod output {
    use super::*;

    pub trait IntoOutput {
        fn output(self) -> Result;
    }

    impl IntoOutput for Error {
        #[inline]
        fn output(self) -> Result {
            Err(self)
        }
    }

    impl<T: IntoOutput> IntoOutput for Option<T> {
        #[inline]
        fn output(self) -> Result {
            self.map_or(Ok(()), IntoOutput::output)
        }
    }

    impl<T: IntoOutput> IntoOutput for Result<T> {
        #[inline]
        fn output(self) -> Result {
            self.and_then(IntoOutput::output)
        }
    }

    macro_rules! output {
        ($($p:ident, $t:ident),*) => {
            impl<'a, $($t: IntoOutput,)*> IntoOutput for ($($t,)*) {
                #[inline]
                fn output(self) -> Result {
                    let ($($p,)*) = self;
                    $($p.output()?;)*
                    Ok(())
                }
            }
        };
    }

    recurse!(output);
}

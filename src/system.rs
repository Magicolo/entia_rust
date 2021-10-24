use self::runner::*;
use crate::{
    depend::{Depend, Dependency},
    inject::{Get, Inject, InjectContext},
    world::World,
};
use entia_core::Call;
use std::{
    any::{type_name, TypeId},
    cell::UnsafeCell,
    fmt,
    mem::replace,
    mem::swap,
    sync::atomic::{AtomicUsize, Ordering},
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

#[derive(Debug)]
pub enum Error {
    WrongWorld,
    MissingSystem,
    FailedToInject,
    DependencyConflict(usize),
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

impl<'a, I: Inject, C: Call<I> + Call<<I::State as Get<'a>>::Item> + 'static>
    IntoSystem<I::Input, (I, C)> for C
{
    fn into_system(self, input: I::Input, world: &mut World) -> Result<System, Error> {
        let identifier = System::reserve();
        let state = I::initialize(input, InjectContext::new(identifier, world))
            .ok_or(Error::FailedToInject)?;
        let system = unsafe {
            System::new(
                Some(identifier),
                I::name(),
                (self, state, identifier),
                |(run, state, _), world| run.call(state.get(world)),
                |(_, state, identifier), world| {
                    I::update(state, InjectContext::new(*identifier, world))
                },
                |(_, state, identifier), world| {
                    I::resolve(state, InjectContext::new(*identifier, world))
                },
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

impl System {
    pub fn reserve() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub unsafe fn new<'a, T: 'static>(
        identifier: Option<usize>,
        name: String,
        state: T,
        run: fn(&'a mut T, &'a World),
        update: fn(&'a mut T, &'a mut World),
        resolve: fn(&'a mut T, &'a mut World),
        depend: fn(&'a mut T, &'a World) -> Vec<Dependency>,
    ) -> Self {
        // SAFETY: Since this crate controls the execution of the system's functions, it can guarantee
        // that they are not run in parallel which would allow for races.

        // SAFETY: The 'new' function is declared as unsafe because the user must guarantee that no reference
        // to the 'World' outlives the call of the function pointers. Normally this could be enforced by Rust but
        // there seem to be a limitation in the expressivity of the type system to be able to express the desired
        // intention.

        let identifier = identifier.unwrap_or_else(Self::reserve);
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
                Box::new(move |world| unsafe { depend(&mut *state.get(), &*(world as *const _)) })
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
        f.debug_tuple(type_name::<Self>())
            .field(&self.name())
            .finish()
    }
}

pub mod runner {
    use std::collections::HashMap;

    use entia_core::bits::Bits;

    use crate::depend::Scope;

    use super::*;

    pub struct Runner {
        pub(crate) identifier: usize,
        pub(crate) blocks: Vec<Block>,
        pub(crate) segments: usize,
    }

    #[derive(Default)]
    pub struct Block {
        systems: Vec<System>,
        dependencies: Vec<Dependency>,
    }

    unsafe impl Send for System {}

    impl Runner {
        pub fn new(
            systems: impl IntoIterator<Item = System>,
            world: &mut World,
        ) -> Result<Self, Error> {
            Ok(Self {
                identifier: world.identifier,
                blocks: Self::schedule(systems.into_iter(), world)?,
                segments: world.segments.len(),
            })
        }

        pub fn run(mut self, world: &mut World) -> Result<Runner, Error> {
            if self.identifier != world.identifier {
                return Err(Error::WrongWorld);
            }

            if self.segments != world.segments.len() {
                self.blocks = Self::schedule(
                    self.blocks.into_iter().map(|block| block.systems).flatten(),
                    world,
                )?;
                self.segments = world.segments.len();
            }

            for block in self.blocks.iter_mut() {
                // If segments have been added to the world, this may mean that the dependencies used to schedule the systems
                // are not up to date, therefore it is not safe to run the systems in parallel.
                if self.segments == world.segments.len() && block.systems.len() > 1 {
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

                for system in block.systems.iter_mut() {
                    (system.resolve)(world);
                }
            }

            Ok(self)
        }

        pub(crate) fn schedule(
            systems: impl Iterator<Item = System>,
            world: &mut World,
        ) -> Result<Vec<Block>, Error> {
            #[derive(Debug)]
            enum Has {
                All,
                None,
                Indices(Bits),
            }

            #[derive(Debug, Default)]
            struct State {
                unknown: bool,
                reads: HashMap<TypeId, Has>,
                writes: HashMap<TypeId, Has>,
                defers: HashMap<TypeId, Has>,
            }

            impl Has {
                pub fn add(&mut self, index: usize) -> bool {
                    match self {
                        Self::All => false,
                        Self::None => {
                            *self = Has::Indices(Bits::new());
                            self.add(index)
                        }
                        Self::Indices(bits) => {
                            if bits.has(index) {
                                false
                            } else {
                                bits.set(index, true);
                                true
                            }
                        }
                    }
                }

                pub fn has(&self, index: usize) -> bool {
                    match self {
                        Self::All => true,
                        Self::None => false,
                        Self::Indices(bits) => bits.has(index),
                    }
                }
            }

            impl Default for Has {
                fn default() -> Self {
                    Self::None
                }
            }

            impl State {
                pub fn inner_conflicts(&mut self, dependencies: &Vec<Dependency>) -> bool {
                    dependencies
                        .iter()
                        .any(|dependency| self.inner_conflict(None, dependency.clone()))
                }

                pub fn outer_conflicts(&mut self, dependencies: &Vec<Dependency>) -> bool {
                    dependencies
                        .iter()
                        .any(|dependency| self.outer_conflict(None, dependency.clone()))
                }

                pub fn clear(&mut self) {
                    self.unknown = false;
                    self.reads.clear();
                    self.writes.clear();
                    self.defers.clear();
                }

                fn inner_conflict(&mut self, index: Option<usize>, dependency: Dependency) -> bool {
                    match (index, dependency) {
                        (_, Dependency::Unknown) => {
                            self.unknown = true;
                            false
                        }
                        (_, Dependency::At(index, dependency)) => {
                            self.inner_conflict(Some(index), *dependency)
                        }
                        (_, Dependency::Ignore(Scope::All, _)) => false,
                        (_, Dependency::Ignore(Scope::Inner, _)) => false,
                        (index, Dependency::Ignore(Scope::Outer, dependency)) => {
                            self.inner_conflict(index, *dependency)
                        }
                        (Some(index), Dependency::Read(identifier)) => {
                            if has(&self.writes, identifier, index) {
                                true
                            } else {
                                add(&mut self.reads, identifier, index);
                                false
                            }
                        }
                        (Some(index), Dependency::Write(identifier)) => {
                            if has(&self.reads, identifier, index)
                                || has(&self.writes, identifier, index)
                            {
                                return true;
                            } else {
                                add(&mut self.writes, identifier, index);
                                false
                            }
                        }
                        (Some(index), Dependency::Defer(identifier)) => {
                            add(&mut self.defers, identifier, index);
                            false
                        }
                        (None, Dependency::Read(identifier)) => {
                            if has_any(&self.writes, identifier) {
                                true
                            } else {
                                add_all(&mut self.reads, identifier);
                                false
                            }
                        }
                        (None, Dependency::Write(identifier)) => {
                            if has_any(&self.reads, identifier) || has_any(&self.writes, identifier)
                            {
                                return true;
                            } else {
                                add_all(&mut self.writes, identifier);
                                false
                            }
                        }
                        (None, Dependency::Defer(identifier)) => {
                            add_all(&mut self.defers, identifier);
                            false
                        }
                    }
                }

                fn outer_conflict(&mut self, index: Option<usize>, dependency: Dependency) -> bool {
                    match (index, dependency) {
                        (_, Dependency::Unknown) => true,
                        (_, Dependency::At(index, dependency)) => {
                            self.outer_conflict(Some(index), *dependency)
                        }
                        (_, Dependency::Ignore(Scope::All, _)) => false,
                        (_, Dependency::Ignore(Scope::Outer, _)) => false,
                        (index, Dependency::Ignore(Scope::Inner, dependency)) => {
                            self.inner_conflict(index, *dependency)
                        }
                        (Some(index), Dependency::Read(identifier)) => {
                            if has(&self.writes, identifier, index)
                                || has(&self.defers, identifier, index)
                            {
                                true
                            } else {
                                add(&mut self.reads, identifier, index);
                                false
                            }
                        }
                        (Some(index), Dependency::Write(identifier)) => {
                            if has(&self.reads, identifier, index)
                                || has(&self.writes, identifier, index)
                                || has(&self.defers, identifier, index)
                            {
                                true
                            } else {
                                add(&mut self.writes, identifier, index);
                                false
                            }
                        }
                        (Some(index), Dependency::Defer(identifier)) => {
                            add(&mut self.defers, identifier, index);
                            false
                        }
                        (None, Dependency::Read(identifier)) => {
                            if has_any(&self.writes, identifier)
                                || has_any(&self.defers, identifier)
                            {
                                true
                            } else {
                                add_all(&mut self.reads, identifier);
                                false
                            }
                        }
                        (None, Dependency::Write(identifier)) => {
                            if has_any(&self.reads, identifier)
                                || has_any(&self.writes, identifier)
                                || has_any(&self.defers, identifier)
                            {
                                true
                            } else {
                                add_all(&mut self.writes, identifier);
                                false
                            }
                        }
                        (None, Dependency::Defer(identifier)) => {
                            add_all(&mut self.defers, identifier);
                            false
                        }
                    }
                }
            }

            fn add(map: &mut HashMap<TypeId, Has>, identifier: TypeId, index: usize) -> bool {
                map.entry(identifier).or_default().add(index)
            }

            fn add_all(map: &mut HashMap<TypeId, Has>, identifier: TypeId) {
                *map.entry(identifier).or_default() = Has::All;
            }

            fn has(map: &HashMap<TypeId, Has>, identifier: TypeId, index: usize) -> bool {
                map.get(&identifier)
                    .map(|has| has.has(index))
                    .unwrap_or(false)
            }

            fn has_any(map: &HashMap<TypeId, Has>, identifier: TypeId) -> bool {
                has(map, identifier, usize::MAX)
            }

            let mut blocks = Vec::new();
            let mut block = Block::default();
            let mut inner = State::default();
            let mut outer = State::default();

            for mut system in systems {
                (system.update)(world);

                let mut dependencies = (system.depend)(world);
                if inner.inner_conflicts(&dependencies) {
                    // TODO: Add more details to the error.
                    // TODO: Rather than returning early, continue collecting errors.
                    return Err(Error::DependencyConflict(system.identifier()));
                } else if outer.outer_conflicts(&dependencies) {
                    // TODO: When 'outer_conflicts' are detected, can later systems be still included in the block if they do not
                    // have 'outer_conflicts'? Dependencies would need to be accumulated even for conflicting systems and a system
                    // that has a 'Dependency::Unknown' should never be crossed.
                    if block.systems.len() > 0 {
                        blocks.push(replace(&mut block, Block::default()));
                    }
                    swap(&mut inner, &mut outer);
                }

                block.systems.push(system);
                block.dependencies.append(&mut dependencies);
                inner.clear();
            }

            if block.systems.len() > 0 {
                blocks.push(block);
            }

            Ok(blocks)
        }
    }
}

pub mod schedule {
    use super::*;

    pub struct Scheduler<'a> {
        pub(crate) systems: Vec<Result<System, Error>>,
        pub(crate) world: &'a mut World,
    }

    impl World {
        pub fn scheduler(&mut self) -> Scheduler {
            Scheduler {
                systems: Vec::new(),
                world: self,
            }
        }

        pub fn run<I: Default, M, S: IntoSystem<I, M>>(&mut self, system: S) -> Result<(), Error> {
            self.run_with(I::default(), system)
        }

        pub fn run_with<I, M, S: IntoSystem<I, M>>(
            &mut self,
            input: I,
            system: S,
        ) -> Result<(), Error> {
            let runner = self.scheduler().schedule_with(input, system).runner()?;
            runner.run(self)?;
            Ok(())
        }
    }

    impl<'a> Scheduler<'a> {
        pub fn pipe(self, schedule: impl FnOnce(Self) -> Self) -> Self {
            schedule(self)
        }

        pub fn schedule<I: Default, M, S: IntoSystem<I, M>>(self, system: S) -> Self {
            self.schedule_with(I::default(), system)
        }

        pub fn schedule_with<I, M, S: IntoSystem<I, M>>(mut self, input: I, system: S) -> Self {
            let system = system.into_system(input, self.world);
            self.systems.push(system);
            self
        }

        pub fn synchronize(self) -> Self {
            self.schedule(unsafe {
                System::new(
                    None,
                    "Synchronize".into(),
                    (),
                    |_, _| {},
                    |_, _| {},
                    |_, _| {},
                    |_, _| vec![Dependency::Unknown],
                )
            })
        }

        pub fn runner(self) -> Result<Runner, Error> {
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
                None => Runner::new(systems, self.world),
            }
        }
    }
}

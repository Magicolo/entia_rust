use crate::call::*;
use crate::inject::*;
use crate::world::*;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Dependency {
    Unknown,
    Read(usize, TypeId),
    Write(usize, TypeId),
}

pub struct Runner {
    identifier: usize,
    systems: Vec<Vec<System>>,
    segments: usize,
}

pub struct System {
    run: Box<dyn FnMut(&World)>,
    update: Box<dyn FnMut(&mut World)>,
    resolve: Box<dyn FnMut(&mut World)>,
    dependencies: Box<dyn FnMut(&World) -> Vec<Dependency>>,
}

pub struct Scheduler<'a> {
    pub(crate) schedules: Vec<Option<System>>,
    pub(crate) world: &'a mut World,
}

unsafe impl Send for System {}

impl Runner {
    pub fn run(&mut self, world: &mut World) {
        if self.identifier != world.identifier {
            panic!();
        }

        let count = world.segments.len();
        if count != self.segments {
            for systems in self.systems.iter_mut() {
                systems.iter_mut().for_each(|system| (system.update)(world));
            }
            self.systems = Self::schedule(self.systems.drain(..).flatten(), world).unwrap();
            self.segments = count;
        }

        let mut changed = false;
        for systems in self.systems.iter_mut() {
            if changed {
                for system in systems {
                    (system.update)(world);
                    (system.run)(world);
                    (system.resolve)(world);
                }
            } else if systems.len() == 1 {
                let system = &mut systems[0];
                (system.run)(world);
                (system.resolve)(world);
                changed |= count < world.segments.len();
            } else {
                use rayon::prelude::*;
                systems
                    .par_iter_mut()
                    .for_each(|system| (system.run)(world));
                systems
                    .iter_mut()
                    .for_each(|system| (system.resolve)(world));
                changed |= count < world.segments.len();
            }
        }
    }

    fn conflicts(
        dependencies: &Vec<Dependency>,
        reads: &mut HashSet<(usize, TypeId)>,
        writes: &mut HashSet<(usize, TypeId)>,
    ) -> bool {
        for dependency in dependencies {
            match dependency {
                Dependency::Unknown => return true,
                Dependency::Read(segment, store) => {
                    let pair = (*segment, *store);
                    if writes.contains(&pair) {
                        return true;
                    }
                    reads.insert(pair);
                }
                Dependency::Write(segment, store) => {
                    let pair = (*segment, *store);
                    if reads.contains(&pair) || writes.contains(&pair) {
                        return true;
                    }
                    writes.insert(pair);
                }
            }
        }
        false
    }

    fn schedule(systems: impl Iterator<Item = System>, world: &World) -> Option<Vec<Vec<System>>> {
        let mut pairs = Vec::new();
        let mut sequence = Vec::new();
        let mut parallel = Vec::new();
        let mut reads = HashSet::new();
        let mut writes = HashSet::new();

        for mut system in systems {
            let dependencies = (system.dependencies)(world);
            if Self::conflicts(&dependencies, &mut reads, &mut writes) {
                return None;
            }
            reads.clear();
            writes.clear();
            pairs.push((system, dependencies));
        }

        for (system, dependencies) in pairs {
            if Self::conflicts(&dependencies, &mut reads, &mut writes) {
                if parallel.len() > 0 {
                    sequence.push(std::mem::replace(&mut parallel, Vec::new()));
                }
                reads.clear();
                writes.clear();
            } else {
                parallel.push(system);
            }
        }

        if parallel.len() > 0 {
            sequence.push(parallel);
        }
        Some(sequence)
    }
}

impl System {
    #[inline]
    pub fn new<'a, T: 'static>(
        state: T,
        run: fn(&'a mut T, &World),
        update: fn(&'a mut T, &mut World),
        resolve: fn(&'a mut T, &mut World),
        dependencies: fn(&'a mut T, &World) -> Vec<Dependency>,
    ) -> Self {
        let state = Arc::new(UnsafeCell::new(state));
        // SAFETY: Since this crate controls the execution of the system's functions, it can guarantee
        // that they are not run in parallel which would allow for races.
        Self {
            run: {
                let state = state.clone();
                Box::new(move |world| run(unsafe { &mut *state.get() }, world))
            },
            update: {
                let state = state.clone();
                Box::new(move |world| update(unsafe { &mut *state.get() }, world))
            },
            resolve: {
                let state = state.clone();
                Box::new(move |world| resolve(unsafe { &mut *state.get() }, world))
            },
            dependencies: {
                let state = state.clone();
                Box::new(move |world| dependencies(unsafe { &mut *state.get() }, world))
            },
        }
    }
}

impl<'a> Scheduler<'a> {
    pub fn pipe<F: FnOnce(Self) -> Self>(self, pipe: F) -> Self {
        pipe(self)
    }

    pub fn system<I: Inject, C: Call<I, ()> + Call<<I::State as Get<'a>>::Item, ()> + 'static>(
        mut self,
        run: C,
    ) -> Self {
        self.schedules.push(I::initialize(self.world).map(|state| {
            System::new(
                (run, state),
                |(run, state), world| run.call(state.get(world)),
                |(_, state), world| I::update(state, world),
                |(_, state), world| I::resolve(state, world),
                |(_, state), world| I::dependencies(state, world),
            )
        }));
        self
    }

    pub fn synchronize(mut self) -> Self {
        self.schedules.push(Some(System::new(
            (),
            |_, _| {},
            |_, _| {},
            |_, _| {},
            |_, _| vec![Dependency::Unknown],
        )));
        self
    }

    pub fn schedule(self) -> Option<Runner> {
        let mut runs = Vec::new();
        for system in self.schedules {
            runs.push(system?);
        }
        // TODO: return a 'Result<Runner<'a>, Error>'
        Some(Runner {
            identifier: self.world.identifier,
            systems: Runner::schedule(runs.drain(..), self.world)?,
            segments: 0,
        })
    }
}

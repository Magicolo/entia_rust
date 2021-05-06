use crate::inject::*;
use crate::world::*;
use crate::*;
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

pub struct Runner<'a> {
    systems: Vec<Vec<System<'a>>>,
    segments: usize,
    world: &'a World,
}

pub struct System<'a> {
    run: Box<dyn Fn() + 'a>,
    update: Box<dyn FnMut() + 'a>,
    resolve: Box<dyn FnMut() + 'a>,
    dependencies: Box<dyn Fn() -> Vec<Dependency> + 'a>,
}

pub struct Scheduler<'a> {
    pub(crate) schedules: Vec<Option<System<'a>>>,
    pub(crate) world: &'a World,
}

unsafe impl Send for System<'_> {}

impl<'a> Runner<'a> {
    pub fn run(&mut self) {
        let count = self.world.0.segments.len();
        if count != self.segments {
            for systems in self.systems.iter_mut() {
                systems.iter_mut().for_each(|system| (system.update)());
            }
            self.systems = Self::schedule(self.systems.drain(..).flatten()).unwrap();
            self.segments = count;
        }

        let mut changed = false;
        for systems in self.systems.iter_mut() {
            if changed {
                for system in systems {
                    (system.update)();
                    (system.run)();
                    (system.resolve)();
                }
            } else if systems.len() == 1 {
                let system = &mut systems[0];
                (system.run)();
                (system.resolve)();
                changed |= count < self.world.0.segments.len();
            } else {
                use rayon::prelude::*;
                systems.par_iter_mut().for_each(|system| (system.run)());
                systems.iter_mut().for_each(|system| (system.resolve)());
                changed |= count < self.world.0.segments.len();
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

    fn schedule(systems: impl Iterator<Item = System<'a>>) -> Option<Vec<Vec<System<'a>>>> {
        let mut pairs = Vec::new();
        let mut sequence = Vec::new();
        let mut parallel = Vec::new();
        let mut reads = HashSet::new();
        let mut writes = HashSet::new();

        for system in systems {
            let dependencies = (system.dependencies)();
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

impl<'a> System<'a> {
    #[inline]
    pub fn new<T: 'a>(
        state: T,
        run: fn(&mut T),
        update: fn(&mut T),
        resolve: fn(&mut T),
        dependencies: fn(&mut T) -> Vec<Dependency>,
    ) -> Self {
        let state = Arc::new(UnsafeCell::new(state));
        // SAFETY: Since this crate controls the execution of the system's functions, it can guarantee
        // that they are not run in parallel which would allow for races.
        Self {
            run: {
                let state = state.clone();
                Box::new(move || run(unsafe { &mut *state.get() }))
            },
            update: {
                let state = state.clone();
                Box::new(move || update(unsafe { &mut *state.get() }))
            },
            resolve: {
                let state = state.clone();
                Box::new(move || resolve(unsafe { &mut *state.get() }))
            },
            dependencies: {
                let state = state.clone();
                Box::new(move || dependencies(unsafe { &mut *state.get() }))
            },
        }
    }
}

impl<'a> Scheduler<'a> {
    pub fn pipe<F: FnOnce(Self) -> Self>(self, pipe: F) -> Self {
        pipe(self)
    }

    pub fn system<I: Inject<'a>, O>(mut self, run: impl Call<I, O> + 'a) -> Self {
        self.schedules.push(I::initialize(self.world).map(|state| {
            System::new(
                (run, state),
                |(run, state)| {
                    run.call(I::inject(state));
                },
                |(_, state)| I::update(state),
                |(_, state)| I::resolve(state),
                |(_, state)| I::dependencies(state),
            )
        }));
        self
    }

    // pub fn system<I: Inject<'a>, R: FnMut(I) + 'a>(mut self, run: R) -> Self {
    //     self.schedules.push(I::initialize(self.world).map(|state| {
    //         System::new(
    //             (run, state),
    //             |(run, state)| run(I::inject(state)),
    //             |(_, state)| I::update(state),
    //             |(_, state)| I::resolve(state),
    //             |(_, state)| I::dependencies(state),
    //         )
    //     }));
    //     self
    // }

    pub fn synchronize(mut self) -> Self {
        self.schedules.push(Some(System::new(
            (),
            |_| {},
            |_| {},
            |_| {},
            |_| vec![Dependency::Unknown],
        )));
        self
    }

    pub fn schedule(self) -> Option<Runner<'a>> {
        let mut runs = Vec::new();
        for system in self.schedules {
            runs.push(system?);
        }
        // TODO: return a 'Result<Runner<'a>, Error>'
        Some(Runner {
            systems: Runner::schedule(runs.drain(..))?,
            segments: 0,
            world: self.world,
        })
    }
}

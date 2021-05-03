use crate::world::*;
use crate::*;
use std::any::Any;
use std::any::TypeId;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Dependency {
    Unknown,
    Read(usize, TypeId),
    Write(usize, TypeId),
}

pub struct Runner(Vec<Vec<System>>, usize);

impl Runner {
    pub fn run(&mut self, world: &mut World) {
        let count = world.segments.len();
        if count != self.1 {
            self.0 = Self::schedule(self.0.drain(..).flatten(), world).unwrap();
            self.1 = count;
        }

        let mut changed = false;
        for systems in self.0.iter_mut() {
            if changed {
                for system in systems {
                    (system.update)(&mut system.state, world);
                    (system.run)(&system.state, world);
                    (system.resolve)(&mut system.state, world);
                }
            } else if systems.len() == 1 {
                let system = &mut systems[0];
                (system.run)(&system.state, world);
                (system.resolve)(&mut system.state, world);
                changed |= count < world.segments.len();
            } else {
                use rayon::prelude::*;
                systems
                    .par_iter()
                    .for_each(|system| (system.run)(&system.state, world));
                systems
                    .iter_mut()
                    .for_each(|system| (system.resolve)(&mut system.state, world));
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

    fn schedule(
        systems: impl Iterator<Item = System>,
        world: &mut World,
    ) -> Option<Vec<Vec<System>>> {
        let mut pairs = Vec::new();
        let mut sequence = Vec::new();
        let mut parallel = Vec::new();
        let mut reads = HashSet::new();
        let mut writes = HashSet::new();

        for mut system in systems {
            let dependencies = (system.update)(&mut system.state, world);
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

pub struct System {
    state: Box<dyn Any>,
    update: fn(&mut dyn Any, &mut World) -> Vec<Dependency>,
    resolve: fn(&mut dyn Any, &mut World),
    run: fn(&dyn Any, &World),
}

pub trait IntoSystem<P> {
    fn system(self, world: &mut World) -> Option<System>;
}

#[derive(Default, Clone)]
pub struct Scheduler {
    schedules: Vec<Arc<dyn Fn(&mut World) -> Option<System>>>,
}

unsafe impl Sync for System {}

impl<P> IntoSystem<P> for System {
    fn system(self, _: &mut World) -> Option<System> {
        Some(self)
    }
}

impl<P, S: IntoSystem<S> + Clone> IntoSystem<P> for &S {
    fn system(self, world: &mut World) -> Option<System> {
        self.clone().system(world)
    }
}

impl<I: Inject + 'static, O, C: Call<I, O> + 'static> IntoSystem<(I, O)> for Arc<C> {
    fn system(self, world: &mut World) -> Option<System> {
        I::initialize(world).map(|state| System {
            state: Box::new((self, state)),
            update: |state, world| {
                let (_, state) = state.downcast_mut::<(Arc<C>, I::State)>().unwrap();
                I::update(state, world)
            },
            resolve: |state, world| {
                let (_, state) = state.downcast_mut::<(Arc<C>, I::State)>().unwrap();
                I::resolve(state, world)
            },
            run: |state, world| {
                // let (call, state) =
                //     unsafe { &*(state as *const dyn Any as *const (Arc<C>, I::State)) };
                let (call, state) = state.downcast_ref::<(Arc<C>, I::State)>().unwrap();
                call.call(I::inject(state, world));
            },
        })
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pipe<F: FnOnce(&Self) -> Self>(&self, pipe: F) -> Self {
        pipe(self)
    }

    pub fn system<P, S: 'static>(&self, system: S) -> Self
    where
        Arc<S>: IntoSystem<P>,
    {
        let mut scheduler = self.clone();
        let system = Arc::new(system);
        scheduler
            .schedules
            .push(Arc::new(move |world| system.clone().system(world)));
        scheduler
    }

    pub fn synchronize(&self) -> Self {
        let mut scheduler = self.clone();
        scheduler.schedules.push(Arc::new(|_| {
            Some(System {
                state: Box::new(()),
                update: |_, _| vec![Dependency::Unknown],
                run: |_, _| {},
                resolve: |_, _| {},
            })
        }));
        scheduler
    }

    pub fn schedule(&self, world: &mut World) -> Option<Runner> {
        let mut runs = Vec::new();
        for schedule in self.schedules.iter() {
            runs.push(schedule(world)?);
        }
        // TODO: return a 'Result<Runner<'a>, Error>'
        Some(Runner(Runner::schedule(runs.drain(..), world)?, 0))
    }
}

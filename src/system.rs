use crate::world::*;
use crate::*;
use std::any::Any;
use std::any::TypeId;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Dependency {
    Unknown,
    Read(usize, TypeId),
    Write(usize, TypeId),
}

pub struct Runner(Vec<Vec<Run>>);

impl Runner {
    pub fn run(&mut self, world: &mut World) {
        let count = world.segments.len();
        let mut changed = false;
        for runs in self.0.iter_mut() {
            if changed {
                for run in runs {
                    (run.update)(&mut run.state, world);
                    (run.run)(&run.state, world);
                    (run.resolve)(&mut run.state, world);
                }
            } else if runs.len() == 1 {
                let run = &mut runs[0];
                (run.run)(&run.state, world);
                (run.resolve)(&mut run.state, world);
                changed |= count < world.segments.len();
            } else {
                use rayon::prelude::*;
                runs.par_iter().for_each(|run| (run.run)(&run.state, world));
                runs.iter_mut()
                    .for_each(|run| (run.resolve)(&mut run.state, world));
                changed |= count < world.segments.len();
            }
        }

        if changed {
            self.0 = Self::schedule(self.0.drain(..).flatten(), world).unwrap();
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

    fn schedule(runs: impl Iterator<Item = Run>, world: &mut World) -> Option<Vec<Vec<Run>>> {
        let mut pairs = Vec::new();
        let mut sequence = Vec::new();
        let mut parallel = Vec::new();
        let mut reads = HashSet::new();
        let mut writes = HashSet::new();

        for mut run in runs {
            let dependencies = (run.update)(&mut run.state, world);
            if Self::conflicts(&dependencies, &mut reads, &mut writes) {
                return None;
            }
            reads.clear();
            writes.clear();
            pairs.push((run, dependencies));
        }

        for (run, dependencies) in pairs {
            if Self::conflicts(&dependencies, &mut reads, &mut writes) {
                if parallel.len() > 0 {
                    sequence.push(std::mem::replace(&mut parallel, Vec::new()));
                }
                reads.clear();
                writes.clear();
            } else {
                parallel.push(run);
            }
        }

        if parallel.len() > 0 {
            sequence.push(parallel);
        }
        Some(sequence)
    }
}

pub struct Run {
    state: Box<dyn Any>,
    update: fn(&mut dyn Any, &mut World) -> Vec<Dependency>,
    resolve: fn(&mut dyn Any, &mut World),
    run: fn(&dyn Any, &World),
}

#[derive(Default, Clone)]
pub struct Scheduler {
    schedules: Vec<Arc<dyn Fn(&mut World) -> Option<Run>>>,
}

pub trait System<S> {
    fn initialize(world: &mut World) -> Option<S>;
    fn update(state: &mut S, world: &mut World) -> Vec<Dependency>;
    fn resolve(state: &S, world: &mut World);
    fn run(&self, state: &S, world: &World);
}

type State<T, P> = (T, PhantomData<&'static P>);
impl<I: Inject, O, C: Call<I, O>> System<State<I::State, (I, O)>> for C {
    fn initialize(world: &mut World) -> Option<State<I::State, (I, O)>> {
        Some((I::initialize(world)?, PhantomData))
    }

    fn update((state, _): &mut State<I::State, (I, O)>, world: &mut World) -> Vec<Dependency> {
        I::update(state, world)
    }

    fn resolve((inject, _): &State<I::State, (I, O)>, world: &mut World) {
        I::resolve(inject, world);
    }

    fn run(&self, (state, _): &State<I::State, (I, O)>, world: &World) {
        self.call(I::get(state, world));
    }
}

unsafe impl Sync for Run {}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pipe<F: FnOnce(&Self) -> Self>(&self, pipe: F) -> Self {
        pipe(self)
    }

    pub fn system<T: Send + 'static, S: System<T> + Sync + Send + 'static>(
        &self,
        system: S,
    ) -> Self {
        let mut scheduler = self.clone();
        let system = Arc::new(system);
        scheduler.schedules.push(Arc::new(move |world| {
            S::initialize(world).map(|state| Run {
                state: Box::new((system.clone(), state)),
                update: |state, world| {
                    let (_, state) = state.downcast_mut::<(Arc<S>, T)>().unwrap();
                    S::update(state, world)
                },
                resolve: |state, world| {
                    let (_, state) = state.downcast_mut::<(Arc<S>, T)>().unwrap();
                    S::resolve(state, world)
                },
                run: |state, world| {
                    let (system, state) = state.downcast_ref::<(Arc<S>, T)>().unwrap();
                    system.run(state, world)
                },
            })
        }));
        scheduler
    }

    pub fn synchronize(&self) -> Self {
        let mut scheduler = self.clone();
        scheduler.schedules.push(Arc::new(|_| {
            Some(Run {
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
        Some(Runner(Runner::schedule(runs.drain(..), world)?))
    }
}

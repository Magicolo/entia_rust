use crate::internal::*;
use crate::world::*;
use crate::*;
use std::any::type_name;
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

pub struct Runner<'a>(Box<dyn FnMut() + 'a>);
pub struct Run<'a> {
    _name: String,
    update: Box<dyn FnMut() -> Vec<Dependency> + 'a>,
    resolve: Box<dyn Fn() + 'a>,
    run: Box<dyn Fn() + Sync + 'a>,
}

#[derive(Default, Clone)]
pub struct Scheduler {
    schedules: Vec<Arc<dyn Fn(&World) -> Option<Run>>>,
}

pub trait System<S> {
    fn name() -> String {
        type_name::<Self>().into()
    }
    fn initialize(world: &World) -> Option<S>;
    fn update(state: &mut S) -> Vec<Dependency>;
    fn resolve(state: &S);
    fn run(&self, state: &S);
}

type State<T, P> = (T, PhantomData<&'static P>);
impl<I: Inject, O, C: Call<I, O>> System<State<I::State, (I, O)>> for C {
    fn initialize(world: &World) -> Option<State<I::State, (I, O)>> {
        Some((I::initialize(world)?, PhantomData))
    }

    fn update((state, _): &mut State<I::State, (I, O)>) -> Vec<Dependency> {
        I::update(state)
    }

    fn resolve((inject, _): &State<I::State, (I, O)>) {
        I::resolve(inject);
    }

    fn run(&self, (state, _): &State<I::State, (I, O)>) {
        self.call(I::get(state));
    }
}

unsafe impl<'a> Sync for Run<'a> {}
impl<'a> Runner<'a> {
    #[inline]
    pub fn run(&mut self) {
        self.0()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pipe<F: FnOnce(&Self) -> Self>(&self, pipe: F) -> Self {
        pipe(self)
    }

    pub fn system<S: System<impl Send + 'static> + Sync + Send + 'static>(
        &self,
        system: S,
    ) -> Self {
        let mut scheduler = self.clone();
        let system = Arc::new(system);
        scheduler.schedules.push(Arc::new(move |world| {
            let system = system.clone();
            let state = Arc::new(Wrap::new(S::initialize(&world)?));
            Some(Run {
                _name: S::name(),
                update: {
                    let state = state.clone();
                    Box::new(move || S::update(unsafe { &mut *state.0.get() }))
                },
                resolve: {
                    let state = state.clone();
                    Box::new(move || {
                        S::resolve(unsafe { &*state.0.get() });
                    })
                },
                run: {
                    let state = state.clone();
                    Box::new(move || system.run(unsafe { &*state.0.get() }))
                },
            })
        }));
        scheduler
    }

    pub fn synchronize(&self) -> Self {
        let mut scheduler = self.clone();
        scheduler.schedules.push(Arc::new(|_| {
            Some(Run {
                _name: "synchronize".into(),
                update: Box::new(|| vec![Dependency::Unknown]),
                run: Box::new(|| {}),
                resolve: Box::new(|| {}),
            })
        }));
        scheduler
    }

    pub fn schedule<'a>(&self, world: &'a World) -> Option<Runner<'a>> {
        fn conflicts<'a>(
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

        fn schedule<'a>(runs: impl Iterator<Item = Run<'a>>) -> Option<Vec<Vec<Run<'a>>>> {
            let mut pairs = Vec::new();
            let mut sequence = Vec::new();
            let mut parallel = Vec::new();
            let mut reads = HashSet::new();
            let mut writes = HashSet::new();

            for mut run in runs {
                let dependencies = (run.update)();
                if conflicts(&dependencies, &mut reads, &mut writes) {
                    return None;
                }
                reads.clear();
                writes.clear();
                pairs.push((run, dependencies));
            }

            for (run, dependencies) in pairs {
                if conflicts(&dependencies, &mut reads, &mut writes) {
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

        let mut runs = Vec::new();
        for schedule in self.schedules.iter() {
            runs.push(schedule(world)?);
        }
        let mut sequence = schedule(runs.drain(..))?;
        // TODO: return a 'Result<Runner<'a>, Error>'
        Some(Runner(Box::new(move || {
            let count = world.inner.segments.len();
            let mut changed = false;
            for runs in sequence.iter_mut() {
                if changed {
                    for run in runs {
                        (run.update)();
                        (run.run)();
                        (run.resolve)();
                    }
                } else if runs.len() == 1 {
                    let run = &runs[0];
                    (run.run)();
                    (run.resolve)();
                    changed |= count < world.inner.segments.len();
                } else {
                    use rayon::prelude::*;
                    runs.par_iter().for_each(|run| (run.run)());
                    runs.iter_mut().for_each(|run| (run.resolve)());
                    changed |= count < world.inner.segments.len();
                }
            }

            if changed {
                sequence = schedule(sequence.drain(..).flatten()).unwrap();
            }
        })))
    }
}

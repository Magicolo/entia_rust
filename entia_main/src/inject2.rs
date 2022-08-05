use entia_core::Change;

use crate::{
    depend::{Conflict, Dependency, Scope},
    error::{Error, Result},
    inject::Get,
    world::World,
};
use std::{iter::once, marker::PhantomData, sync::Arc};

pub struct Runner {
    schedules: Vec<Schedule>,
    runs: Vec<Run2>,
    indices: Vec<usize>,
}

pub unsafe trait Inject2 {
    type Input;
    type State: for<'a> Get<'a> + 'static;

    fn initialize(input: Self::Input, world: &mut World) -> Result<Self::State>;
    fn schedule(_: &mut Self::State, _: &mut World) -> Vec<Run2<Self::State>> {
        vec![]
    }
    fn depend(state: &Self::State) -> Vec<Dependency>;
}

pub struct Schedule {
    schedule: Box<dyn FnMut(&mut World) -> Vec<Run2>>,
}

pub struct Scheduler<'a> {
    results: Vec<Result<Schedule>>,
    world: &'a mut World,
}

pub struct Run2<T = ()> {
    run: Box<dyn FnMut(&mut T) -> Result>,
    dependencies: Vec<Dependency>,
    _marker: PhantomData<T>,
}

pub struct Injector2<I: Inject2> {
    identifier: usize,
    name: String,
    world: usize,
    version: usize,
    state: Arc<I::State>,
    runs: Vec<Run2>,
    dependencies: Vec<Dependency>,
    _marker: PhantomData<I>,
}

impl Schedule {
    pub fn new(schedule: impl FnMut(&mut World) -> Vec<Run2> + 'static) -> Self {
        Self {
            schedule: Box::new(schedule),
        }
    }

    pub fn new_with<T: Clone + Send + Sync + 'static>(
        state: T,
        mut schedule: impl FnMut(T, &mut World) -> Vec<Run2<T>> + 'static,
    ) -> Self {
        Self {
            schedule: Box::new(move |world| {
                schedule(state.clone(), world)
                    .into_iter()
                    .map(|mut run| {
                        Run2::new(
                            {
                                let mut state = state.clone();
                                move |_| (run.run)(&mut state)
                            },
                            run.dependencies,
                        )
                    })
                    .collect()
            }),
        }
    }
}

impl<T: 'static> Run2<T> {
    pub fn new(
        mut run: impl FnMut(&mut T) -> Result + 'static,
        dependencies: Vec<Dependency>,
    ) -> Self {
        Self {
            run: Box::new(move |state| run(state)),
            dependencies,
            _marker: PhantomData,
        }
    }
}

impl Run2 {
    #[inline]
    pub fn run(&mut self) -> Result {
        (self.run)(&mut ())
    }
}

impl<I: Inject2> Injector2<I> {
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn identifier(&self) -> usize {
        self.identifier
    }

    #[inline]
    pub fn version(&self) -> usize {
        self.version
    }

    pub fn update(&mut self, world: &mut World) -> Result<bool> {
        if self.world != world.identifier() {
            return Err(Error::WrongWorld);
        } else if self.version == world.version() {
            return Ok(false);
        }

        let mut conflict = Conflict::default();
        let mut version = self.version;
        // 'I::schedule' may cause more changes of the 'world.version()'. Loop until the version has stabilized.
        for _ in 0..1_000 {
            if version.change(world.version()) {
                self.runs = schedule::<I>(&self.state, world);
                for run in self.runs.iter() {
                    conflict
                        .detect(Scope::Inner, &run.dependencies)
                        .map_err(Error::Depend)?;
                    conflict.clear();
                }
            } else {
                break;
            }
        }

        if version.change(world.version()) {
            return Err(Error::UnstableWorldVersion);
        }

        self.dependencies = I::depend(&self.state);
        conflict
            .detect(Scope::Inner, &self.dependencies)
            .map_err(Error::Depend)?;

        // Only commit the new version if scheduling and dependency analysis succeed.
        self.version = version;
        Ok(true)
    }

    pub fn run<T, R: FnOnce(<I::State as Get<'_>>::Item) -> T>(
        &mut self,
        world: &mut World,
        run: R,
    ) -> Result<T> {
        self.update(world)?;
        let value = run(unsafe { cast(&self.state).get() });
        for run in self.runs.iter_mut() {
            run.run()?;
        }
        Ok(value)
    }
}

impl<'a> Scheduler<'a> {
    pub fn add<I: Inject2, R: FnMut(I) -> Result + Send + Sync + 'static>(
        mut self,
        input: I::Input,
        run: R,
    ) -> Self
    where
        I::State: Get<'a, Item = I> + Send + Sync,
    {
        self.results.push(match I::initialize(input, self.world) {
            Ok(state) => {
                let state = Arc::new(state);
                let run = Arc::new(run);
                Ok(Schedule::new(move |world| {
                    let outer = cast(&state);
                    once(Run2::new(
                        {
                            let state = state.clone();
                            let run = run.clone();
                            move |_| {
                                let state = cast(&state);
                                let run = cast(&run);
                                run(unsafe { state.get() })
                            }
                        },
                        I::depend(outer),
                    ))
                    .chain(schedule::<I>(&state, world))
                    .collect()
                }))
            }
            Err(error) => Err(error),
        });
        // It is assumed that every 'initialize' modifies the world.
        self.world.modify();
        self
    }

    pub fn schedule(self) -> Result<Runner> {
        let mut schedules = Vec::new();
        let mut errors = Vec::new();

        for schedule in self.results {
            match schedule {
                Ok(system) => schedules.push(system),
                Err(error) => errors.push(error),
            }
        }

        match Error::All(errors).flatten(true) {
            Some(error) => Err(error),
            None => Ok(Runner {
                schedules,
                runs: vec![],
                indices: vec![],
            }),
        }
    }
}

fn schedule<I: Inject2>(state: &Arc<I::State>, world: &mut World) -> Vec<Run2> {
    I::schedule(cast(state), world)
        .into_iter()
        .map(|run| resolve(state.clone(), run))
        .collect()
}

fn resolve<S: 'static>(state: Arc<S>, mut run: Run2<S>) -> Run2 {
    Run2 {
        run: Box::new(move |_| (run.run)(cast(&state))),
        dependencies: run.dependencies,
        _marker: PhantomData,
    }
}

#[inline]
fn cast<'a, T>(state: &Arc<T>) -> &'a mut T {
    unsafe { &mut *(Arc::as_ptr(&state) as *mut T) }
}

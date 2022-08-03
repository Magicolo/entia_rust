use crate::{
    depend::Dependency,
    error::{Error, Result},
    inject::Get,
    world::World,
};
use std::{iter::once, marker::PhantomData, sync::Arc};

pub struct Schedule {
    schedule: Box<dyn FnMut(&mut World) -> Vec<Run2>>,
}

pub struct Scheduler<'a> {
    results: Vec<Result<Schedule>>,
    world: &'a mut World,
}

pub struct Run2<T = ()> {
    run: Box<dyn FnMut(&mut T, &World) -> Result>,
    dependencies: Vec<Dependency>,
    _marker: PhantomData<T>,
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
                                move |_, world| (run.run)(&mut state, world)
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
        mut run: impl FnMut(&mut T, &World) -> Result + 'static,
        dependencies: Vec<Dependency>,
    ) -> Self {
        Self {
            run: Box::new(move |state, world| run(state, world)),
            dependencies,
            _marker: PhantomData,
        }
    }

    pub(crate) fn adapt<U>(mut self, mut adapt: impl FnMut(&mut U) -> &mut T + 'static) -> Run2<U> {
        Run2 {
            run: Box::new(move |state, world| (self.run)(adapt(state), world)),
            dependencies: self.dependencies,
            _marker: PhantomData,
        }
    }
}

pub struct Runner {
    schedules: Vec<Schedule>,
    runs: Vec<Run2>,
    indices: Vec<usize>,
}

pub unsafe trait Inject2 {
    type Input;
    type State: for<'a> Get<'a> + 'static;

    fn initialize(input: Self::Input, world: &mut World) -> Result<Self::State>;
    fn depend(state: &Self::State, world: &World) -> Vec<Dependency>;
    fn schedule(_: &mut Self::State, _: &mut World) -> Vec<Run2<Self::State>> {
        vec![]
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
        #[inline]
        fn cast<'a, T>(state: &Arc<T>) -> &'a mut T {
            unsafe { &mut *(Arc::as_ptr(&state) as *mut T) }
        }

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
                            move |_, world| {
                                let state = cast(&state);
                                let run = cast(&run);
                                let world = unsafe { &*(world as *const World) };
                                run(unsafe { state.get(world) })
                            }
                        },
                        I::depend(outer, world),
                    ))
                    .chain(I::schedule(outer, world).into_iter().map(|run| {
                        let state = state.clone();
                        run.adapt(move |_| cast(&state))
                    }))
                    .collect()
                }))
            }
            Err(error) => Err(error),
        });
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

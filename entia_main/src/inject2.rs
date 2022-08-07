use entia_core::Change;
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::{
    depend::{Conflict, Dependency, Scope},
    error::{Error, Result},
    identify,
    inject::Get,
    world::World,
};
use std::{
    collections::HashSet,
    iter::once,
    marker::PhantomData,
    ops::Not,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::yield_now,
};

pub struct State {
    done: bool,
    error: Option<Error>,
    blockers: Vec<(usize, Error)>,
}

pub struct Runner {
    world: usize,
    version: usize,
    schedules: Vec<Schedule>,
    control: bool,
    index: AtomicUsize,
    success: AtomicBool,
    runs: Vec<Mutex<(Run2, State)>>,
    conflict: Conflict,
    pool: ThreadPool,
}

pub unsafe trait Inject2 {
    type Input;
    type State: for<'a> Get<'a> + 'static;

    // Is there a way to remove the 'identifier'? Only 'Defer' uses it...
    fn initialize(input: Self::Input, identifier: usize, world: &mut World) -> Result<Self::State>;
    fn schedule(_: &mut Self::State, _: &mut World) -> Vec<Run2<Self::State>> {
        vec![]
    }
    fn depend(state: &Self::State) -> Vec<Dependency>;
}

pub struct Schedule {
    identifier: usize,
    schedule: Box<dyn FnMut(&mut World) -> Vec<Run2>>,
}

pub struct Scheduler<'a> {
    results: Vec<Result<Schedule>>,
    world: &'a mut World,
}

pub struct Run2<T = ()> {
    run: Box<dyn FnMut(&mut T) -> Result + Send + Sync>,
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
    pub fn new(identifier: usize, schedule: impl FnMut(&mut World) -> Vec<Run2> + 'static) -> Self {
        Self {
            identifier,
            schedule: Box::new(schedule),
        }
    }

    pub const fn identifier(&self) -> usize {
        self.identifier
    }

    pub fn schedule(&mut self, world: &mut World) -> Vec<Run2> {
        (self.schedule)(world)
    }
}

impl<T: 'static> Run2<T> {
    pub fn new(
        mut run: impl FnMut(&mut T) -> Result + Send + Sync + 'static,
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

impl<'a> Scheduler<'a> {
    pub fn add<I: Inject2, R: FnMut(I) -> Result + Send + Sync + 'static>(
        mut self,
        input: I::Input,
        run: R,
    ) -> Self
    where
        I::State: Get<'a, Item = I> + Send + Sync,
    {
        let identifier = identify();
        self.results
            .push(match I::initialize(input, identifier, self.world) {
                Ok(state) => {
                    let state = Arc::new(state);
                    let run = Arc::new(run);
                    Ok(Schedule::new(identifier, move |world| {
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
                world: self.world.identifier(),
                version: 0,
                schedules,
                control: false,
                index: 0.into(),
                success: AtomicBool::new(true),
                runs: vec![],
                conflict: Conflict::default(),
                pool: ThreadPoolBuilder::new()
                    .build()
                    .map_err(|_| Error::FailedToSchedule)?,
            }),
        }
    }
}

impl<I: Inject2> Injector2<I>
where
    I::State: Send + Sync,
{
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn identifier(&self) -> usize {
        self.identifier
    }

    pub fn version(&self) -> usize {
        self.version
    }

    pub fn update(&mut self, world: &mut World) -> Result<bool> {
        if self.world != world.identifier() {
            return Err(Error::WrongWorld {
                expected: self.world,
                actual: world.identifier(),
            });
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
                        .detect(Scope::Inner, &run.dependencies, true)
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
            .detect(Scope::Inner, &self.dependencies, true)
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

impl Runner {
    pub const fn version(&self) -> usize {
        self.version
    }

    pub fn update(&mut self, world: &mut World) -> Result<bool> {
        let success = self.success.get_mut();
        if self.world != world.identifier() {
            return Err(Error::WrongWorld {
                expected: self.world,
                actual: world.identifier(),
            });
        } else if *success && self.version == world.version() {
            return Ok(false);
        }

        let mut version = self.version;
        // 'I::schedule' may cause more changes of the 'world.version()'. Loop until the version has stabilized.
        for _ in 0..1_000 {
            if success.change(true) | version.change(world.version()) {
                self.runs = self
                    .schedules
                    .iter_mut()
                    .flat_map(|schedule| schedule.schedule(world))
                    .map(|run| {
                        Mutex::new((
                            run,
                            State {
                                done: self.control,
                                error: None,
                                blockers: vec![],
                            },
                        ))
                    })
                    .collect();
            } else {
                break;
            }
        }

        if version.change(world.version()) {
            return Err(Error::UnstableWorldVersion);
        }

        self.schedule()?;

        // Only commit the new version if scheduling and dependency analysis succeed.
        self.version = version;
        Ok(true)
    }

    pub fn run(&mut self, world: &mut World) -> Result {
        self.update(world)?;

        let Self {
            success,
            control,
            index,
            runs,
            pool,
            ..
        } = self;

        *control = control.not();
        *index.get_mut() = 0;

        let control = *control;
        pool.scope(|scope| {
            for _ in 0..pool.current_num_threads() {
                scope.spawn(|_| {
                    success.fetch_and(Self::progress(index, runs, control), Ordering::Relaxed);
                });
            }
        });

        if *success.get_mut() {
            Ok(())
        } else {
            Error::all(runs.iter_mut().filter_map(|run| match run.get_mut() {
                Ok((_, state)) => state.error.take(),
                Err(_) => Some(Error::MutexPoison),
            }))
            .flatten(true)
            .map_or(Err(Error::FailedToRun), Err)
        }
    }

    /// The synchronisation mechanism is in 2 parts:
    /// 1. An `AtomicUsize` is used to reserve an index in the `runs` vector. It ensures that each run is executed only once.
    /// 2. A `Mutex` around the run and its state that will force the blocked threads to wait until this run is done. This choice
    /// of synchronisation prevents many sneaky interleavings of threads and is very straightfoward to implement.
    /// - This mechanism can not produce a dead lock as long as the `blockers` are all indices `< index` (which they are by design).
    /// Since runs are executed in order, for any index that is reserved, all indices smaller than that index represent a run that
    /// is done or in progress (not idle) which is important to prevent a spin loop when waiting for `blockers` to finish.
    fn progress(index: &AtomicUsize, runs: &Vec<Mutex<(Run2, State)>>, control: bool) -> bool {
        loop {
            let index = index.fetch_add(1, Ordering::Relaxed);
            let mut guard = match runs.get(index) {
                Some(run) => match run.lock() {
                    Ok(guard) => guard,
                    Err(_) => return false,
                },
                None => return true,
            };

            let mut done = 0;
            while let Some(&(blocker, _)) = guard.1.blockers.get(done) {
                // Sanity check. If this is not the case, this thread might spin loop and consume much CPU.
                debug_assert!(blocker < index);

                match runs[blocker].lock() {
                    Ok(guard) if guard.1.done == control => done += 1,
                    Ok(guard) if guard.1.error.is_some() => return false,
                    Ok(guard) => {
                        // When the lock is taken, it is expected that `done == control` except if a blocker thread paused
                        // after `index.fetch_add` and before `run.lock`. Since this should happen very rarely and is a very transient
                        // state, `yield_now` is used to give the other thread the chance to acquire the lock.
                        // - Drop the guard before yielding to allow the blocker thread to acquire the lock with fewer context switches.
                        drop(guard);
                        yield_now();
                    }
                    Err(_) => return false,
                }
            }

            match guard.0.run() {
                Ok(_) => guard.1.done = control,
                Err(error) => {
                    guard.1.error = Some(error);
                    return false;
                }
            }
        }
    }

    fn schedule(&mut self) -> Result {
        // TODO: This algorithm scales poorly (n^2 / 2, where n is the number of runs) and alot of the work is redundant
        // as shown by the second part. Make this better.
        let mut runs = &mut self.runs[..];
        while let Some((tail, rest)) = runs.split_last_mut() {
            let (run, state) = tail.get_mut().map_err(|_| Error::MutexPoison)?;
            self.conflict
                .detect(Scope::Inner, &run.dependencies, true)
                .map_err(Error::Depend)?;

            for (i, rest) in rest.iter_mut().enumerate() {
                let (previous, _) = rest.get_mut().map_err(|_| Error::MutexPoison)?;
                if let Err(error) =
                    self.conflict
                        .detect(Scope::Outer, &previous.dependencies, false)
                {
                    state.blockers.push((i, error.into()));
                }
            }

            self.conflict.clear();
            runs = rest;
        }

        // Remove redundant blockers.
        let mut runs = &mut self.runs[..];
        let mut set = HashSet::new();
        while let Some((tail, rest)) = runs.split_last_mut() {
            let (_, state) = tail.get_mut().map_err(|_| Error::MutexPoison)?;
            for &(blocker, _) in state.blockers.iter() {
                // `rest[blocker]` ensures that `blocker < rest.len()` which is important when running.
                let (_, previous) = rest[blocker].get_mut().map_err(|_| Error::MutexPoison)?;
                set.extend(previous.blockers.iter().map(|&(blocker, _)| blocker));
            }
            state.blockers.retain(|(blocker, _)| !set.contains(blocker));
            set.clear();
            runs = rest;
        }
        Ok(())
    }
}

fn schedule<I: Inject2>(state: &Arc<I::State>, world: &mut World) -> Vec<Run2>
where
    I::State: Send + Sync,
{
    I::schedule(cast(state), world)
        .into_iter()
        .map(|run| resolve(state.clone(), run))
        .collect()
}

fn resolve<S: Send + Sync + 'static>(state: Arc<S>, mut run: Run2<S>) -> Run2 {
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

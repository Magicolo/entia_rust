use crate::{
    depend::{Conflict, Dependency, Scope},
    error::{Error, Result},
    system::System,
    world::World,
    IntoSystem,
};
use entia_core::Change;
use parking_lot::Mutex;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    any::Any,
    collections::HashSet,
    ops::Not,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread::yield_now,
};

pub struct Runner {
    world: usize,
    version: usize,
    systems: Vec<System>,
    control: bool,
    index: AtomicUsize,
    success: AtomicBool,
    runs: Vec<Mutex<(Run, State)>>,
    conflict: Conflict,
    pool: ThreadPool,
}

pub struct Run {
    run: Box<dyn FnMut(&mut dyn Any) -> Result + Send>,
    dependencies: Vec<Dependency>,
}

struct State {
    state: Arc<dyn Any + Send + Sync>,
    done: bool,
    error: Option<Error>,
    blockers: Vec<(usize, Error)>,
}

impl World {
    pub fn run<I: Default, M, S: IntoSystem<M, Input = I>>(&mut self, system: S) -> Result {
        self.run_with(I::default(), system)
    }

    pub fn run_with<I, M, S: IntoSystem<M, Input = I>>(&mut self, input: I, system: S) -> Result {
        let mut system = system.system(input, self)?;
        let mut version = 0;
        let mut runs = Vec::new();

        for _ in 0..1_000 {
            if version.change(self.version()) {
                runs = system.schedule(self);
            } else {
                break;
            }
        }
        if version.change(self.version()) {
            return Err(Error::UnstableWorldVersion);
        }

        let mut conflict = Conflict::default();
        Error::all(runs.iter().flat_map(|run| {
            conflict.clear();
            conflict
                .detect(Scope::Inner, run.dependencies(), true)
                .err()
        }))
        .flatten(true)
        .map_or(Ok(()), Err)?;

        for run in runs.iter_mut() {
            run.run(&mut system.state)?;
        }

        Ok(())
    }
}

impl Run {
    pub fn new(
        mut run: impl FnMut(&mut dyn Any) -> Result + Send + Sync + 'static,
        dependencies: impl IntoIterator<Item = Dependency>,
    ) -> Self {
        Self {
            run: Box::new(move |state| run(state)),
            dependencies: dependencies.into_iter().collect(),
        }
    }

    pub(crate) fn run(&mut self, state: &mut dyn Any) -> Result {
        (self.run)(state)
    }

    #[inline]
    pub fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }
}

impl Runner {
    pub fn new<I: IntoIterator<Item = System>>(
        parallelism: usize,
        systems: I,
        world: &mut World,
    ) -> Result<Self> {
        Ok(Self {
            world: world.identifier(),
            version: 0,
            systems: systems.into_iter().collect(),
            control: false,
            index: 0.into(),
            success: AtomicBool::new(true),
            runs: vec![],
            conflict: Conflict::default(),
            pool: ThreadPoolBuilder::new()
                .num_threads(parallelism)
                .build()
                .map_err(|_| Error::FailedToSchedule)?,
        })
    }

    #[inline]
    pub const fn version(&self) -> usize {
        self.version
    }

    #[inline]
    pub fn systems(&self) -> &[System] {
        &self.systems
    }

    pub fn update(&mut self, world: &mut World) -> Result<bool> {
        let success = *self.success.get_mut();
        if self.world != world.identifier() {
            return Err(Error::WrongWorld {
                expected: self.world,
                actual: world.identifier(),
            });
        } else if success && self.version == world.version() {
            return Ok(false);
        }

        let mut version = if success { self.version } else { 0 };
        // 'I::schedule' may cause more changes of the 'world.version()'. Loop until the version has stabilized.
        for _ in 0..1_000 {
            if version.change(world.version()) {
                self.runs = self
                    .systems
                    .iter_mut()
                    .flat_map(|system| {
                        system.schedule(world).into_iter().map(|run| {
                            Mutex::new((
                                run,
                                State {
                                    state: system.state.clone(),
                                    done: self.control,
                                    error: None,
                                    blockers: vec![],
                                },
                            ))
                        })
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

        // Only commit the new version and restore `success` if scheduling and dependency analysis succeed.
        self.version = version;
        *self.success.get_mut() = true;
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
            Error::all(
                runs.iter_mut()
                    .filter_map(|run| run.get_mut().1.error.take()),
            )
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
    /// - This mechanism has a lookahead that is equal to the degree of parallelism which is currently the number of logical CPUs by default.
    fn progress(index: &AtomicUsize, runs: &Vec<Mutex<(Run, State)>>, control: bool) -> bool {
        loop {
            // `Ordering` doesn't matter here, only atomicity.
            let index = index.fetch_add(1, Ordering::Relaxed);
            let mut guard = match runs.get(index) {
                // This `lock` may only contend if the run is a blocker of another run that took the lock before it. This is highly unlikely
                // because this thread would've had to pause after the `fetch_add` and before the `lock` while another thread would've then
                // gone through `fetch_add` itself up to its blocker `lock`. Even if this happened, the other thread detects this
                // state and drops the lock immediately.
                Some(run) => run.lock(),
                None => return true,
            };

            let mut done = 0;
            while let Some(&(blocker, _)) = guard.1.blockers.get(done) {
                // Sanity check. If this is not the case, this thread might spin loop and consume too much CPU.
                debug_assert!(blocker < index);

                // This `lock` may contend on 2 things:
                // - Other threads are also trying to determine if this blocker is done. Since the lock is held for so little time, this
                // should not be worth optimizing.
                // - The blocker thread is executing. This contention exists by design to free up CPU resources while the execution completes.
                let guard = runs[blocker].lock();
                if guard.1.done == control {
                    drop(guard);
                    done += 1
                } else if guard.1.error.is_some() {
                    drop(guard);
                    return false;
                } else {
                    // When the lock is taken, it is expected that `done == control` except if a blocker thread paused
                    // after `index.fetch_add` and before `run.lock`. Even though this is a spin lock, since this should happen
                    // very rarely and is a very transient state, it is considered to be ok.
                    // - `yield_now` is used to give the running thread a chance to acquire the lock.
                    // - Drop the guard before yielding to allow the blocker thread to acquire the lock with fewer context switches.
                    drop(guard);
                    yield_now();
                }
            }

            let state = as_mut(&mut guard.1.state);
            match guard.0.run(state) {
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
            let (run, state) = tail.get_mut();
            self.conflict
                .detect(Scope::Inner, &run.dependencies, true)
                .map_err(Error::Depend)?;

            for (i, rest) in rest.iter_mut().enumerate() {
                let (previous, _) = rest.get_mut();
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
            let (_, state) = tail.get_mut();
            for &(blocker, _) in state.blockers.iter() {
                // `rest[blocker]` ensures that `blocker < rest.len()` which is important when running.
                let (_, previous) = rest[blocker].get_mut();
                set.extend(previous.blockers.iter().map(|&(blocker, _)| blocker));
            }
            state.blockers.retain(|(blocker, _)| !set.contains(blocker));
            set.clear();
            runs = rest;
        }
        Ok(())
    }
}

#[inline]
pub(crate) fn as_mut<'a, T: ?Sized>(state: &mut Arc<T>) -> &'a mut T {
    unsafe { &mut *(Arc::as_ptr(&state) as *mut T) }
}

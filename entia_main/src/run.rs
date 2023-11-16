use crate::{
    depend::{Conflict, Dependency, Order},
    error::{Error, Result},
    system::System,
    world::World,
    IntoSystem,
};
use entia_core::Change;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
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
    runs: Runs,
    conflict: Conflict,
    pool: ThreadPool,
}

type Runs = Vec<(RwLock<(Run, State)>, Blockers)>;

pub struct Run {
    run: Box<dyn FnMut(&mut dyn Any) -> Result + Send + Sync>,
    dependencies: Vec<Dependency>,
}

#[derive(Debug)]
struct State {
    state: Arc<dyn Any + Send + Sync>,
    done: bool,
    error: Option<Error>,
}

#[derive(Default)]
struct Blockers {
    strong: Vec<(usize, AtomicBool, Error)>,
    weak: Vec<(usize, AtomicBool)>,
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
        Error::all(runs.iter().filter_map(|run| {
            conflict.clear();
            conflict.detect_inner(run.dependencies(), true).err()
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
        if self.world != world.identifier() {
            return Err(Error::WrongWorld {
                expected: self.world,
                actual: world.identifier(),
            });
        } else if self.version == world.version() {
            return Ok(false);
        }

        let mut version = self.version;
        // 'I::schedule' may cause more changes of the 'world.version()'. Loop until the version has stabilized.
        for _ in 0..1_000 {
            if version.change(world.version()) {
                self.runs = self
                    .systems
                    .iter_mut()
                    .flat_map(|system| {
                        system.schedule(world).into_iter().map(|run| {
                            (
                                RwLock::new((
                                    run,
                                    State {
                                        state: system.state.clone(),
                                        done: self.control,
                                        error: None,
                                    },
                                )),
                                Blockers::default(),
                            )
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
        Ok(true)
    }

    pub fn run(&mut self, world: &mut World) -> Result {
        self.update(world)?;

        let Self {
            control,
            runs,
            pool,
            ..
        } = self;

        *control = control.not();

        let index = AtomicUsize::new(0);
        let mut success = AtomicBool::new(true);
        let control = *control;
        pool.scope(|scope| {
            for _ in 0..pool.current_num_threads() {
                scope.spawn(|_| {
                    success.fetch_and(Self::progress(&index, runs, control), Ordering::Relaxed);
                });
            }
        });

        if success.into_inner() {
            Ok(())
        } else {
            Error::all(
                runs.iter_mut()
                    .filter_map(|(run, _)| run.get_mut().1.error.take()),
            )
            .flatten(true)
            .map_or(Err(Error::FailedToRun), Err)
        }
    }

    /// The synchronization mechanism is in 2 parts:
    /// 1. An `AtomicUsize` is used to reserve an index in the `runs` vector. It ensures that each run is executed only once.
    /// 2. A `Mutex` around the run and its state that will force the blocked threads to wait until this run is done. This choice
    /// of synchronization prevents sneaky interleaving of threads and is very straightforward to implement.
    /// - This mechanism can not produce a dead lock as long as the `blockers` are all indices `< index` (which they are by design).
    /// Since runs are executed in order, for any index that is reserved, all indices smaller than that index represent a run that
    /// is done or in progress (not idle) which is important to prevent a spin loop when waiting for `blockers` to finish.
    /// - This mechanism has a lookahead that is equal to the degree of parallelism which is currently the number of logical CPUs by default.
    fn progress(index: &AtomicUsize, runs: &Runs, control: bool) -> bool {
        fn progress(index: usize, runs: &Runs, control: bool, lock: bool) -> Option<bool> {
            let (run, blockers) = match runs.get(index) {
                Some(run) => run,
                None => return Some(true),
            };

            match if lock {
                Some(run.read())
            } else {
                run.try_read()
            } {
                Some(guard) if guard.1.done == control => return Some(true),
                Some(guard) if guard.1.error.is_some() => return None,
                _ => {}
            }

            let mut ready = true;
            for (blocker, done, _) in blockers.strong.iter() {
                debug_assert!(*blocker < index);

                if done.load(Ordering::Acquire) == control {
                    continue;
                }

                // Do not try to `progress` here since if `blocker < index`, it is expected that the blocker index will be
                // locked by its responsible thread imminently. So this lock should be kept for the least amount of time.
                match if lock {
                    Some(runs[*blocker].0.read())
                } else {
                    runs[*blocker].0.try_read()
                } {
                    Some(guard) if guard.1.done == control => {
                        done.store(control, Ordering::Release)
                    }
                    Some(guard) if guard.1.error.is_some() => return None,
                    Some(_) | None => ready = false,
                };
            }

            let mut guards = Vec::new();
            for (blocker, done) in blockers.weak.iter() {
                if done.load(Ordering::Acquire) == control {
                    continue;
                }

                match runs[*blocker].0.try_read() {
                    Some(guard) if guard.1.done => done.store(control, Ordering::Release),
                    Some(guard) if guard.1.error.is_some() => return None,
                    Some(guard) if ready => guards.push(guard),
                    guard => {
                        drop(guard);
                        if guards.len() > 0 {
                            guards.clear();
                            ready = false;
                        } else if progress(*blocker, runs, control, false)? {
                            done.store(control, Ordering::Release);
                        } else {
                            ready = false;
                        }
                    }
                };
            }

            if ready {
                let guard = run.upgradable_read();
                if guard.1.done == control {
                    return Some(true);
                } else if guard.1.error.is_some() {
                    return None;
                }

                let mut guard = RwLockUpgradableReadGuard::upgrade(guard);
                let input = as_mut(&mut guard.1.state);
                let result = guard.0.run(input);
                guards.clear();
                match result {
                    Ok(_) => {
                        guard.1.done = control;
                        Some(true)
                    }
                    Err(error) => {
                        guard.1.error = Some(error);
                        None
                    }
                }
            } else {
                Some(false)
            }
        }

        loop {
            // `Ordering` doesn't matter here, only atomicity.
            let index = index.fetch_add(1, Ordering::Relaxed);
            // let mut guard = match runs.get(index) {
            //     // This `lock` may only contend if the run is a blocker of another run that took the lock before it. This is highly unlikely
            //     // because this thread would've had to pause after the `fetch_add` and before the `lock` while another thread would've then
            //     // gone through `fetch_add` itself up to its blocker `lock`. Even if this happened, the other thread detects this
            //     // state and drops the lock immediately.
            //     Some(run) => run.lock(),
            //     None => return true,
            // };

            match progress(index, &runs, control, false) {
                Some(true) => continue,
                Some(false) => loop {
                    match progress(index, &runs, control, true) {
                        Some(true) => break,
                        Some(false) => yield_now(),
                        None => return false,
                    }
                },
                None => return false,
            }

            // let mut done = 0;
            // while let Some(&(blocker, _)) = guard.1.strict_blockers.get(done) {
            //     // Sanity check. If this is not the case, this thread might spin loop and consume too much CPU.
            //     debug_assert!(blocker < index);

            //     // This `lock` may contend on 2 things:
            //     // - Other threads are also trying to determine if this blocker is done. Since the lock is held for so little time, this
            //     // should not be worth optimizing.
            //     // - The blocker thread is executing. This contention exists by design to free up CPU resources while the execution completes.
            //     let guard = runs[blocker].lock();
            //     if guard.1.end == control {
            //         drop(guard);
            //         done += 1
            //     } else if guard.1.error.is_some() {
            //         drop(guard);
            //         return false;
            //     } else {
            //         // When the lock is taken, it is expected that `done == control` except if a blocker thread paused
            //         // after `index.fetch_add` and before `run.lock`. Even though this is a spin lock, since this should happen
            //         // very rarely and is a very transient state, it is considered to be ok.
            //         // - `yield_now` is used to give the running thread a chance to acquire the lock.
            //         // - Drop the guard before yielding to allow the blocker thread to acquire the lock with fewer context switches.
            //         drop(guard);
            //         yield_now();
            //     }
            // }

            // let state = as_mut(&mut guard.1.state);
            // match guard.0.run(state) {
            //     Ok(_) => guard.1.end = control,
            //     Err(error) => {
            //         guard.1.error = Some(error);
            //         return false;
            //     }
            // }
        }
    }

    /// Remove transitive pre blockers.
    fn refine_strong_blockers(&mut self) {
        let mut runs = &mut self.runs[..];
        let mut set = HashSet::new();
        while let Some(((_, tail), rest)) = runs.split_last_mut() {
            for &(blocker, _, _) in tail.strong.iter() {
                // `rest[blocker]` ensures that `blocker < rest.len()` which is important when running.
                let strong = rest[blocker].1.strong.iter();
                set.extend(strong.map(|&(blocker, _, _)| blocker));
            }
            tail.strong.retain(|(blocker, _, _)| !set.contains(blocker));
            set.clear();
            runs = rest;
        }
    }

    /// Removes the post blockers that have pre blockers later than the current run.
    // fn refine_weak_blockers(&mut self) {
    //     let mut runs = &mut self.runs[..];
    //     let mut index = 0;
    //     while let Some((head, rest)) = runs.split_first_mut() {
    //         let (_, state) = head.get_mut();
    //         index += 1;
    //         state.weak_blockers.retain(|&(blocker, _)| {
    //             let (_, next) = rest[blocker - index].get_mut();
    //             next.strong_blockers
    //                 .iter()
    //                 .all(|&(blocker, _, _)| blocker < index)
    //         });
    //         runs = rest;
    //     }
    // }

    fn schedule(&mut self) -> Result {
        // TODO: This algorithm scales poorly (n^2 / 2, where n is the number of runs) and a lot of the work is redundant
        // as shown by the refining part. This could be optimized.
        let mut runs = &mut self.runs[..];
        while let Some((tail, rest)) = runs.split_last_mut() {
            let (run, _) = tail.0.get_mut();
            self.conflict.detect_inner(&run.dependencies, true)?;

            let index = rest.len();
            for (i, rest) in rest.iter_mut().enumerate() {
                let pair = rest.0.get_mut();
                match self.conflict.detect_outer(&pair.0.dependencies, true) {
                    Ok(Order::Strict) => {}
                    Ok(Order::Relax) => {
                        tail.1.weak.push((i, self.control.into()));
                        rest.1.weak.push((index, self.control.into()));
                    }
                    Err(error) => tail.1.strong.push((i, self.control.into(), error.into())),
                }
            }

            runs = rest;
        }

        self.refine_strong_blockers();
        // self.refine_weak_blockers();
        Ok(())
    }
}

#[inline]
pub(crate) fn as_mut<'a, T: ?Sized>(state: &mut Arc<T>) -> &'a mut T {
    unsafe { &mut *(Arc::as_ptr(&state) as *mut T) }
}

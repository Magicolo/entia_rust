use crate::dependency::Dependency;
use crate::Inject;
use crate::Resource;
use crate::World;
use crossbeam::scope;
use std::mem::{replace, transmute};

pub type Run = Box<dyn FnMut()>;
pub struct Runner {
    run: Run,
    dependencies: Vec<Dependency>,
}

pub type Schedule = Box<dyn FnOnce(World) -> Runner>;
pub struct Scheduler {
    schedules: Vec<Schedule>,
}

pub trait System<'a, P = ()> {
    fn schedule(self, world: World) -> Runner;
}

impl Runner {
    #[inline]
    pub fn new<F: FnMut() + 'static>(run: F, dependencies: Vec<Dependency>) -> Self {
        let run = Box::new(run);
        Self { run, dependencies }
    }
}

impl Default for Runner {
    #[inline]
    fn default() -> Self {
        Self::new(|| {}, Vec::new())
    }
}

impl Scheduler {
    #[inline]
    pub fn new() -> Self {
        Self {
            schedules: Vec::new(),
        }
    }

    pub fn add<'a, P, S: System<'a, P> + 'static>(mut self, system: S) -> Self {
        self.schedules
            .push(Box::new(move |world| system.schedule(world)));
        self
    }
}

impl<'a> System<'a> for Scheduler {
    fn schedule(self, world: World) -> Runner {
        fn take<T>(vector: &mut Vec<T>) -> Vec<T> {
            replace(vector, Vec::new())
        }

        fn unzip(mut runners: Vec<Runner>) -> (Vec<Run>, Vec<Dependency>) {
            let mut runs = Vec::with_capacity(runners.len());
            let mut dependencies = Vec::with_capacity(runners.len());
            for mut runner in runners.drain(..) {
                runs.push(runner.run);
                dependencies.append(&mut runner.dependencies);
            }
            (runs, dependencies)
        }

        fn as_sequence(mut runners: Vec<Runner>) -> Option<Runner> {
            if runners.len() <= 1 {
                return runners.pop();
            }

            let (mut runs, dependencies) = unzip(runners);
            Some(Runner::new(
                move || {
                    for run in &mut runs {
                        run();
                    }
                },
                dependencies,
            ))
        }

        fn as_parallel(mut runners: Vec<Runner>) -> Option<Runner> {
            if runners.len() <= 1 {
                return runners.pop();
            }

            let (mut runs, dependencies) = unzip(runners);
            Some(Runner::new(
                move || {
                    // TODO: This is most likely not the most performant way to parallelize systems.
                    scope(|scope| {
                        for run in &mut runs {
                            let run: &mut Box<dyn FnMut() + Send> = unsafe { transmute(run) };
                            scope.spawn(move |_| run());
                        }
                    })
                    .unwrap();
                },
                dependencies,
            ))
        }

        let mut dependencies = Vec::new();
        let mut sequence = Vec::new();
        let mut parallel = Vec::new();
        let mut start = 0;
        for schedule in self.schedules {
            let runner = schedule(world.clone());
            dependencies.extend(runner.dependencies.iter());
            if Dependency::synchronous(&dependencies[start..]) {
                start = dependencies.len();
                if let Some(parallel) = as_parallel(take(&mut parallel)) {
                    sequence.push(parallel);
                }
            }
            parallel.push(runner);
        }

        if let Some(parallel) = as_parallel(take(&mut parallel)) {
            sequence.push(parallel);
        }

        as_sequence(take(&mut parallel)).unwrap_or_default()
    }
}

fn test(world: World) {
    struct Time {}
    impl Resource for Time {}
    let mut runner = Scheduler::new()
        .add(|time: &mut Time| {})
        // .add(|boba: &mut Boba| {})
        .schedule(world);

    loop {
        (runner.run)();
    }
}

// TODO: implement actual systems
impl<'a, I: Inject<'a>, F: Fn(I)> System<'a, [I; 1]> for F {
    fn schedule(self, mut world: World) -> Runner {
        let a = I::inject(&mut world);
        Runner::default()
        // .map(|state| {
        //     Runner::new(
        //         move |world| self(I::get(&mut state, world)),
        //         I::dependencies(),
        //     )
        // })
    }
}

pub trait Fett<'a> {
    type State: 'a;

    fn inject(world: World) -> Option<Self::State>;
    fn get(state: &'a mut Self::State) -> Self;
}

pub struct Boba(World);
impl<'a> Fett<'a> for &'a mut Boba {
    type State = Boba;

    fn inject(world: World) -> Option<Self::State> {
        Some(Boba(world))
    }

    fn get(state: &'a mut Self::State) -> Self {
        state
    }
}

impl<'a, R: Resource + 'static> Fett<'a> for &'a mut R {
    type State = (World, usize, usize);

    fn inject(world: World) -> Option<Self::State> {
        Some((world, 0, 0))
    }

    fn get(state: &'a mut Self::State) -> Self {
        let (world, segment, store) = state;
        unsafe {
            let store = world.get().segments[*segment].storez[*store].0 as *mut R;
            &mut *store
        }
    }
}

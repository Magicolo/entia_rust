use crate::component::Segment;
use crate::component::Store;
use crate::dependency::Dependency;
use crate::Component;
use crate::Entity;
use crate::Resource;
use crate::World;
use crossbeam::scope;
use std::mem::replace;

fn test(world: World) {
    struct Time;
    impl Resource for Time {}
    struct Position;
    struct Velocity;
    impl Component for Position {}
    impl Component for Velocity {}
    let mut runner = Scheduler::new()
        .add(|_: &mut Time| {})
        .add(|time: &mut Time| |entity: Entity| {})
        .add(|group: Group<Entity>| for entity in group.into_iter() {})
        .add(|group: Group<&Position>| for position in group.into_iter() {})
        .schedule(world);

    loop {
        runner.run();
    }
}

pub type Run = Box<dyn FnMut()>;
pub enum Runner {
    System(Run, Vec<Dependency>),
    Sequence(Vec<Runner>),
    Parallel(Vec<Runner>),
}

pub type Schedule = Box<dyn FnOnce(World) -> Runner>;
pub struct Scheduler {
    schedules: Vec<Schedule>,
}

// Traits 'System', 'Inject' and 'Query' are marked as unsafe because a wrong implementation could cause
// all sorts of undefined behaviors. An implementor must be aware of the implicit requirements of these traits.
pub unsafe trait System<P = ()> {
    fn schedule(self, world: World) -> Runner;
}

pub unsafe trait Inject {
    type State: 'static;

    fn dependencies() -> Vec<Dependency>;
    fn state(world: World) -> Option<Self::State>;
    unsafe fn inject(state: &mut Self::State) -> Self;
}

pub unsafe trait Query {
    type State: 'static;

    fn dependencies() -> Vec<Dependency>;
    fn state(segment: Segment, world: World) -> Option<Self::State>;
    unsafe fn query(state: &Self::State, index: usize) -> Self;
}

#[derive(Clone)]
pub struct Group<'a, Q: Query> {
    states: &'a Vec<(Q::State, usize)>,
}

pub struct GroupIterator<'a, Q: Query> {
    indices: (usize, usize),
    group: &'a Group<'a, Q>,
}

impl<'a, Q: Query> IntoIterator for &'a Group<'a, Q> {
    type Item = Q;
    type IntoIter = GroupIterator<'a, Q>;

    fn into_iter(self) -> Self::IntoIter {
        GroupIterator {
            indices: (0, 0),
            group: self,
        }
    }
}

impl<'a, Q: Query> Iterator for GroupIterator<'a, Q> {
    type Item = Q;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((state, count)) = self.group.states.get(self.indices.0) {
            let index = self.indices.1;
            if index < *count {
                self.indices.1 += 1;
                return Some(unsafe { Q::query(state, index) });
            } else {
                self.indices.0 += 1;
            }
        }
        None
    }
}

unsafe impl Sync for Runner {}
unsafe impl Send for Runner {}
impl Runner {
    pub fn optimize(self) -> Self {
        // - remove empty parallel/sequence
        // - replace singleton parallel/sequence by the first child
        // - merge sequence of sequences
        // - can parallel of parallels can be merged?
        todo!()
    }

    pub fn run(&mut self) {
        match self {
            Runner::System(run, _) => run(),
            Runner::Sequence(runners) => {
                for runner in runners {
                    runner.run();
                }
            }
            Runner::Parallel(runners) => scope(|scope| {
                for runner in runners {
                    scope.spawn(move |_| runner.run());
                }
            })
            .unwrap(),
        }
    }

    pub fn dependencies(&self) -> Vec<Dependency> {
        match self {
            Runner::System(_, dependencies) => dependencies.clone(),
            Runner::Sequence(runners) | Runner::Parallel(runners) => runners
                .iter()
                .map(|runner| runner.dependencies())
                .flatten()
                .collect(),
        }
    }
}

impl Default for Runner {
    #[inline]
    fn default() -> Self {
        Runner::Sequence(Vec::new())
    }
}

impl Scheduler {
    #[inline]
    pub fn new() -> Self {
        Self {
            schedules: Vec::new(),
        }
    }

    pub fn add<P, S: System<P> + 'static>(mut self, system: S) -> Self {
        self.schedules
            .push(Box::new(move |world| system.schedule(world)));
        self
    }
}

unsafe impl System for Scheduler {
    fn schedule(self, world: World) -> Runner {
        fn take<T>(vector: &mut Vec<T>) -> Vec<T> {
            replace(vector, Vec::new())
        }

        let mut dependencies = Vec::new();
        let mut sequence = Vec::new();
        let mut parallel = Vec::new();
        let mut start = 0;
        for schedule in self.schedules {
            let runner = schedule(world.clone());
            dependencies.append(&mut runner.dependencies());
            if Dependency::synchronous(&dependencies[start..]) {
                start = dependencies.len();
                sequence.push(Runner::Parallel(take(&mut parallel)));
            }
            parallel.push(runner);
        }
        sequence.push(Runner::Parallel(take(&mut parallel)));
        Runner::Sequence(take(&mut sequence)).optimize()
    }
}

unsafe impl<'a, Q: Query + 'static> Inject for Group<'a, Q> {
    type State = (usize, Vec<(Q::State, usize)>, World);

    fn dependencies() -> Vec<Dependency> {
        Q::dependencies()
    }

    fn state(world: World) -> Option<Self::State> {
        Some((0, Vec::new(), world))
    }

    unsafe fn inject(state: &mut Self::State) -> Self {
        todo!()
        // let (index, states, world) = state;
        // let segments = world.get().segments;
        // for i in *index..segments.len() {
        //     let segment = &segments[i];
        //     if let Some(state) = Q::state(segment.clone(), world.clone()) {
        //         states.push((state, segment.get().entities.len()));
        //     }
        // }
        // Group { states }
    }
}

unsafe impl<R: Resource + 'static> Inject for &R {
    type State = Store<R>;

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::write::<R>()]
    }

    fn state(world: World) -> Option<Self::State> {
        unsafe { world.get().get_resource_store() }
    }

    unsafe fn inject(state: &mut Self::State) -> Self {
        &(&*state.inner.get())[0]
    }
}

unsafe impl<R: Resource + 'static> Inject for &mut R {
    type State = Store<R>;

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::write::<R>()]
    }

    fn state(world: World) -> Option<Self::State> {
        unsafe { world.get().get_resource_store() }
    }

    unsafe fn inject(state: &mut Self::State) -> Self {
        &mut (&mut *state.inner.get())[0]
    }
}

unsafe impl Query for Entity {
    type State = Segment;

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::read::<Entity>()]
    }

    fn state(segment: Segment, _: World) -> Option<Self::State> {
        Some(segment)
    }

    #[inline(always)]
    unsafe fn query(state: &Self::State, index: usize) -> Self {
        state.get().entities[index]
    }
}

unsafe impl<C: Component + 'static> Query for &C {
    type State = Store<C>;

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::read::<Entity>()]
    }

    fn state(segment: Segment, _: World) -> Option<Self::State> {
        segment.get_store().cloned()
    }

    #[inline(always)]
    unsafe fn query(state: &Self::State, index: usize) -> Self {
        &(&*state.inner.get())[index]
    }
}

unsafe impl<C: Component + 'static> Query for &mut C {
    type State = Store<C>;

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::read::<Entity>()]
    }

    fn state(segment: Segment, _: World) -> Option<Self::State> {
        segment.get_store().cloned()
    }

    #[inline(always)]
    unsafe fn query(state: &Self::State, index: usize) -> Self {
        &mut (&mut *state.inner.get())[index]
    }
}

#[inline]
fn update_inject<I: Inject>(state: &mut Option<I::State>, world: World) -> Option<I> {
    if state.is_none() {
        *state = I::state(world);
    }
    state.as_mut().map(|state| unsafe { I::inject(state) })
}

#[inline]
fn update_queries<Q: Query>(states: &mut Vec<(Q::State, usize)>, index: &mut usize, world: World) {
    let segments = unsafe { &world.get().segments };
    for i in *index..segments.len() {
        let segment = &segments[i];
        if let Some(state) = Q::state(segment.clone(), world.clone()) {
            states.push((state, unsafe { segment.get().entities.len() }));
        }
    }
    *index = segments.len();
}

#[inline]
fn run_queries<Q: Query, F: Fn(Q)>(run: &F, states: &Vec<(Q::State, usize)>) {
    for (state, count) in states {
        for index in 0..*count {
            run(unsafe { Q::query(state, index) })
        }
    }
}

unsafe impl<I: Inject, F: Fn(I) + 'static> System<[(I, ()); 1]> for F {
    fn schedule<'a>(self, world: World) -> Runner {
        let mut state = None;
        Runner::System(
            Box::new(move || {
                if let Some(inject) = update_inject(&mut state, world.clone()) {
                    self(inject);
                }
            }),
            I::dependencies(),
        )
    }
}

unsafe impl<Q: Query, F: Fn(Q) + 'static> System<[((), Q); 2]> for F {
    fn schedule(self, world: World) -> Runner {
        let mut states: Vec<(Q::State, usize)> = Vec::new();
        let mut index = 0;
        Runner::System(
            Box::new(move || {
                update_queries::<Q>(&mut states, &mut index, world.clone());
                run_queries(&self, &states);
            }),
            Q::dependencies(),
        )
    }
}

unsafe impl<I: Inject, Q: Query, FI: Fn(I) -> FQ + 'static, FQ: Fn(Q)> System<[(I, Q); 3]> for FI {
    fn schedule(self, world: World) -> Runner {
        let mut state: Option<I::State> = None;
        let mut states: Vec<(Q::State, usize)> = Vec::new();
        let mut index = 0;
        Runner::System(
            Box::new(move || {
                if let Some(inject) = update_inject::<I>(&mut state, world.clone()) {
                    update_queries::<Q>(&mut states, &mut index, world.clone());
                    run_queries(&self(inject), &states);
                }
            }),
            vec![I::dependencies(), Q::dependencies()]
                .drain(..)
                .flatten()
                .collect(),
        )
    }
}

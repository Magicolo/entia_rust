use std::any::type_name;
use std::any::Any;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

/*
SYSTEMS
- Runners must be able to re-initialize and re-schedule all systems when a segment is added.
- This will happen when the 'Defer' module is resolved which occurs at the next synchronization point.
- There should not be a significant performance hit since segment addition/removal is expected to be rare and happen mainly
in the first frames of execution.

RESOURCES
- There will be 1 segment per resource such that the same segment/dependency system can be used for them.
- Resource segments should only allocate 1 store with 1 slot with the resource in it.
- Resource entities must not be query-able (could be accomplished with a simple 'bool' in segments).

DEPENDENCIES
- Design a contract API that ensures that dependencies are well defined.
- To gain access to a given resource, a user must provide a corresponding 'Contract' that is provided by a 'Contractor'.
- The 'Contractor' then stores a copy of each emitted contract to later convert them into corresponding dependencies.
- Ex: System::initialize(contractor: &mut Contractor, world: &mut World) -> Store<Time> {
    world.get_resource(contractor.resource(TypeId::of::<Time>()))
        OR
    world.get_resource::<Time>(contractor)
        OR
    contractor.resource::<Time>(world) // This doesn't require the 'World' to know about the 'Contractor'.
        OR
    contractor.resource::<Time>() // The contractor can hold its own reference to the 'World'.
}
*/

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entity {
    index: u32,
    generation: u32,
}

#[derive(Default)]
pub struct WorldInner {
    _entities: AtomicPtr<Entity>,
    _last: AtomicUsize,
    segments: Vec<Segment>,
}
#[derive(Default)]
pub struct World {
    inner: Arc<WorldInner>,
}

pub struct With<Q: Query> {
    _marker: PhantomData<Q>,
}

pub struct SegmentInner {
    pub index: usize,
    pub types: Vec<TypeId>,
    pub entities: Vec<Entity>,
    pub stores: Vec<Arc<dyn Any + Send + Sync + 'static>>,
    pub indices: HashMap<TypeId, usize>,
}
pub struct Segment {
    inner: Arc<SegmentInner>,
}
pub struct Group<Q: Query> {
    inner: Arc<WorldInner>,
    queries: Arc<Vec<(Q::State, Arc<SegmentInner>)>>,
}
pub struct GroupIterator<Q: Query> {
    segment: usize,
    index: usize,
    group: Group<Q>,
}

pub struct Run<'a> {
    _name: String,
    update: Box<dyn FnMut() -> Vec<Dependency> + 'a>,
    resolve: Box<dyn Fn() + 'a>,
    run: Box<dyn Fn() + Sync + 'a>,
}
pub struct Runner<'a>(Box<dyn FnMut() + 'a>);
#[derive(Debug, Clone)]
pub enum Dependency {
    Unknown,
    Read(usize, TypeId),
    Write(usize, TypeId),
}

#[derive(Default, Clone)]
pub struct Scheduler {
    schedules: Vec<Arc<dyn for<'b> Fn(&'b World) -> Option<Run<'b>>>>,
}
pub trait Resource: Send + Sync + 'static {}
pub trait Component: Send + Sync + 'static {}

pub trait Inject {
    type State;
    fn initialize(world: &World) -> Option<Self::State>;
    fn update(state: &mut Self::State) -> Vec<Dependency>;
    fn resolve(state: &Self::State);
    fn get(state: &Self::State) -> Self;
}

pub trait Query {
    type State;
    fn initialize(segment: &Segment) -> Option<Self::State>;
    fn update(state: &mut Self::State) -> Vec<Dependency>;
    fn resolve(state: &Self::State);
    fn get(index: usize, state: &Self::State) -> Self;
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

pub struct Defer {}
pub struct Template<T>(T);
pub struct Wrap<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Wrap<T> {}
impl<T> Wrap<T> {
    pub fn new(value: T) -> Self {
        Wrap(UnsafeCell::new(value))
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
            // TODO: Validate the internal system's dependencies (ex: [&mut Position, &mut Position] must be invalid).
            // - return a 'Result<Runner<'a>, Error>'
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
            dependencies: Vec<Dependency>,
            reads: &mut HashSet<(usize, TypeId)>,
            writes: &mut HashSet<(usize, TypeId)>,
        ) -> bool {
            for dependency in dependencies {
                match dependency {
                    Dependency::Unknown => return true,
                    Dependency::Read(segment, store) => {
                        let pair = (segment, store);
                        if writes.contains(&pair) {
                            return true;
                        }
                        reads.insert(pair);
                    }
                    Dependency::Write(segment, store) => {
                        let pair = (segment, store);
                        if reads.contains(&pair) || writes.contains(&pair) {
                            return true;
                        }
                        writes.insert(pair);
                    }
                }
            }
            false
        }

        fn schedule<'a>(runs: impl Iterator<Item = Run<'a>>) -> Vec<Vec<Run<'a>>> {
            let mut sequence = Vec::new();
            let mut parallel = Vec::new();
            let mut reads = HashSet::new();
            let mut writes = HashSet::new();

            for mut run in runs {
                if conflicts((run.update)(), &mut reads, &mut writes) {
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
            sequence
        }

        let mut runs = Vec::new();
        for schedule in self.schedules.iter() {
            runs.push(schedule(world)?);
        }
        let mut sequence = schedule(runs.drain(..));
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
                sequence = schedule(sequence.drain(..).flatten());
            }
        })))
    }
}

impl<Q: Query> Default for With<Q> {
    fn default() -> Self {
        With {
            _marker: PhantomData,
        }
    }
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn find_segment(&self, _types: &[TypeId]) -> Option<Segment> {
        todo!()
    }
}

impl Defer {
    pub fn create<T>(&self, _entities: &mut [Entity], _template: Template<T>) {}
    pub fn destroy(&self, _entities: &[Entity]) {}
    pub fn add<C: Component>(&self, _entity: Entity, _component: C) {}
    pub fn remove<C: Component>(&self, _entity: Entity) {}
}

impl Inject for Defer {
    type State = ();

    fn initialize(_: &World) -> Option<Self::State> {
        Some(())
    }

    fn update(_: &mut Self::State) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State) {
        todo!()
    }

    fn get(_: &Self::State) -> Self {
        todo!()
    }
}

impl<R: Resource> Inject for &R {
    type State = (Arc<SegmentInner>, Arc<Vec<Wrap<R>>>);

    fn initialize(world: &World) -> Option<Self::State> {
        let segment = world.find_segment(&[TypeId::of::<R>()])?;
        let store = segment.inner.stores[0].clone().downcast().ok()?;
        Some((segment.inner.clone(), store))
    }

    fn update((segment, _): &mut Self::State) -> Vec<Dependency> {
        vec![Dependency::Read(segment.index, TypeId::of::<R>())]
    }

    fn resolve(_: &Self::State) {}

    fn get((_, store): &Self::State) -> Self {
        unsafe { &*store[0].0.get() }
    }
}

impl<R: Resource> Inject for &mut R {
    type State = (Arc<SegmentInner>, Arc<Vec<Wrap<R>>>);

    fn initialize(world: &World) -> Option<Self::State> {
        let segment = world.find_segment(&[TypeId::of::<R>()])?;
        let store = segment.inner.stores[0].clone().downcast().ok()?;
        Some((segment.inner.clone(), store))
    }

    fn update((segment, _): &mut Self::State) -> Vec<Dependency> {
        vec![Dependency::Write(segment.index, TypeId::of::<R>())]
    }

    fn resolve(_: &Self::State) {}

    fn get((_, store): &Self::State) -> Self {
        unsafe { &mut *store[0].0.get() }
    }
}

impl Inject for () {
    type State = ();

    fn initialize(_: &World) -> Option<Self::State> {
        Some(())
    }

    fn update(_: &mut Self::State) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State) {}

    fn get(_: &Self::State) -> Self {
        ()
    }
}

impl<I: Inject> Inject for (I,) {
    type State = I::State;

    fn initialize(world: &World) -> Option<Self::State> {
        I::initialize(world)
    }

    fn update(state: &mut Self::State) -> Vec<Dependency> {
        I::update(state)
    }

    fn resolve(state: &Self::State) {
        I::resolve(state);
    }

    fn get(state: &Self::State) -> Self {
        (I::get(state),)
    }
}

impl<I1: Inject, I2: Inject> Inject for (I1, I2) {
    type State = (I1::State, I2::State);

    fn initialize(world: &World) -> Option<Self::State> {
        Some((I1::initialize(world)?, I2::initialize(world)?))
    }

    fn update((state1, state2): &mut Self::State) -> Vec<Dependency> {
        let mut dependencies = I1::update(state1);
        dependencies.append(&mut I2::update(state2));
        dependencies
    }

    fn resolve((state1, state2): &Self::State) {
        I1::resolve(state1);
        I2::resolve(state2);
    }

    fn get((state1, state2): &Self::State) -> Self {
        (I1::get(state1), I2::get(state2))
    }
}

impl<Q: Query> Inject for Group<Q> {
    type State = (
        usize,
        Arc<Vec<(Q::State, Arc<SegmentInner>)>>,
        Arc<WorldInner>,
    );

    fn initialize(world: &World) -> Option<Self::State> {
        Some((0, Arc::new(Vec::new()), world.inner.clone()))
    }

    fn update((index, queries, inner): &mut Self::State) -> Vec<Dependency> {
        // TODO: Ensure that a user cannot persist a 'Group<Q>' outside of the execution of a system.
        // - Otherwise, 'Arc::get_mut' will fail...
        let mut dependencies = Vec::new();
        if let Some(queries) = Arc::get_mut(queries) {
            query_update::<Q>(index, queries, &inner.segments, &mut dependencies);
        }
        dependencies
    }

    fn resolve((_, queries, _): &Self::State) {
        for (query, _) in queries.iter() {
            Q::resolve(query);
        }
    }

    fn get((_, queries, inner): &Self::State) -> Self {
        Group {
            inner: inner.clone(),
            queries: queries.clone(),
        }
    }
}

impl<Q: Query> Group<Q> {
    #[inline]
    pub fn each<F: Fn(Q)>(&self, each: F) {
        for (query, segment) in self.queries.iter() {
            for i in 0..segment.entities.len() {
                each(Q::get(i, query));
            }
        }
    }
}

impl<Q: Query> Clone for Group<Q> {
    fn clone(&self) -> Self {
        Group {
            inner: self.inner.clone(),
            queries: self.queries.clone(),
        }
    }
}

impl<Q: Query> IntoIterator for Group<Q> {
    type Item = Q;
    type IntoIter = GroupIterator<Q>;

    fn into_iter(self) -> Self::IntoIter {
        GroupIterator {
            segment: 0,
            index: 0,
            group: self,
        }
    }
}

impl<Q: Query> IntoIterator for &Group<Q> {
    type Item = Q;
    type IntoIter = GroupIterator<Q>;

    fn into_iter(self) -> Self::IntoIter {
        GroupIterator {
            segment: 0,
            index: 0,
            group: self.clone(),
        }
    }
}

impl<Q: Query> Iterator for GroupIterator<Q> {
    type Item = Q;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((query, segment)) = self.group.queries.get(self.segment) {
            if self.index < segment.entities.len() {
                let query = Q::get(self.index, query);
                self.index += 1;
                return Some(query);
            } else {
                self.segment += 1;
                self.index = 0;
            }
        }
        None
    }
}

impl Query for Entity {
    type State = Arc<SegmentInner>;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Some(segment.inner.clone())
    }

    fn update(_: &mut Self::State) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get(index: usize, inner: &Self::State) -> Self {
        inner.entities[index]
    }
}

impl<C: Component> Query for &C {
    type State = (Arc<Vec<Wrap<C>>>, Arc<SegmentInner>);

    fn initialize(segment: &Segment) -> Option<Self::State> {
        let inner = segment.inner.clone();
        let index = inner.indices.get(&TypeId::of::<C>())?;
        let store = inner.stores.get(*index)?;
        let store = store.clone().downcast().ok()?;
        Some((store, inner))
    }

    fn update((_, inner): &mut Self::State) -> Vec<Dependency> {
        vec![Dependency::Read(inner.index, TypeId::of::<C>())]
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get(index: usize, (store, _): &Self::State) -> Self {
        unsafe { &*store[index].0.get() }
    }
}

impl<C: Component> Query for &mut C {
    type State = (Arc<Vec<Wrap<C>>>, Arc<SegmentInner>);

    fn initialize(segment: &Segment) -> Option<Self::State> {
        let inner = segment.inner.clone();
        let index = inner.indices.get(&TypeId::of::<C>())?;
        let store = inner.stores.get(*index)?;
        let store = store.clone().downcast().ok()?;
        Some((store, inner))
    }

    fn update((_, inner): &mut Self::State) -> Vec<Dependency> {
        vec![Dependency::Write(inner.index, TypeId::of::<C>())]
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get(index: usize, (store, _): &Self::State) -> Self {
        unsafe { &mut *store[index].0.get() }
    }
}

impl<Q: Query> Query for With<Q> {
    type State = ();

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Q::initialize(segment).map(|_| ())
    }

    fn update(_: &mut Self::State) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get(_: usize, _: &Self::State) -> Self {
        With::default()
    }
}

impl<Q: Query> Query for Option<Q> {
    type State = Option<Q::State>;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Some(Q::initialize(segment))
    }

    fn update(state: &mut Self::State) -> Vec<Dependency> {
        match state {
            Some(state) => Q::update(state),
            None => Vec::new(),
        }
    }

    fn resolve(state: &Self::State) {
        match state {
            Some(state) => Q::resolve(state),
            None => {}
        }
    }

    #[inline]
    fn get(index: usize, state: &Self::State) -> Self {
        match state {
            Some(state) => Some(Q::get(index, state)),
            None => None,
        }
    }
}

impl<Q: Query> Query for (Q,) {
    type State = Q::State;

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Q::initialize(segment)
    }

    fn update(state: &mut Self::State) -> Vec<Dependency> {
        Q::update(state)
    }

    fn resolve(state: &Self::State) {
        Q::resolve(state);
    }

    #[inline]
    fn get(index: usize, state: &Self::State) -> Self {
        (Q::get(index, state),)
    }
}

impl<Q1: Query, Q2: Query> Query for (Q1, Q2) {
    type State = (Q1::State, Q2::State);

    fn initialize(segment: &Segment) -> Option<Self::State> {
        Some((Q1::initialize(segment)?, Q2::initialize(segment)?))
    }

    fn update((state1, state2): &mut Self::State) -> Vec<Dependency> {
        let mut dependencies = Q1::update(state1);
        dependencies.append(&mut Q2::update(state2));
        dependencies
    }

    fn resolve((state1, state2): &Self::State) {
        Q1::resolve(state1);
        Q2::resolve(state2);
    }

    #[inline]
    fn get(index: usize, (state1, state2): &Self::State) -> Self {
        (Q1::get(index, state1), Q2::get(index, state2))
    }
}

impl<I: Inject, F: Fn(I)> System<(I::State, PhantomData<&'static I>)> for F {
    fn initialize(world: &World) -> Option<(I::State, PhantomData<&'static I>)> {
        Some((I::initialize(world)?, PhantomData))
    }

    fn update((state, _): &mut (I::State, PhantomData<&'static I>)) -> Vec<Dependency> {
        I::update(state)
    }

    fn resolve((inject, _): &(I::State, PhantomData<&'static I>)) {
        I::resolve(inject);
    }

    fn run(&self, (state, _): &(I::State, PhantomData<&'static I>)) {
        self(I::get(state));
    }
}

fn query_update<Q: Query>(
    index: &mut usize,
    queries: &mut Vec<(Q::State, Arc<SegmentInner>)>,
    segments: &Vec<Segment>,
    dependencies: &mut Vec<Dependency>,
) {
    while let Some(segment) = segments.get(*index) {
        if let Some(query) = Q::initialize(&segment) {
            queries.push((query, segment.inner.clone()))
        }
        *index += 1;
    }

    for (query, segment) in queries {
        dependencies.push(Dependency::Read(segment.index, TypeId::of::<Entity>()));
        dependencies.append(&mut Q::update(query));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        struct Time(f64);
        struct Physics;
        struct Position(f64, f64, f64);
        struct Velocity(f64, f64, f64);
        impl Resource for Time {}
        impl Resource for Physics {}
        impl Component for Position {}
        impl Component for Velocity {}

        fn physics(scheduler: &Scheduler) -> Scheduler {
            scheduler.system(|_: ()| {})
        }

        fn ui(scheduler: &Scheduler) -> Scheduler {
            scheduler.system(|_: ()| {})
        }

        let scheduler = Scheduler::default()
            .pipe(physics)
            .pipe(ui)
            .synchronize()
            .system(|_: (&Time,)| {})
            .system(|_: Group<Entity>| {})
            .system(|(group,): (Group<(Entity, &mut Position)>,)| {
                for _ in &group {}
                for _ in group {}
            })
            .system(|_: Group<(Entity, With<&Position>)>| {})
            .system(|_: Group<(Entity, (&Position, &Velocity))>| {})
            // Must be prevented since it breaks the invariants of Rust.
            // - will be allowed at compile-time, but will fail to initialize
            .system(|_: Group<(&mut Position, &mut Position)>| {})
            .system(|_: (&Time, &Physics)| {})
            .system(|_: (&Time, Group<Option<&Position>>)| {})
            .synchronize()
            .system(|_: (&Physics, Group<&Velocity>)| {})
            .system(
                |(time, (group1, group2)): (
                    &Time,
                    (Group<&mut Position>, Group<&mut Velocity>),
                )| {
                    group2.each(|velocity| {
                        velocity.0 += time.0;
                        velocity.1 += time.0;
                        velocity.2 += time.0;

                        group1.each(|position| {
                            position.0 += velocity.0;
                            position.1 += velocity.1;
                            position.2 += velocity.2;
                        });
                    });

                    for velocity in &group2 {
                        velocity.0 += time.0;
                        velocity.1 += time.0;
                        velocity.2 += time.0;

                        for position in &group1 {
                            position.0 += velocity.0;
                            position.1 += velocity.1;
                            position.2 += velocity.2;
                        }
                    }
                },
            )
            .system(|_: (Defer,)| {});

        let world = World::new();
        let mut runner = scheduler.schedule(&world).unwrap();
        loop {
            runner.run();
        }
    }
}

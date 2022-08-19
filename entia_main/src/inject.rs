use crate::{
    depend::{Conflict, Dependency, Scope},
    error::{Error, Result},
    identify,
    run::{as_mut, Run},
    tuples_with,
    world::World,
};
use entia_core::{utility::short_type_name, Change};
use std::{any::Any, marker::PhantomData, sync::Arc};

type Schedules = Vec<Box<dyn FnMut(&mut dyn Any, &mut World) -> (Vec<Run>, Vec<Run>)>>;

pub struct Context<'a, T, A> {
    identifier: usize,
    world: &'a mut World,
    adapt: A,
    schedules: &'a mut Schedules,
    _marker: PhantomData<T>,
}

pub struct Schedule<'a, T, A> {
    context: &'a mut Context<'a, T, A>,
    pre: &'a mut Vec<Run>,
    post: &'a mut Vec<Run>,
}

pub trait Adapt<T>: Clone + Send + Sync + 'static {
    fn adapt<'a>(&self, state: &'a mut dyn Any) -> Option<&'a mut T>;

    #[inline]
    fn map<U, F: Fn(&mut T) -> &mut U>(self, map: F) -> Map<T, U, Self, F>
    where
        Map<T, U, Self, F>: Adapt<U>,
    {
        Map(self, map, PhantomData)
    }

    #[inline]
    fn flat_map<U, F: Fn(&mut T) -> Option<&mut U>>(self, map: F) -> FlatMap<T, U, Self, F>
    where
        FlatMap<T, U, Self, F>: Adapt<U>,
    {
        FlatMap(self, map, PhantomData)
    }
}

pub struct Cast<T>(PhantomData<fn(T)>);
pub struct Map<T, U, A, F>(A, F, PhantomData<fn(T, U)>);
pub struct FlatMap<T, U, A, F>(A, F, PhantomData<fn(T, U)>);

pub unsafe trait Inject {
    type Input;
    type State: for<'a> Get<'a> + Send + Sync + 'static;

    fn initialize<A: Adapt<Self::State>>(
        input: Self::Input,
        context: Context<Self::State, A>,
    ) -> Result<Self::State>;

    fn depend(state: &Self::State) -> Vec<Dependency>;
}

pub trait Get<'a> {
    type Item;
    unsafe fn get(&'a mut self) -> Self::Item;
}

pub struct Injector<I: Inject> {
    identifier: usize,
    name: String,
    world: usize,
    version: usize,
    state: Arc<I::State>,
    schedules: Schedules,
    pre: Vec<Run>,
    post: Vec<Run>,
    dependencies: Vec<Dependency>,
    _marker: PhantomData<I>,
}

impl<T> Cast<T> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'a, T, A: Adapt<T>> Schedule<'a, T, A> {
    #[inline]
    pub fn context<'b>(&'b mut self) -> &'b mut Context<'a, T, A>
    where
        'a: 'b,
    {
        &mut self.context
    }

    pub fn pre<
        F: FnMut(&mut T) -> Result + Send + Sync + 'static,
        I: IntoIterator<Item = Dependency>,
    >(
        &mut self,
        run: F,
        dependencies: I,
    ) {
        self.pre.push(self.run(run, dependencies));
    }

    pub fn post<
        F: FnMut(&mut T) -> Result + Send + Sync + 'static,
        I: IntoIterator<Item = Dependency>,
    >(
        &mut self,
        run: F,
        dependencies: I,
    ) {
        self.post.push(self.run(run, dependencies));
    }

    fn run<
        F: FnMut(&mut T) -> Result + Send + Sync + 'static,
        I: IntoIterator<Item = Dependency>,
    >(
        &self,
        mut run: F,
        dependencies: I,
    ) -> Run {
        let adapt = self.context.adapt.clone();
        Run::new(
            move |state| match adapt.adapt(state) {
                Some(state) => run(state),
                None => Ok(()),
            },
            dependencies,
        )
    }
}

impl<T, A> Context<'_, T, A> {
    #[inline]
    pub fn world(&mut self) -> &mut World {
        self.world
    }

    #[inline]
    pub const fn identifier(&self) -> usize {
        self.identifier
    }
}

impl<T: 'static, A: Adapt<T>> Context<'_, T, A> {
    pub fn new<'a>(
        adapt: A,
        schedules: &'a mut Schedules,
        world: &'a mut World,
    ) -> Context<'a, T, A> {
        Context {
            identifier: identify(),
            world,
            adapt,
            schedules,
            _marker: PhantomData,
        }
    }

    pub fn schedule<F: FnMut(&mut T, Schedule<T, A>) + 'static>(&mut self, mut schedule: F) {
        let identifier = self.identifier;
        let adapt = self.adapt.clone();
        self.schedules.push(Box::new(move |state, world| {
            let mut pre = Vec::new();
            let mut post = Vec::new();
            if let Some(state) = adapt.adapt(state) {
                let mut schedules = Vec::new();
                let mut context = Context {
                    identifier,
                    world,
                    adapt: adapt.clone(),
                    schedules: &mut schedules,
                    _marker: PhantomData,
                };
                schedule(
                    state,
                    Schedule {
                        context: &mut context,
                        pre: &mut pre,
                        post: &mut post,
                    },
                );
                for schedule in schedules.iter_mut() {
                    let runs = schedule(state, world);
                    pre.extend(runs.0);
                    post.extend(runs.1);
                }
            }
            (pre, post)
        }));
    }

    pub fn own(&mut self) -> Context<'_, T, A> {
        Context {
            identifier: self.identifier,
            world: self.world,
            adapt: self.adapt.clone(),
            schedules: self.schedules,
            _marker: PhantomData,
        }
    }

    pub fn map<U, F: Fn(&mut T) -> &mut U>(&mut self, map: F) -> Context<'_, U, Map<T, U, A, F>>
    where
        Map<T, U, A, F>: Adapt<U>,
    {
        Context {
            identifier: self.identifier,
            world: self.world,
            adapt: self.adapt.clone().map(map),
            schedules: self.schedules,
            _marker: PhantomData,
        }
    }

    pub fn flat_map<U, F: Fn(&mut T) -> Option<&mut U>>(
        &mut self,
        map: F,
    ) -> Context<'_, U, FlatMap<T, U, A, F>>
    where
        FlatMap<T, U, A, F>: Adapt<U>,
    {
        Context {
            identifier: self.identifier,
            world: self.world,
            adapt: self.adapt.clone().flat_map(map),
            schedules: self.schedules,
            _marker: PhantomData,
        }
    }
}

impl<
        T: 'static,
        U: 'static,
        A: Adapt<T>,
        F: Fn(&mut T) -> &mut U + Clone + Send + Sync + 'static,
    > Adapt<U> for Map<T, U, A, F>
{
    #[inline]
    fn adapt<'a>(&self, state: &'a mut dyn Any) -> Option<&'a mut U> {
        Some(self.1(self.0.adapt(state)?))
    }
}

impl<
        T: 'static,
        U: 'static,
        A: Adapt<T>,
        F: Fn(&mut T) -> Option<&mut U> + Clone + Send + Sync + 'static,
    > Adapt<U> for FlatMap<T, U, A, F>
{
    #[inline]
    fn adapt<'a>(&self, state: &'a mut dyn Any) -> Option<&'a mut U> {
        self.1(self.0.adapt(state)?)
    }
}

impl<T: 'static> Adapt<T> for Cast<T> {
    #[inline]
    fn adapt<'a>(&self, state: &'a mut dyn Any) -> Option<&'a mut T> {
        state.downcast_mut()
    }
}

impl<T> Clone for Cast<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T, U, A: Clone, F: Clone> Clone for Map<T, U, A, F> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl<T, U, A: Clone, F: Clone> Clone for FlatMap<T, U, A, F> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), PhantomData)
    }
}

impl World {
    pub fn injector<I: Inject>(&mut self) -> Result<Injector<I>>
    where
        I::Input: Default,
    {
        self.injector_with(I::Input::default())
    }

    pub fn injector_with<I: Inject>(&mut self, input: I::Input) -> Result<Injector<I>> {
        let identifier = identify();
        let mut schedules = Vec::new();
        let context = Context {
            identifier,
            world: self,
            adapt: Cast::new(),
            schedules: &mut schedules,
            _marker: PhantomData,
        };
        let state = I::initialize(input, context)?;
        self.modify();
        Ok(Injector {
            identifier,
            name: short_type_name::<I>(),
            world: self.identifier(),
            version: 0,
            state: Arc::new(state),
            schedules,
            pre: Vec::new(),
            post: Vec::new(),
            dependencies: Vec::new(),
            _marker: PhantomData,
        })
    }
}

impl<I: Inject> Injector<I> {
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
                self.pre.clear();
                self.post.clear();

                let state = as_mut(&mut self.state);
                for schedule in self.schedules.iter_mut() {
                    let runs = schedule(state, world);
                    self.pre.extend(runs.0);
                    self.post.extend(runs.1);
                }

                for run in self.pre.iter().chain(self.post.iter()) {
                    conflict
                        .detect(Scope::Inner, run.dependencies(), true)
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
        let state = as_mut(&mut self.state);
        for run in self.pre.iter_mut() {
            run.run(state)?;
        }
        let value = run(unsafe { state.get() });
        for run in self.post.iter_mut() {
            run.run(state)?;
        }
        Ok(value)
    }
}

unsafe impl<I: Inject, const N: usize> Inject for [I; N] {
    type Input = [I::Input; N];
    type State = [I::State; N];

    fn initialize<A: Adapt<Self::State>>(
        input: Self::Input,
        mut context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        let mut items = [(); N].map(|_| None);
        for (i, input) in input.into_iter().enumerate() {
            items[i] = Some(I::initialize(
                input,
                context.map(move |state| &mut state[i]),
            )?);
        }
        Ok(items.map(Option::unwrap))
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for item in state {
            dependencies.append(&mut I::depend(&item));
        }
        dependencies
    }
}

impl<'a, T: Get<'a>, const N: usize> Get<'a> for [T; N] {
    type Item = [T::Item; N];

    #[inline]
    unsafe fn get(&'a mut self) -> Self::Item {
        let mut iterator = self.iter_mut();
        [(); N].map(|_| iterator.next().unwrap().get())
    }
}

unsafe impl<T> Inject for PhantomData<T> {
    type Input = <() as Inject>::Input;
    type State = <() as Inject>::State;

    fn initialize<A: Adapt<Self::State>>(
        input: Self::Input,
        context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        <()>::initialize(input, context)
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        <()>::depend(state)
    }
}

macro_rules! inject {
    ($n:ident $(, $p:ident, $t:ident, $i:tt)*) => {
        unsafe impl<$($t: Inject,)*> Inject for ($($t,)*) {
            type Input = ($($t::Input,)*);
            type State = ($($t::State,)*);

            fn initialize<A: Adapt<Self::State>>(
                ($($p,)*): Self::Input,
                mut _context: Context<Self::State, A>,
            ) -> Result<Self::State> {
                Ok(($($t::initialize($p, _context.map(|state| &mut state.$i))?,)*))
            }

            fn depend(($($p,)*): &Self::State) -> Vec<Dependency> {
                let mut _dependencies = Vec::new();
                $(_dependencies.append(&mut $t::depend($p));)*
                _dependencies
            }
        }

        impl<'a, $($t: Get<'a>,)*> Get<'a> for ($($t,)*) {
            type Item = ($($t::Item,)*);

            #[inline]
            unsafe fn get(&'a mut self) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.get(),)*)
            }
        }
    };
}

tuples_with!(inject);

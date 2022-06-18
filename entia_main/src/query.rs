use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    error::Result,
    filter::Filter,
    inject::{self, Get, Inject},
    item::{At, Context, Item},
    meta::Meta,
    resource::{Read, Write},
    world::World,
    Resource,
};
use std::{
    any::type_name,
    fmt::{self},
    iter,
    marker::PhantomData,
};

pub struct Query<'a, I: Item, F = ()> {
    pub(crate) inner: &'a Inner<I::State, F>,
    pub(crate) entities: &'a Entities,
    pub(crate) world: &'a World,
}

pub struct State<I: Item, F> {
    pub(crate) inner: Write<Inner<I::State, F>>,
    pub(crate) entities: Read<Entities>,
}

pub struct Inner<S, F> {
    pub(crate) segments: Vec<Option<usize>>,
    pub(crate) states: Vec<(S, usize)>,
    _marker: PhantomData<fn(F)>,
}

impl<S: Send + Sync + 'static, F: 'static> Resource for Inner<S, F> {
    fn initialize(_: &Meta, _: &mut World) -> Result<Self> {
        Ok(Self {
            segments: Vec::new(),
            states: Vec::new(),
            _marker: PhantomData,
        })
    }
}

impl<I: Item, F> fmt::Debug for Query<'_, I, F>
where
    for<'a> <I::State as At<'a>>::Ref: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(type_name::<Self>())?;
        f.debug_list().entries(self).finish()
    }
}

impl<I: Item, F: Filter + 'static> Inject for Query<'_, I, F>
where
    I::State: Send + Sync + 'static,
{
    type Input = ();
    type State = State<I, F>;

    fn initialize(_: Self::Input, mut context: inject::Context) -> Result<Self::State> {
        let inner = <Write<_> as Inject>::initialize(None, context.owned())?;
        let entities = <Read<_> as Inject>::initialize(None, context.owned())?;
        Ok(State { inner, entities })
    }

    fn update(state: &mut Self::State, mut context: inject::Context) -> Result {
        let identifier = context.identifier();
        let world = context.world();
        let inner = state.inner.as_mut();
        while let Some(segment) = world.segments().get(inner.segments.len()) {
            if F::filter(segment, world) {
                let segment = segment.index();
                if let Ok(item) = I::initialize(Context::new(identifier, segment, world)) {
                    inner.segments.push(Some(inner.states.len()));
                    inner.states.push((item, segment));
                    continue;
                }
            }
            inner.segments.push(None);
        }
        Ok(())
    }
}

impl<'a, I: Item, F: 'static> Get<'a> for State<I, F>
where
    I::State: Send + Sync + 'static,
{
    type Item = Query<'a, I, F>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Query {
            inner: self.inner.as_ref(),
            entities: self.entities.as_ref(),
            world,
        }
    }
}

unsafe impl<I: Item, F: 'static> Depend for State<I, F>
where
    I::State: Send + Sync + 'static,
{
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.entities.depend(world);
        let inner = self.inner.as_ref();
        for (item, segment) in inner.states.iter() {
            dependencies.push(Dependency::read::<Entity>().segment(*segment));
            dependencies.append(&mut item.depend(world));
        }
        dependencies
    }
}

macro_rules! iterator {
    ($t:ident, $at:ident, $iter:ident, $each:ident, $get:ident, $item:ident, [$($mut:tt)?]) => {
        impl<'a, I: Item, F> Query<'a, I, F> {
            #[inline]
            pub fn $iter<'b>(&'b $($mut)? self) -> $t<'a, 'b, I, F> where 'a: 'b {
                self.into_iter()
            }

            pub fn $each(& $($mut)? self, mut each: impl FnMut(<I::State as At<'a>>::$item)) {
                let segments = self.world.segments();
                for (state, segment) in &self.inner.states {
                    // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks..
                    let segment = unsafe { segments.get_unchecked(*segment) };
                    let $($mut)? state = state.get(self.world);

                    for i in 0..segment.count() {
                        // if let Some(item) = I::State::$at(& $($mut)? state, i) {
                        //     each(item);
                        // }
                        each(I::State::$at(& $($mut)? state, i));
                    }
                }
            }

            pub fn $get(& $($mut)? self, entity: impl Into<Entity>) -> Option<<I::State as At<'a>>::$item> {
                let datum = self.entities.get_datum(entity.into())?;
                let index = self.inner.segments[datum.segment_index as usize]?;
                let (state, _) = &self.inner.states[index];
                let $($mut)? state = state.get(self.world);
                Some(<I::State as At<'_>>::$at(& $($mut)? state, datum.store_index as usize))
            }
        }

        pub struct $t<'a, 'b, I: Item, F> {
            index: usize,
            count: usize,
            segment: usize,
            state: Option<<I::State as At<'a>>::State>,
            query: &'b $($mut)? Query<'a, I, F>,
        }

        impl<'a: 'b, 'b, I: Item, F> IntoIterator for &'b $($mut)? Query<'a, I, F> {
            type Item = <I::State as At<'a>>::$item;
            type IntoIter = $t<'a, 'b, I, F>;

            #[inline]
            fn into_iter(self) -> Self::IntoIter {
                $t {
                    index: 0,
                    count: 0,
                    state: None,
                    segment: 0,
                    query: self,
               }
            }
        }

        impl<'a: 'b, 'b, I: Item, F> iter::Iterator for $t<'a, 'b, I, F> {
            type Item = <I::State as At<'a>>::$item;

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    if self.index < self.count {
                        // SAFETY: In order to pass the 'self.index < self.count' check, 'self.state' had to be set.
                        // This holds as long as 'self.count' was initialized to 0.
                        let state = unsafe { self.state.as_mut().unwrap_unchecked() };
                        let item = I::State::$at(state, self.index);
                        self.index += 1;
                        // if let Some(item) = item { break Some(item); }
                        break Some(item);
                    }
                    else if let Some((state, segment)) = self.query.inner.states.get(self.segment) {
                        let segments = self.query.world.segments();
                        // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks..
                        let segment = unsafe { segments.get_unchecked(*segment) };
                        self.segment += 1;
                        self.state = Some(state.get(self.query.world));
                        self.index = 0;
                        self.count = segment.count();
                    }
                    else {
                        break None;
                    }
                }
            }
        }
    };
}
iterator!(RefIterator, at, iter, each, get, Ref, []);
iterator!(MutIterator, at_mut, iter_mut, each_mut, get_mut, Mut, [mut]);

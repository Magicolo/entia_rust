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
    ops::RangeFull,
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

impl<'a, I: Item, F> fmt::Debug for Query<'a, I, F>
where
    <&'a Self as IntoIterator>::Item: fmt::Debug,
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
    ($t:ident, $at:ident, $chunks:ident, $iter:ident, $each:ident, $get:ident, $item:ident, [$($mut:tt)?]) => {
        impl<'a, I: Item, F> Query<'a, I, F> {
            #[inline]
            pub fn $iter<'b>(&'b $($mut)? self) -> $t<'a, 'b, I, F> where 'a: 'b {
                self.into_iter()
            }

            pub fn $chunks(& $($mut)? self) -> impl Iterator<Item = <I::State as At<'_, RangeFull>>::$item>
            where
                I::State: for<'b> At<'b, RangeFull>
            {
                let segments = self.world.segments();
                self.inner.states.iter().filter_map(|(state, segment)| {
                    // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks.
                    let $($mut)? state = state.get(unsafe { segments.get_unchecked(*segment) })?;
                    Some(unsafe { I::State::$at(& $($mut)? state, ..) })
                })
            }

            pub fn $each<E: FnMut(<I::State as At<'_>>::$item)>(& $($mut)? self, mut each: E) {
                let segments = self.world.segments();
                for (state, segment) in &self.inner.states {
                    // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks.
                    let segment = unsafe { segments.get_unchecked(*segment) };
                    if let Some($($mut)? state) = state.get(segment) {
                        for i in 0..segment.count() {
                            // SAFETY: The safety requirements of 'at_unchecked/_mut' guarantee that is it safe to provide an index
                            // within '0..segment.count()'.
                            each(unsafe { I::State::$at(& $($mut)? state, i) });
                        }
                    }
                }
            }

            pub fn $get<E: Into<Entity>>(& $($mut)? self, entity: E) -> Option<<I::State as At<'_>>::$item> {
                let datum = self.entities.get_datum(entity.into())?;
                let index = self.inner.segments[datum.segment_index as usize]?;
                let (state, segment) = &self.inner.states[index];
                // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks.
                let segment = unsafe { self.world.segments().get_unchecked(*segment) };
                let $($mut)? state = state.get(segment)?;
                // SAFETY: 'entities.get_datum' validates that the 'store_index' is valid and is therefore safe to use.
                Some(unsafe { I::State::$at(& $($mut)? state, datum.store_index as usize) })
            }
        }

        pub struct $t<'a, 'b, I: Item, F> {
            index: usize,
            count: usize,
            segment: usize,
            query: &'b $($mut)? Query<'a, I, F>,
            state: Option<<I::State as At<'a>>::State>,
        }

        impl<'a: 'b, 'b, I: Item, F> IntoIterator for &'b $($mut)? Query<'a, I, F> {
            type Item = <I::State as At<'a>>::$item;
            type IntoIter = $t<'a, 'b, I, F>;

            #[inline]
            fn into_iter(self) -> Self::IntoIter {
                $t {
                    index: 0,
                    count: 0,
                    segment: 0,
                    query: self,
                    state: None,
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
                        let $($mut)? state = unsafe { self.state.as_mut().unwrap_unchecked() };
                        // SAFETY: 'self.index' has been checked to be in range.
                        let item = unsafe { I::State::$at(& $($mut)? state, self.index) };
                        self.index += 1;
                        break Some(item);
                    } else {
                        let (state, segment) = self.query.inner.states.get(self.segment)?;
                        let segments = self.query.world.segments();
                        // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks..
                        let segment = unsafe { segments.get_unchecked(*segment) };
                        self.segment += 1;
                        // The segment may be skipped.
                        if let Some(state) = state.get(segment) {
                            self.index = 0;
                            self.count = segment.count();
                            self.state = Some(state);
                        }
                    }
                }
            }
        }
    };
}
iterator!(IteratorRef, at_ref, chunks, iter, each, get, Ref, []);
iterator!(IteratorMut, at_mut, chunks_mut, iter_mut, each_mut, get_mut, Mut, [mut]);

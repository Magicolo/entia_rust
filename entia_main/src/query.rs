use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    error::Result,
    filter::Filter,
    inject::{Get, Inject},
    item::{At, Item},
    resource::{Read, Write},
    segment::{Segment, Segments},
    world::World,
};
use std::{
    any::type_name,
    fmt::{self},
    iter,
    marker::PhantomData,
    ops::{DerefMut, RangeFull},
};

pub struct Query<'a, I: Item, F = ()> {
    pub(crate) inner: &'a Inner<I::State, F>,
    pub(crate) entities: &'a Entities,
    pub(crate) segments: &'a Segments,
}

pub struct State<I: Item, F> {
    pub(crate) identifier: usize,
    pub(crate) inner: Write<Inner<I::State, F>>,
    pub(crate) segments: Read<Segments>,
    pub(crate) entities: Read<Entities>,
}

pub struct Inner<S, F> {
    pub(crate) segments: Vec<Option<usize>>,
    pub(crate) states: Vec<(S, usize)>,
    _marker: PhantomData<fn(F)>,
}

impl<S: Send + Sync + 'static, F: 'static> Default for Inner<S, F> {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            states: Vec::new(),
            _marker: PhantomData,
        }
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

    fn initialize(_: Self::Input, identifier: usize, world: &mut World) -> Result<Self::State> {
        let inner = <Write<_> as Inject>::initialize(None, identifier, world)?;
        let segments = <Read<_> as Inject>::initialize(None, identifier, world)?;
        let entities = <Read<_> as Inject>::initialize(None, identifier, world)?;
        Ok(State {
            identifier,
            inner,
            segments,
            entities,
        })
    }

    fn update(state: &mut Self::State, world: &mut World) -> Result {
        let inner = state.inner.deref_mut();
        while let Some(segment) = state.segments[..].get(inner.segments.len()) {
            if F::filter(segment) {
                if let Ok(item) = I::initialize(state.identifier, segment, world) {
                    inner.segments.push(Some(inner.states.len()));
                    inner.states.push((item, segment.index()));
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
    unsafe fn get(&'a mut self) -> Self::Item {
        Query {
            inner: &self.inner,
            entities: &self.entities,
            segments: &self.segments,
        }
    }
}

unsafe impl<I: Item, F: 'static> Depend for State<I, F>
where
    I::State: Send + Sync + 'static,
{
    fn depend(&self) -> Vec<Dependency> {
        let mut dependencies = self.entities.depend();
        for (item, segment) in self.inner.states.iter() {
            dependencies.push(Dependency::read::<Segment>(
                self.segments[*segment].identifier(),
            ));
            dependencies.append(&mut item.depend());
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
                self.inner.states.iter().filter_map(|(state, segment)| {
                    // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks.
                    let $($mut)? state = state.get(unsafe { self.segments.get_unchecked(*segment) })?;
                    Some(unsafe { I::State::$at(& $($mut)? state, ..) })
                })
            }

            pub fn $each<E: FnMut(<I::State as At<'a>>::$item)>(& $($mut)? self, mut each: E) {
                for (state, segment) in &self.inner.states {
                    // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks.
                    let segment = unsafe { self.segments.get_unchecked(*segment) };
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
                let segment = unsafe { self.segments.get_unchecked(*segment) };
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
                        // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks..
                        let segment = unsafe { self.query.segments.get_unchecked(*segment) };
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

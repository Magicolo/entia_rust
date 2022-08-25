use crate::{
    depend::Dependency,
    entities::Entities,
    entity::Entity,
    error::Result,
    filter::Filter,
    inject::{Adapt, Context, Get, Inject},
    item::{At, Item},
    resource::Resource,
    resource::{Read, Write},
    segment::Segments,
};
use std::{
    any::type_name,
    fmt::{self},
    marker::PhantomData,
    ops::{DerefMut, RangeFull},
};

pub struct Query<'a, I: Item, F = ()> {
    pub(crate) inner: &'a Inner<I::State, F>,
    pub(crate) entities: &'a Entities,
    pub(crate) segments: &'a Segments,
}

pub struct State<I: Item, F> {
    pub(crate) inner: Write<Inner<I::State, F>>,
    pub(crate) segments: Read<Segments>,
    pub(crate) entities: Read<Entities>,
}

pub struct Inner<S, F> {
    pub(crate) segments: Vec<usize>,
    pub(crate) states: Vec<(S, usize)>,
    _marker: PhantomData<fn(F)>,
}

impl<S: Send + Sync + 'static, F: 'static> Resource for Inner<S, F> {}

impl<S, F: 'static> Default for Inner<S, F> {
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

unsafe impl<I: Item + 'static, F: Filter + 'static> Inject for Query<'_, I, F> {
    type Input = ();
    type State = State<I, F>;

    fn initialize<A: Adapt<Self::State>>(
        _: Self::Input,
        mut context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        let inner = Write::initialize(None, context.map(|state| &mut state.inner))?;
        let segments = Read::initialize(None, context.map(|state| &mut state.segments))?;
        let entities = Read::initialize(None, context.map(|state| &mut state.entities))?;
        context.schedule(|state, mut schedule| {
            let inner = state.inner.deref_mut();
            while let Some(segment) = state.segments[..].get(inner.segments.len()) {
                if F::filter(segment) {
                    let index = inner.states.len();
                    if let Ok(item) = I::initialize(
                        segment,
                        schedule
                            .context()
                            .map(move |state| &mut state.inner.states[index].0),
                    ) {
                        inner.segments.push(index);
                        inner.states.push((item, segment.index()));
                        continue;
                    }
                }
                inner.segments.push(usize::MAX);
            }
        });
        Ok(State {
            inner,
            segments,
            entities,
        })
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        let mut dependencies = Read::depend(&state.inner.read());
        dependencies.extend(Read::depend(&state.entities));
        dependencies.extend(Read::depend(&state.segments));
        for (item, segment) in state.inner.states.iter() {
            dependencies.push(Dependency::read_at(state.segments[*segment].identifier()));
            dependencies.extend(I::depend(item));
        }
        dependencies
    }
}

impl<'a, I: Item, F: 'static> Get<'a> for State<I, F> {
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

macro_rules! iter {
    ($s:expr, $at:ident, [$($mut:tt)?]) => {{
        let segments = $s.segments;
        $s.inner.states.iter().flat_map(move |(state, segment)| {
            // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks..
            let segment = unsafe { segments.get_unchecked(*segment) };
            state.get(segment).into_iter().flat_map(|$($mut)? state| {
                (0..segment.count()).map(move |index| unsafe { I::State::$at(& $($mut)? state, index) })
            })
        })
    }};
}

macro_rules! iterator {
    ($at:ident, $chunks:ident, $iter:ident, $each:ident, $get:ident, $item:ident, [$($mut:tt)?]) => {
        impl<'a, I: Item, F> Query<'a, I, F> {
            #[inline]
            pub fn $iter(& $($mut)? self) -> impl DoubleEndedIterator<Item = <I::State as At<'a>>::$item> {
                iter!(self, $at, [$($mut)?])
            }

            pub fn $chunks(& $($mut)? self) -> impl DoubleEndedIterator<Item = <I::State as At<RangeFull>>::$item>
            where
                I::State: for<'b> At<'b, RangeFull>
            {
                self.inner.states.iter().filter_map(|(state, segment)| {
                    // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks.
                    let $($mut)? state = state.get(unsafe { self.segments.get_unchecked(*segment) })?;
                    Some(unsafe { I::State::$at(& $($mut)? state, ..) })
                })
            }

            pub fn $each<E: FnMut(<I::State as At>::$item)>(& $($mut)? self, mut each: E) {
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
                let index = self.inner.segments[datum.segment as usize];
                let (state, segment) = &self.inner.states.get(index)?;
                // SAFETY: The 'segment' index has already been checked to be in range and the 'world.segments' vector never shrinks.
                let segment = unsafe { self.segments.get_unchecked(*segment) };
                let $($mut)? state = state.get(segment)?;
                // SAFETY: 'entities.get_datum' validates that the 'store_index' is valid and is therefore safe to use.
                Some(unsafe { I::State::$at(& $($mut)? state, datum.store as usize) })
            }
        }

        impl<'a, I: Item, F> IntoIterator for & $($mut)? Query<'a, I, F> {
            type Item = <I::State as At<'a>>::$item;
            type IntoIter = impl DoubleEndedIterator<Item = <I::State as At<'a>>::$item>;

            #[inline]
            fn into_iter(self) -> Self::IntoIter {
                iter!(self, $at, [$($mut)?])
            }
        }
    };
}
iterator!(at_ref, chunks, iter, each, get, Ref, []);
iterator!(at_mut, chunks_mut, iter_mut, each_mut, get_mut, Mut, [mut]);

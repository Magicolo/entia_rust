use self::{filter::*, item::*};
use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    inject::{self, Get, Inject},
    read::{self, Read},
    world::{segment::Segment, World},
    write::{self, Write},
    Result,
};
use std::{
    any::type_name,
    fmt::{self},
    iter,
    marker::PhantomData,
};

pub struct Query<'a, I: Item, F: Filter = ()> {
    pub(crate) inner: &'a Inner<I, F>,
    pub(crate) entities: &'a Entities,
    pub(crate) world: &'a World,
}

pub struct State<I: Item, F: Filter> {
    pub(crate) inner: write::State<Inner<I, F>>,
    pub(crate) entities: read::State<Entities>,
}

pub struct Iterator<'a, 'b, I: Item, F: Filter> {
    index: usize,
    segment: usize,
    query: &'b Query<'a, I, F>,
}

pub(crate) struct Inner<I: Item, F: Filter> {
    pub(crate) segments: Vec<Option<usize>>,
    pub(crate) states: Vec<(I::State, usize)>,
    _marker: PhantomData<fn(F)>,
}

impl<I: Item, F: Filter> Default for Inner<I, F> {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            states: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<'a, I: Item, F: Filter> Query<'a, I, F> {
    pub fn len(&self) -> usize {
        self.inner
            .states
            .iter()
            .map(|&(_, segment)| self.world.segments[segment].count)
            .sum()
    }

    #[inline]
    pub fn each(&'a self, mut each: impl FnMut(<I::State as At<'a>>::Item)) {
        for (state, segment) in &self.inner.states {
            let segment = &self.world.segments[*segment];
            for i in 0..segment.count {
                each(state.at(i, self.world));
            }
        }
    }

    pub fn get(&self, entity: Entity) -> Option<<I::State as At<'_>>::Item> {
        let datum = self.entities.get_datum(entity)?;
        let index = self.inner.segments[datum.segment_index as usize]?;
        let (state, _) = &self.inner.states[index];
        Some(state.at(datum.store_index as usize, self.world))
    }
}

impl<'a, 'b: 'a, I: Item, F: Filter> IntoIterator for &'b Query<'a, I, F> {
    type Item = <I::State as At<'a>>::Item;
    type IntoIter = Iterator<'a, 'b, I, F>;

    fn into_iter(self) -> Self::IntoIter {
        Iterator {
            index: 0,
            segment: 0,
            query: self,
        }
    }
}

impl<'a, 'b: 'a, I: Item, F: Filter> iter::Iterator for Iterator<'a, 'b, I, F> {
    type Item = <I::State as At<'a>>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((item, segment)) = self.query.inner.states.get(self.segment) {
            let segment = &self.query.world.segments[*segment];
            if self.index < segment.count {
                let item = item.at(self.index, self.query.world);
                self.index += 1;
                return Some(item);
            } else {
                self.segment += 1;
                self.index = 0;
            }
        }
        None
    }
}
impl<I: Item, F: Filter> fmt::Debug for Query<'_, I, F>
where
    for<'a> <I::State as At<'a>>::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(type_name::<Self>())?;
        f.debug_list().entries(self).finish()
    }
}

impl<'a, I: Item + 'static, F: Filter + 'static> Inject for Query<'a, I, F>
where
    I::State: Send + Sync,
{
    type Input = ();
    type State = State<I, F>;

    fn initialize(_: Self::Input, mut context: inject::Context) -> Result<Self::State> {
        let inner = <Write<Inner<I, F>> as Inject>::initialize(None, context.owned())?;
        let entities = <Read<Entities> as Inject>::initialize(None, context.owned())?;
        Ok(State { inner, entities })
    }

    fn update(state: &mut Self::State, mut context: inject::Context) {
        let identifier = context.identifier();
        let world = context.world();
        let inner = state.inner.as_mut();
        while let Some(segment) = world.segments.get(inner.segments.len()) {
            if F::filter(segment, world) {
                let segment = segment.index;
                if let Ok(item) = I::initialize(Context::new(identifier, segment, world)) {
                    inner.segments.push(Some(inner.states.len()));
                    inner.states.push((item, segment));
                    continue;
                }
            }
            inner.segments.push(None);
        }

        for (state, segment) in inner.states.iter_mut() {
            let context = Context::new(context.identifier(), *segment, context.world());
            I::update(state, context);
        }
    }
}

impl<I: Item, F: Filter> Clone for State<I, F> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            entities: self.entities.clone(),
        }
    }
}

impl<'a, I: Item + 'static, F: Filter + 'static> Get<'a> for State<I, F> {
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

unsafe impl<I: Item + 'static, F: Filter + 'static> Depend for State<I, F> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.entities.depend(world);
        let inner = self.inner.as_ref();
        for (item, segment) in inner.states.iter() {
            dependencies.push(Dependency::read::<Entity>().at(*segment));
            dependencies.append(&mut item.depend(world));
        }
        dependencies
    }
}

pub mod item {
    use super::*;

    pub struct Context<'a> {
        identifier: usize,
        segment: usize,
        world: &'a mut World,
    }

    pub trait Item {
        type State: for<'a> At<'a> + Depend;
        fn initialize(context: Context) -> Result<Self::State>;
        #[inline]
        fn update(_: &mut Self::State, _: Context) {}
    }

    pub trait At<'a> {
        type Item;
        fn at(&'a self, index: usize, world: &'a World) -> Self::Item;
    }

    impl<'a> Context<'a> {
        pub fn new(identifier: usize, segment: usize, world: &'a mut World) -> Self {
            Self {
                identifier,
                segment,
                world,
            }
        }

        pub fn identifier(&self) -> usize {
            self.identifier
        }

        pub fn segment(&self) -> &Segment {
            &self.world.segments[self.segment]
        }

        pub fn world(&mut self) -> &mut World {
            self.world
        }

        pub fn owned(&mut self) -> Context {
            self.with(self.segment)
        }

        pub fn with(&mut self, segment: usize) -> Context {
            Context::new(self.identifier, segment, self.world)
        }
    }

    impl<'a> Into<inject::Context<'a>> for Context<'a> {
        fn into(self) -> inject::Context<'a> {
            inject::Context::new(self.identifier, self.world)
        }
    }

    impl<I: Item> Item for Option<I> {
        type State = Option<I::State>;

        fn initialize(context: Context) -> Result<Self::State> {
            Ok(I::initialize(context).ok())
        }

        #[inline]
        fn update(state: &mut Self::State, context: Context) {
            if let Some(state) = state {
                I::update(state, context);
            }
        }
    }

    impl<'a, A: At<'a>> At<'a> for Option<A> {
        type Item = Option<A::Item>;

        #[inline]
        fn at(&'a self, index: usize, world: &'a World) -> Self::Item {
            Some(self.as_ref()?.at(index, world))
        }
    }

    impl<T> Item for PhantomData<T> {
        type State = <() as Item>::State;
        fn initialize(context: Context) -> Result<Self::State> {
            <() as Item>::initialize(context)
        }
    }

    impl<'a, T> At<'a> for PhantomData<T> {
        type Item = <() as At<'a>>::Item;
        #[inline]
        fn at(&'a self, index: usize, world: &'a World) -> Self::Item {
            ().at(index, world)
        }
    }

    macro_rules! item {
        ($($p:ident, $t:ident),*) => {
            impl<$($t: Item,)*> Item for ($($t,)*) {
                type State = ($($t::State,)*);

                fn initialize(mut _context: Context) -> Result<Self::State> {
                    Ok(($($t::initialize(_context.owned())?,)*))
                }

                fn update(($($p,)*): &mut Self::State, mut _context: Context) {
                    $($t::update($p, _context.owned());)*
                }
            }

            impl<'a, $($t: At<'a>,)*> At<'a> for ($($t,)*) {
                type Item = ($($t::Item,)*);

                #[inline]
                fn at(&'a self, _index: usize, _world: &'a World) -> Self::Item {
                    let ($($p,)*) = self;
                    ($($p.at(_index, _world),)*)
                }
            }
        };
    }

    entia_macro::recurse_16!(item);
}

pub mod filter {
    use super::*;

    pub trait Filter {
        fn filter(segment: &Segment, world: &World) -> bool;
    }

    #[derive(Copy, Clone, Debug)]
    pub struct Has<T>(PhantomData<T>);
    #[derive(Copy, Clone, Debug)]
    pub struct Not<F>(PhantomData<F>);

    impl<T: Send + Sync + 'static> Filter for Has<T> {
        fn filter(segment: &Segment, world: &World) -> bool {
            if let Ok(meta) = world.get_meta::<T>() {
                segment.store(&meta).is_ok()
            } else {
                false
            }
        }
    }

    impl<F: Filter> Filter for Not<F> {
        fn filter(segment: &Segment, world: &World) -> bool {
            !F::filter(segment, world)
        }
    }

    impl<T> Filter for PhantomData<T> {
        fn filter(segment: &Segment, world: &World) -> bool {
            <() as Filter>::filter(segment, world)
        }
    }

    macro_rules! filter {
        ($($t:ident, $p:ident),*) => {
            impl<$($t: Filter,)*> Filter for ($($t,)*) {
                fn filter(_segment: &Segment, _world: &World) -> bool {
                    $($t::filter(_segment, _world) &&)* true
                }
            }
        };
    }

    entia_macro::recurse_16!(filter);
}

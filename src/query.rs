use self::{filter::*, item::*};
use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    inject::{Get, Inject, InjectContext},
    resource::Resource,
    segment::Segment,
    world::World,
    write::{self, Write},
};
use std::{any::TypeId, iter, marker::PhantomData};

pub struct Query<'a, I: item::Item, F: Filter = ()> {
    pub(crate) inner: &'a Inner<I, F>,
    pub(crate) entities: &'a Entities,
    pub(crate) world: &'a World,
}

pub struct State<I: Item, F: Filter> {
    pub(crate) inner: write::State<Inner<I, F>>,
    pub(crate) entities: write::State<Entities>,
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

impl<I: Item + 'static, F: Filter> Resource for Inner<I, F> {}

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
            let count = segment.count;
            for i in 0..count {
                each(state.at(i, self.world));
            }
        }
    }

    pub fn get(&self, entity: Entity) -> Option<<I::State as At<'_>>::Item> {
        match self.entities.get_datum(entity) {
            Some(datum) => {
                let index = datum.index() as usize;
                let segment = datum.segment() as usize;
                for state in &self.inner.states {
                    if state.1 == segment {
                        return Some(state.0.at(index, self.world));
                    }
                }
                None
            }
            None => None,
        }
    }

    pub fn has(&self, entity: Entity) -> bool {
        self.entities
            .get_datum(entity)
            .and_then(|datum| self.inner.segments[datum.segment() as usize])
            .is_some()
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

unsafe impl<'a, I: Item + 'static, F: Filter> Inject for Query<'a, I, F> {
    type Input = ();
    type State = State<I, F>;

    fn initialize(_: Self::Input, mut context: InjectContext) -> Option<Self::State> {
        let inner = <Write<Inner<I, F>> as Inject>::initialize(None, context.owned())?;
        let entities = <Write<Entities> as Inject>::initialize(None, context.owned())?;
        Some(State { inner, entities })
    }

    fn update(state: &mut Self::State, mut context: InjectContext) {
        let identifier = context.identifier();
        let world = context.world();
        let inner = state.inner.as_mut();
        while let Some(segment) = world.segments.get(inner.segments.len()) {
            if F::filter(segment, world) {
                let segment = segment.index;
                if let Some(item) = I::initialize(ItemContext::new(identifier, segment, world)) {
                    inner.segments.push(Some(segment));
                    inner.states.push((item, segment));
                    continue;
                }
            }
            inner.segments.push(None);
        }

        for (state, segment) in inner.states.iter_mut() {
            let context = ItemContext::new(context.identifier(), *segment, context.world());
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

impl<'a, I: Item + 'static, F: Filter> Get<'a> for State<I, F> {
    type Item = Query<'a, I, F>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Query {
            inner: self.inner.get(world),
            entities: self.entities.get(world),
            world,
        }
    }
}

unsafe impl<I: Item + 'static, F: Filter> Depend for State<I, F> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.entities.depend(world);
        let inner = self.inner.as_ref();
        for (item, segment) in inner.states.iter() {
            dependencies.push(Dependency::Read(*segment, TypeId::of::<Entity>()));
            dependencies.append(&mut item.depend(world));
        }
        dependencies
    }
}

pub mod item {
    use super::*;

    pub struct ItemContext<'a> {
        identifier: usize,
        segment: usize,
        world: &'a mut World,
    }

    pub unsafe trait Item: Send {
        type State: for<'a> At<'a> + Depend + Send + 'static;
        fn initialize(context: ItemContext) -> Option<Self::State>;
        #[inline]
        fn update(_: &mut Self::State, _: ItemContext) {}
    }

    pub trait At<'a> {
        type Item;
        fn at(&'a self, index: usize, world: &'a World) -> Self::Item;
    }

    impl<'a> ItemContext<'a> {
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

        pub fn owned(&mut self) -> ItemContext {
            self.with(self.segment)
        }

        pub fn with(&mut self, segment: usize) -> ItemContext {
            ItemContext::new(self.identifier, segment, self.world)
        }
    }

    impl<'a> Into<InjectContext<'a>> for ItemContext<'a> {
        fn into(self) -> InjectContext<'a> {
            InjectContext::new(self.identifier, self.world)
        }
    }

    unsafe impl<I: Item> Item for Option<I> {
        type State = Option<I::State>;

        fn initialize(context: ItemContext) -> Option<Self::State> {
            Some(I::initialize(context))
        }

        #[inline]
        fn update(state: &mut Self::State, context: ItemContext) {
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

    macro_rules! item {
        ($($p:ident, $t:ident),*) => {
            unsafe impl<$($t: Item,)*> Item for ($($t,)*) {
                type State = ($($t::State,)*);

                fn initialize(mut _context: ItemContext) -> Option<Self::State> {
                    Some(($($t::initialize(_context.owned())?,)*))
                }

                fn update(($($p,)*): &mut Self::State, mut _context: ItemContext) {
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

    entia_macro::recurse_32!(item);
}

pub mod filter {
    use super::*;

    pub trait Filter: Send + 'static {
        fn filter(segment: &Segment, world: &World) -> bool;
    }

    pub struct Not<F: Filter>(PhantomData<F>);

    impl<F: Filter> Filter for Not<F> {
        fn filter(segment: &Segment, world: &World) -> bool {
            !F::filter(segment, world)
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

    entia_macro::recurse_32!(filter);
}

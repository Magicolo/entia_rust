use self::{filter::*, item::*};
use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    error::Result,
    inject::{self, Get, Inject},
    read::Read,
    recurse,
    world::{segment::Segment, World},
    write::Write,
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

impl<S, F> Default for Inner<S, F> {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            states: Vec::new(),
            _marker: PhantomData,
        }
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
        while let Some(segment) = world.segments.get(inner.segments.len()) {
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
    I::State: 'static,
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
    I::State: 'static,
{
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

macro_rules! iterator {
    ($t:ident, $at:ident, $iter:ident, $each:ident, $get:ident, $item:ident, [$($mut:tt)?]) => {
        impl<'a, I: Item, F> Query<'a, I, F> {
            #[inline]
            pub fn $iter<'b>(&'b $($mut)? self) -> $t<'a, 'b, I, F> where 'a: 'b {
                self.into_iter()
            }

            pub fn $each(& $($mut)? self, mut each: impl FnMut(<I::State as At<'a>>::$item)) {
                for (state, segment) in &self.inner.states {
                    let segment = &self.world.segments[*segment];
                    let $($mut)? state = state.get(self.world);
                    for i in 0..segment.count() {
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
                if self.index < self.count {
                    if let Some(state) = & $($mut)? self.state {
                        let item = I::State::$at(state, self.index);
                        self.index += 1;
                        return Some(item);
                    }
                }

                while let Some((state, segment)) = self.query.inner.states.get(self.segment) {
                    let segment = &self.query.world.segments[*segment];
                    self.segment += 1;
                    match segment.count() {
                        0 => continue,
                        1 => {
                            let $($mut)? state = state.get(self.query.world);
                            return Some(I::State::$at(& $($mut)? state, 0));
                        }
                        count => {
                            let $($mut)? state = state.get(self.query.world);
                            let item = I::State::$at(& $($mut)? state, 0);
                            self.index = 1;
                            self.state = Some(state);
                            self.count = count;
                            return Some(item);
                        }
                    }
                }

                None
            }
        }
    };
}
iterator!(RefIterator, at, iter, each, get, Ref, []);
iterator!(MutIterator, at_mut, iter_mut, each_mut, get_mut, Mut, [mut]);

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
    }

    pub trait At<'a> {
        type State;
        type Ref;
        type Mut;
        fn get(&'a self, world: &'a World) -> Self::State;
        fn at(state: &Self::State, index: usize) -> Self::Ref;
        fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut;
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
    }

    impl<'a, A: At<'a>> At<'a> for Option<A> {
        type State = Option<A::State>;
        type Ref = Option<A::Ref>;
        type Mut = Option<A::Mut>;

        #[inline]
        fn get(&'a self, world: &'a World) -> Self::State {
            Some(self.as_ref()?.get(world))
        }

        #[inline]
        fn at(state: &Self::State, index: usize) -> Self::Ref {
            Some(A::at(state.as_ref()?, index))
        }

        #[inline]
        fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut {
            Some(A::at_mut(state.as_mut()?, index))
        }
    }

    impl<T> Item for PhantomData<T> {
        type State = <() as Item>::State;
        fn initialize(context: Context) -> Result<Self::State> {
            <() as Item>::initialize(context)
        }
    }

    impl<'a, T> At<'a> for PhantomData<T> {
        type State = <() as At<'a>>::State;
        type Ref = <() as At<'a>>::Ref;
        type Mut = <() as At<'a>>::Mut;

        #[inline]
        fn get(&'a self, world: &'a World) -> Self::State {
            ().get(world)
        }

        #[inline]
        fn at(state: &Self::State, index: usize) -> Self::Ref {
            <()>::at(state, index)
        }

        #[inline]
        fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut {
            <()>::at_mut(state, index)
        }
    }

    macro_rules! item {
        ($($p:ident, $t:ident),*) => {
            impl<$($t: Item,)*> Item for ($($t,)*) {
                type State = ($($t::State,)*);

                fn initialize(mut _context: Context) -> Result<Self::State> {
                    Ok(($($t::initialize(_context.owned())?,)*))
                }
            }

            impl<'a, $($t: At<'a>,)*> At<'a> for ($($t,)*) {
                type State = ($($t::State,)*);
                type Ref = ($($t::Ref,)*);
                type Mut = ($($t::Mut,)*);

                #[inline]
                fn get(&'a self, _world: &'a World) -> Self::State {
                    let ($($p,)*) = self;
                    ($($p.get(_world),)*)
                }

                #[inline]
                fn at(_state: &Self::State, _index: usize) -> Self::Ref {
                    let ($($p,)*) = _state;
                    ($($t::at($p, _index),)*)
                }

                #[inline]
                fn at_mut(_state: &mut Self::State, _index: usize) -> Self::Mut {
                    let ($($p,)*) = _state;
                    ($($t::at_mut($p, _index),)*)
                }
            }
        };
    }

    recurse!(item);
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
                segment.component_types().contains(&meta.identifier)
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

    recurse!(filter);
}

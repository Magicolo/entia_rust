use crate::{
    depend::{Depend, Dependency},
    entities::{Datum, Entities},
    entity::Entity,
    error::Result,
    inject::Inject,
    query::{
        self,
        filter::Filter,
        item::{At, Context, Item},
        Query,
    },
    read::Read,
    world::World,
};
use std::{any::type_name, fmt, iter::from_fn, marker::PhantomData};

// Do not replace '&'a Entities' by 'Read<Entities>' to remove the lifetime. This would allow users store a module that has
// dependencies which would effectively hide those dependencies and potentially cause non-deterministic behaviours.
// - Ex: a system adopts could be scheduled in parallel with a system that reads the children of a family instead
// of being scheduled in sequence because of the 'defer-read' dependency conflict that couldn't be seen.
#[derive(Clone)]
pub struct Family<'a>(Entity, &'a Entities);
pub struct State(Read<Entity>, Read<Entities>);

impl<'a> Family<'a> {
    #[inline]
    pub(crate) const fn new(entity: Entity, entities: &'a Entities) -> Self {
        Self(entity, entities)
    }

    #[inline]
    pub const fn entity(&self) -> Entity {
        self.0
    }

    #[inline]
    pub fn root(&self) -> Self {
        Self(self.1.root(self.0), self.1)
    }

    #[inline]
    pub fn parent(&self) -> Option<Self> {
        Some(Self(self.1.parent(self.0)?, self.1))
    }

    #[inline]
    pub fn children(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .children(entity)
            .map(move |child| Self(child, entities))
    }

    #[inline]
    pub fn ancestors(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .ancestors(entity)
            .map(move |parent| Self(parent, entities))
    }

    #[inline]
    pub fn descendants(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .descendants(entity)
            .map(move |child| Self(child, entities))
    }

    #[inline]
    pub fn siblings(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .siblings(entity)
            .map(move |sibling| Self(sibling, entities))
    }

    #[inline]
    pub fn ascend<T>(
        &self,
        mut up: impl FnMut(Self) -> Option<T>,
        mut down: impl FnMut(Self) -> Option<T>,
    ) -> Option<T> {
        self.1.ascend(
            self.0,
            |parent| up(Self(parent, self.1)),
            |parent| down(Self(parent, self.1)),
        )
    }

    #[inline]
    pub fn descend<T>(
        &self,
        mut down: impl FnMut(Self) -> Option<T>,
        mut up: impl FnMut(Self) -> Option<T>,
    ) -> Option<T> {
        self.1.descend(
            self.0,
            |child| down(Self(child, self.1)),
            |child| up(Self(child, self.1)),
        )
    }
}

impl Into<Entity> for Family<'_> {
    #[inline]
    fn into(self) -> Entity {
        self.entity()
    }
}

impl fmt::Debug for Family<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("{:?}", self.entity()))
            .field("parent", &self.parent().map(|parent| parent.entity()))
            .field(
                "children",
                &self
                    .children()
                    .map(|child| child.entity())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Item for Family<'_> {
    type State = State;

    fn initialize(mut context: Context) -> Result<Self::State> {
        Ok(State(
            <Read<_> as Item>::initialize(context.owned())?,
            <Read<_> as Inject>::initialize(None, context.into())?,
        ))
    }
}

impl<'a> At<'a> for State {
    type State = (*const Entity, &'a Entities);
    type Item = Family<'a>;

    #[inline]
    fn get(&'a self, world: &'a World) -> Self::State {
        (self.0.get(world), self.1.as_ref())
    }

    #[inline]
    fn at(state: &Self::State, index: usize) -> Self::Item {
        Family(unsafe { *state.0.add(index) }, state.1)
    }
}

unsafe impl Depend for State {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.1.depend(world);
        // 'Family' may read entities from any segment.
        dependencies.push(Dependency::read::<Entity>());
        dependencies
    }
}

pub mod template {
    use super::*;

    #[derive(Clone)]
    pub struct Family<'a> {
        entity_root: usize,
        entity_index: usize,
        entity_instances: &'a [Entity],
        entity_indices: &'a [EntityIndices],
        segment_indices: &'a [SegmentIndices],
    }

    #[derive(Clone)]
    pub struct Families<'a> {
        entity_roots: &'a [(usize, usize)],
        entity_instances: &'a [Entity],
        entity_indices: &'a [EntityIndices],
        segment_indices: &'a [SegmentIndices],
    }

    #[derive(Clone)]
    pub(crate) struct EntityIndices {
        pub segment: usize,
        pub offset: usize,
        pub parent: Option<usize>,
        pub previous_sibling: Option<usize>,
        pub next_sibling: Option<usize>,
    }

    #[derive(Clone)]
    pub(crate) struct SegmentIndices {
        pub segment: usize,
        pub count: usize,
        pub index: usize,
        pub store: usize,
    }

    impl<'a> Family<'a> {
        #[inline]
        pub(crate) const fn new(
            entity_root: usize,
            entity_index: usize,
            entity_instances: &'a [Entity],
            entity_indices: &'a [EntityIndices],
            segment_indices: &'a [SegmentIndices],
        ) -> Self {
            Self {
                entity_root,
                entity_index,
                entity_instances,
                entity_indices,
                segment_indices,
            }
        }

        pub fn entity(&self) -> Entity {
            let entity_indices = &self.entity_indices[self.entity_index];
            let segment_indices = &self.segment_indices[entity_indices.segment];
            let offset = segment_indices.count * self.entity_root + entity_indices.offset;
            self.entity_instances[segment_indices.index + offset]
        }

        pub fn parent(&self) -> Option<Self> {
            Some(self.with(self.entity_indices[self.entity_index].parent?))
        }

        pub fn root(&self) -> Self {
            // Do not assume that index '0' is the root since there might be multiple roots.
            match self.parent() {
                Some(parent) => parent.root(),
                None => self.clone(),
            }
        }

        pub fn children(&self) -> impl Iterator<Item = Family<'a>> {
            let family = self.clone();
            let parent = Some(self.entity_index);
            let mut next = parent.map(|index| index + 1).filter(|&index| {
                index < self.entity_indices.len() && self.entity_indices[index].parent == parent
            });

            from_fn(move || {
                let current = next?;
                next = family.entity_indices[current].next_sibling;
                Some(family.with(current))
            })
        }

        pub fn siblings(&self) -> impl Iterator<Item = Family<'a>> {
            let entity = self.entity();
            self.parent()
                .map(|parent| parent.children())
                .into_iter()
                .flatten()
                .filter(move |child| child.entity() != entity)
        }

        pub fn ancestors(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
            let mut entities = Vec::new();
            self.ascend(
                |parent| {
                    entities.push(parent);
                    true
                },
                |_| true,
            );
            entities.into_iter()
        }

        pub fn descendants(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
            let mut entities = Vec::new();
            self.descend(
                |child| {
                    entities.push(child);
                    true
                },
                |_| true,
            );
            entities.into_iter()
        }

        pub fn ascend(
            &self,
            mut up: impl FnMut(Self) -> bool,
            mut down: impl FnMut(Self) -> bool,
        ) -> bool {
            fn next<'a>(
                family: &Family<'a>,
                up: &mut impl FnMut(Family<'a>) -> bool,
                down: &mut impl FnMut(Family<'a>) -> bool,
            ) -> bool {
                if let Some(parent) = family.parent() {
                    up(parent.clone()) && next(&parent, up, down) && down(parent)
                } else {
                    true
                }
            }

            next(self, &mut up, &mut down)
        }

        pub fn descend(
            &self,
            mut down: impl FnMut(Self) -> bool,
            mut up: impl FnMut(Self) -> bool,
        ) -> bool {
            #[inline]
            fn next<'a>(
                family: &Family<'a>,
                down: &mut impl FnMut(Family<'a>) -> bool,
                up: &mut impl FnMut(Family<'a>) -> bool,
            ) -> bool {
                for child in family.children() {
                    if down(child.clone()) && next(&child, down, up) && up(child) {
                        continue;
                    } else {
                        return false;
                    }
                }
                true
            }

            next(self, &mut down, &mut up)
        }

        fn with(&self, entity_index: usize) -> Self {
            Self::new(
                self.entity_root,
                entity_index,
                self.entity_instances,
                self.entity_indices,
                self.segment_indices,
            )
        }
    }

    impl fmt::Debug for Family<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let entity = self.entity();
            let parent = self.parent().map(|parent| parent.entity());
            let children: Vec<_> = self.children().map(|child| child.entity()).collect();
            f.debug_struct(&format!("{:?}", entity))
                .field("parent", &parent)
                .field("children", &children)
                .finish()
        }
    }

    impl<'a> Families<'a> {
        pub(crate) const EMPTY: Self = Self {
            entity_roots: &[],
            entity_instances: &[],
            entity_indices: &[],
            segment_indices: &[],
        };

        pub(crate) fn new(
            entity_roots: &'a [(usize, usize)],
            entity_instances: &'a [Entity],
            entity_indices: &'a [EntityIndices],
            segment_indices: &'a [SegmentIndices],
        ) -> Self {
            Self {
                entity_roots,
                entity_instances,
                entity_indices,
                segment_indices,
            }
        }

        pub fn roots(&self) -> impl DoubleEndedIterator<Item = Family<'a>> {
            let families = self.clone();
            families
                .entity_roots
                .iter()
                .map(move |&(entity_root, entity_index)| {
                    Family::new(
                        entity_root,
                        entity_index,
                        families.entity_instances,
                        families.entity_indices,
                        families.segment_indices,
                    )
                })
        }

        pub fn get(&self, index: usize) -> Option<Family<'a>> {
            let &(entity_root, entity_index) = self.entity_roots.get(index)?;
            Some(Family::new(
                entity_root,
                entity_index,
                self.entity_instances,
                self.entity_indices,
                self.segment_indices,
            ))
        }
    }

    impl fmt::Debug for Families<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_list().entries(self.roots()).finish()
        }
    }
}

pub mod item {
    use super::*;

    pub struct LinkIterator<'a, I: Item, F, N> {
        index: u32,
        query: &'a query::Inner<I::State, F>,
        entities: &'a Entities,
        world: &'a World,
        _marker: PhantomData<N>,
    }

    pub trait Next {
        fn next(datum: &Datum) -> u32;
    }

    impl<N: Next> Next for &N {
        #[inline]
        fn next(datum: &Datum) -> u32 {
            N::next(datum)
        }
    }

    impl<N: Next> Next for &mut N {
        #[inline]
        fn next(datum: &Datum) -> u32 {
            N::next(datum)
        }
    }

    impl<'a, I: Item, F, N: Next> Iterator for LinkIterator<'a, I, F, N> {
        type Item = <I::State as At<'a>>::Item;

        fn next(&mut self) -> Option<Self::Item> {
            while let Some(datum) = self.entities.get_datum_at(self.index) {
                self.index = N::next(datum);
                if let Some(item) = get_item::<I, F>(datum, self.query, self.world) {
                    return Some(item);
                }
            }
            None
        }
    }

    fn get_datum_at<N: Next>(
        mut index: u32,
        mut at: usize,
        entities: &Entities,
    ) -> Option<(&Datum, u32)> {
        let mut datum = entities.get_datum_at(index)?;
        while at > 0 {
            at -= 1;
            index = N::next(datum);
            datum = entities.get_datum_at(index)?;
        }
        Some((datum, index))
    }

    fn get_item<'a, I: Item, F>(
        datum: &Datum,
        inner: &'a query::Inner<I::State, F>,
        world: &'a World,
    ) -> Option<<I::State as At<'a>>::Item> {
        let state = inner.segments[datum.segment_index as usize]?;
        let state = inner.states[state].0.get(world);
        Some(I::State::at(&state, datum.store_index as usize))
    }

    pub mod child {
        use super::*;

        pub struct Child<'a, I: Item, F = ()> {
            index: u32,
            query: &'a query::Inner<I::State, F>,
            entities: &'a Entities,
            world: &'a World,
        }

        pub struct State<I: Item, F> {
            entity: <Read<Entity> as Item>::State,
            entities: <Read<Entities> as Inject>::State,
            query: query::State<I, F>,
        }

        impl<I: Item, F> Child<'_, I, F> {
            pub fn get<'a>(&'a self, index: usize) -> Option<<I::State as At<'a>>::Item>
            where
                I::State: At<'a>,
            {
                let current = get_datum_at::<Self>(self.index, index + 1, self.entities)?;
                get_item::<I, F>(current.0, self.query, self.world)
            }
        }

        impl<I: Item, F> Next for Child<'_, I, F> {
            #[inline]
            fn next(datum: &Datum) -> u32 {
                datum.previous_sibling
            }
        }

        impl<I: Item, F> fmt::Debug for Child<'_, I, F>
        where
            for<'a> <I::State as At<'a>>::Item: fmt::Debug,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(type_name::<Self>())?;
                f.debug_list().entries(self).finish()
            }
        }

        impl<I: Item, F: Filter + 'static> Item for Child<'_, I, F>
        where
            I::State: Send + Sync + 'static,
        {
            type State = State<I, F>;

            fn initialize(mut context: Context) -> Result<Self::State> {
                Ok(State {
                    entity: <Read<_> as Item>::initialize(context.owned())?,
                    entities: <Read<_> as Inject>::initialize(None, context.owned().into())?,
                    query: Query::initialize((), context.into())?,
                })
            }

            fn update(
                State {
                    entity,
                    entities,
                    query,
                }: &mut Self::State,
                mut context: Context,
            ) -> Result {
                <Read<_> as Item>::update(entity, context.owned())?;
                <Read<_> as Inject>::update(entities, context.owned().into())?;
                Query::update(query, context.into())?;
                Ok(())
            }
        }

        impl<'a, I: Item, F: 'static> At<'a> for State<I, F>
        where
            I::State: 'static,
        {
            type State = (
                *const Entity,
                &'a Entities,
                &'a query::Inner<I::State, F>,
                &'a World,
            );
            type Item = Child<'a, I, F>;

            #[inline]
            fn get(&'a self, world: &'a World) -> Self::State {
                (
                    self.entity.get(world),
                    self.entities.as_ref(),
                    self.query.inner.as_ref(),
                    world,
                )
            }

            #[inline]
            fn at(state: &Self::State, index: usize) -> Self::Item {
                Child {
                    index: unsafe { *state.0.add(index) }.index(),
                    entities: state.1,
                    query: state.2,
                    world: state.3,
                }
            }
        }

        unsafe impl<I: Item, F: 'static> Depend for State<I, F>
        where
            I::State: 'static,
        {
            fn depend(&self, world: &World) -> Vec<Dependency> {
                let mut dependencies = self.entities.depend(world);
                dependencies.append(&mut self.query.inner.depend(world));
                // 'Child' may read entities from any segment.
                dependencies.push(Dependency::read::<Entity>());
                dependencies
            }
        }

        impl<'a, I: Item, F> IntoIterator for &'a Child<'a, I, F> {
            type Item = <I::State as At<'a>>::Item;
            type IntoIter = LinkIterator<'a, I, F, Self>;
            fn into_iter(self) -> Self::IntoIter {
                LinkIterator {
                    index: self.entities.get_datum_at(self.index).unwrap().first_child,
                    query: self.query,
                    entities: self.entities,
                    world: self.world,
                    _marker: PhantomData,
                }
            }
        }
    }

    pub mod parent {
        use super::*;

        pub struct Parent<'a, I: Item, F = ()> {
            index: u32,
            query: &'a query::Inner<I::State, F>,
            entities: &'a Entities,
            world: &'a World,
        }

        pub struct State<I: Item, F> {
            entity: <Read<Entity> as Item>::State,
            entities: <Read<Entities> as Inject>::State,
            query: query::State<I, F>,
        }

        impl<I: Item, F> Parent<'_, I, F> {
            pub fn get<'a>(&'a self, index: usize) -> Option<<I::State as At<'a>>::Item>
            where
                I::State: At<'a>,
            {
                let pair = get_datum_at::<Self>(self.index, index + 1, self.entities)?;
                get_item::<I, F>(pair.0, self.query, self.world)
            }
        }

        impl<I: Item, F> Next for Parent<'_, I, F> {
            #[inline]
            fn next(datum: &Datum) -> u32 {
                datum.parent
            }
        }

        impl<I: Item, F> fmt::Debug for Parent<'_, I, F>
        where
            for<'a> <I::State as At<'a>>::Item: fmt::Debug,
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(type_name::<Self>())?;
                f.debug_list().entries(self).finish()
            }
        }

        impl<I: Item, F: Filter + 'static> Item for Parent<'_, I, F>
        where
            I::State: Send + Sync + 'static,
        {
            type State = State<I, F>;

            fn initialize(mut context: Context) -> Result<Self::State> {
                Ok(State {
                    entity: <Read<_> as Item>::initialize(context.owned())?,
                    entities: <Read<_> as Inject>::initialize(None, context.owned().into())?,
                    query: Query::initialize((), context.into())?,
                })
            }

            fn update(
                State {
                    entity,
                    entities,
                    query,
                }: &mut Self::State,
                mut context: Context,
            ) -> Result {
                <Read<_> as Item>::update(entity, context.owned())?;
                <Read<_> as Inject>::update(entities, context.owned().into())?;
                Query::update(query, context.into())?;
                Ok(())
            }
        }

        impl<'a, I: Item, F: 'static> At<'a> for State<I, F>
        where
            I::State: 'static,
        {
            type State = (
                *const Entity,
                &'a Entities,
                &'a query::Inner<I::State, F>,
                &'a World,
            );
            type Item = Parent<'a, I, F>;

            #[inline]
            fn get(&'a self, world: &'a World) -> Self::State {
                (
                    self.entity.get(world),
                    self.entities.as_ref(),
                    self.query.inner.as_ref(),
                    world,
                )
            }

            #[inline]
            fn at(state: &Self::State, index: usize) -> Self::Item {
                Parent {
                    index: unsafe { *state.0.add(index) }.index(),
                    entities: state.1,
                    query: state.2,
                    world: state.3,
                }
            }
        }

        unsafe impl<I: Item, F: 'static> Depend for State<I, F>
        where
            I::State: 'static,
        {
            fn depend(&self, world: &World) -> Vec<Dependency> {
                let mut dependencies = self.entities.depend(world);
                dependencies.append(&mut self.query.inner.depend(world));
                // 'Parent' may read entities from any segment.
                dependencies.push(Dependency::read::<Entity>());
                dependencies
            }
        }

        impl<'a, I: Item, F> IntoIterator for &'a Parent<'a, I, F> {
            type Item = <I::State as At<'a>>::Item;
            type IntoIter = LinkIterator<'a, I, F, Self>;
            fn into_iter(self) -> Self::IntoIter {
                LinkIterator {
                    index: self.entities.get_datum_at(self.index).unwrap().parent,
                    query: self.query,
                    entities: self.entities,
                    world: self.world,
                    _marker: PhantomData,
                }
            }
        }
    }
}

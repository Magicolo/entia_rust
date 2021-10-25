use std::{any::type_name, fmt, iter::from_fn, marker::PhantomData};

use crate::{
    depend::{Depend, Dependency},
    entities::{Datum, Entities, Horizontal, Vertical},
    entity::Entity,
    inject::Inject,
    query::{
        self,
        filter::Filter,
        item::{At, Item, ItemContext},
        Query,
    },
    read::Read,
    world::World,
    write,
};

pub mod initial {
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

    pub struct FamiliesIterator<'a, 'b> {
        index: usize,
        families: &'b Families<'a>,
    }

    #[derive(Clone)]
    pub struct EntityIndices {
        pub segment: usize,
        pub offset: usize,
        pub parent: Option<usize>,
        pub previous_sibling: Option<usize>,
        pub next_sibling: Option<usize>,
    }

    #[derive(Clone)]
    pub struct SegmentIndices {
        pub segment: usize,
        pub count: usize,
        pub index: usize,
        pub store: usize,
    }

    impl<'a> Family<'a> {
        pub const EMPTY: Self = Self {
            entity_root: 0,
            entity_index: 0,
            entity_instances: &[],
            entity_indices: &[],
            segment_indices: &[],
        };

        #[inline]
        pub const fn new(
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

        #[inline]
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
            self.parent()
                .map(|parent| parent.root())
                .unwrap_or(self.clone())
        }

        pub fn children(&self, direction: Horizontal) -> impl Iterator<Item = Family<'a>> {
            // TODO: Implement 'Horizontal::FromRight' or, if possible, unify this implementation with 'entities::family'.
            let local = self.clone();
            let parent = Some(self.entity_index);
            let mut next = parent.map(|index| index + 1).filter(|&index| {
                index < self.entity_indices.len() && self.entity_indices[index].parent == parent
            });

            from_fn(move || {
                let current = next?;
                next = local.entity_indices[current].next_sibling;
                Some(local.with(current))
            })
        }

        pub fn ascend(&self, direction: Vertical, mut each: impl FnMut(Self)) {
            fn from_top<'a>(family: &Family<'a>, each: &mut impl FnMut(Family<'a>)) {
                if let Some(parent) = family.parent() {
                    from_top(&parent, each);
                    each(parent);
                }
            }

            fn from_bottom<'a>(family: &Family<'a>, each: &mut impl FnMut(Family<'a>)) {
                if let Some(parent) = family.parent() {
                    each(parent.clone());
                    from_bottom(&parent, each);
                }
            }

            match direction {
                Vertical::FromTop => from_top(self, &mut each),
                Vertical::FromBottom => from_bottom(self, &mut each),
            }
        }

        pub fn descend(&self, direction: (Horizontal, Vertical), mut each: impl FnMut(Self)) {
            fn from_top<'a>(
                family: &Family<'a>,
                direction: Horizontal,
                each: &mut impl FnMut(Family<'a>),
            ) {
                for child in family.children(direction) {
                    each(child.clone());
                    from_top(&child, direction, each);
                }
            }

            fn from_bototm<'a>(
                family: &Family<'a>,
                direction: Horizontal,
                each: &mut impl FnMut(Family<'a>),
            ) {
                for child in family.children(direction) {
                    from_bototm(&child, direction, each);
                    each(child);
                }
            }

            match direction.1 {
                Vertical::FromTop => from_top(self, direction.0, &mut each),
                Vertical::FromBottom => from_bototm(self, direction.0, &mut each),
            }
        }

        pub fn descendants(
            &self,
            direction: (Horizontal, Vertical),
        ) -> impl Iterator<Item = Family<'a>> {
            let mut descendants = Vec::new();
            self.descend(direction, |child| descendants.push(child));
            descendants.into_iter()
        }

        pub fn ancestors(&self) -> impl Iterator<Item = Family<'a>> {
            let local = self.clone();
            let mut next = self.entity_indices[self.entity_index].parent;
            from_fn(move || {
                let current = next?;
                next = local.entity_indices[current].parent;
                Some(local.with(current))
            })
        }

        pub fn siblings(&self, direction: Horizontal) -> impl Iterator<Item = Family<'a>> {
            let entity_index = self.entity_index;
            self.parent()
                .map(|parent| parent.children(direction))
                .into_iter()
                .flatten()
                .filter(move |child| child.entity_index != entity_index)
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

    impl std::fmt::Debug for Family<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let entity = self.entity();
            let parent = self.parent().map(|parent| parent.entity());
            let children: Vec<_> = self
                .children(Horizontal::FromLeft)
                .map(|child| child.entity())
                .collect();
            f.debug_struct("Family")
                .field("entity", &entity)
                .field("parent", &parent)
                .field("children", &children)
                .finish()
        }
    }

    impl<'a> Families<'a> {
        pub const EMPTY: Self = Self {
            entity_roots: &[],
            entity_instances: &[],
            entity_indices: &[],
            segment_indices: &[],
        };

        pub fn new(
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

        pub fn len(&self) -> usize {
            self.entity_roots.len()
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

    impl std::fmt::Debug for Families<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_list().entries(self).finish()
        }
    }

    impl<'a, 'b> IntoIterator for &'b Families<'a> {
        type Item = Family<'a>;
        type IntoIter = FamiliesIterator<'a, 'b>;

        fn into_iter(self) -> Self::IntoIter {
            Self::IntoIter {
                index: 0,
                families: self,
            }
        }
    }

    impl<'a, 'b> Iterator for FamiliesIterator<'a, 'b> {
        type Item = Family<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            let family = self.families.get(self.index)?;
            self.index += 1;
            Some(family)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (self.len(), Some(self.len()))
        }
    }

    impl<'a, 'b> ExactSizeIterator for FamiliesIterator<'a, 'b> {
        fn len(&self) -> usize {
            self.families.len() - self.index
        }
    }
}

pub mod item {
    use super::*;

    pub struct Child<'a, I: Item, F: Filter = ()> {
        index: u32,
        query: write::State<query::Inner<I, F>>,
        entities: &'a Entities,
        world: &'a World,
    }

    pub struct Parent<'a, I: Item, F: Filter = ()> {
        index: u32,
        query: write::State<query::Inner<I, F>>,
        entities: &'a Entities,
        world: &'a World,
    }

    pub struct ChildState<I: Item + 'static, F: Filter> {
        entity: <Read<Entity> as Item>::State,
        entities: <Read<Entities> as Inject>::State,
        query: query::State<I, F>,
    }

    pub struct ParentState<I: Item + 'static, F: Filter> {
        entity: <Read<Entity> as Item>::State,
        entities: <Read<Entities> as Inject>::State,
        query: query::State<I, F>,
    }

    pub struct LinkIterator<'a, I: Item, F: Filter, N: Next> {
        index: u32,
        query: &'a query::Inner<I, F>,
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

    impl<I: Item + 'static, F: Filter> Child<'_, I, F> {
        pub fn get<'a>(&'a self, index: usize) -> Option<<I::State as At<'a>>::Item>
        where
            I::State: At<'a>,
        {
            let current = get_datum_at::<Self>(self.index, index + 1, self.entities)?;
            get_item(current.0, self.query.as_ref(), self.world)
        }
    }

    impl<I: Item, F: Filter> Next for Child<'_, I, F> {
        #[inline]
        fn next(datum: &Datum) -> u32 {
            datum.previous_sibling
        }
    }

    impl<I: Item + 'static, F: Filter> fmt::Debug for Child<'_, I, F>
    where
        for<'a> <I::State as At<'a>>::Item: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(type_name::<Self>())?;
            f.debug_list().entries(self).finish()
        }
    }

    unsafe impl<I: Item + 'static, F: Filter> Item for Child<'_, I, F> {
        type State = ChildState<I, F>;

        fn initialize(mut context: ItemContext) -> Option<Self::State> {
            Some(ChildState {
                entity: <Read<Entity> as Item>::initialize(context.owned())?,
                entities: <Read<Entities> as Inject>::initialize(None, context.owned().into())?,
                query: <Query<I, F> as Inject>::initialize((), context.into())?,
            })
        }

        #[inline]
        fn update(
            ChildState {
                entity,
                entities,
                query,
            }: &mut Self::State,
            mut context: ItemContext,
        ) {
            <Read<Entity> as Item>::update(entity, context.owned());
            <Read<Entities> as Inject>::update(entities, context.owned().into());
            <Query<I, F> as Inject>::update(query, context.into());
        }
    }

    impl<'a, I: Item<State = impl At<'a> + 'static>, F: Filter> At<'a> for ChildState<I, F> {
        type Item = Child<'a, I, F>;

        #[inline]
        fn at(&'a self, index: usize, world: &'a World) -> Self::Item {
            Child {
                index: self.entity.at(index, world).index,
                entities: self.entities.as_ref(),
                query: self.query.inner.clone(),
                world,
            }
        }
    }

    unsafe impl<I: Item, F: Filter> Depend for ChildState<I, F> {
        fn depend(&self, world: &World) -> Vec<Dependency> {
            let mut dependencies = self.entity.depend(world);
            dependencies.append(&mut self.entities.depend(world));
            dependencies.append(&mut self.query.inner.depend(world));
            dependencies
        }
    }

    impl<'a, I: Item<State = impl At<'a> + 'a> + 'static, F: Filter> IntoIterator
        for &'a Child<'a, I, F>
    {
        type Item = <I::State as At<'a>>::Item;
        type IntoIter = LinkIterator<'a, I, F, Self>;
        fn into_iter(self) -> Self::IntoIter {
            LinkIterator {
                index: self.entities.data.0[self.index as usize].first_child,
                query: self.query.as_ref(),
                entities: self.entities,
                world: self.world,
                _marker: PhantomData,
            }
        }
    }

    impl<I: Item + 'static, F: Filter> Parent<'_, I, F> {
        pub fn get<'a>(&'a self, index: usize) -> Option<<I::State as At<'a>>::Item>
        where
            I::State: At<'a>,
        {
            let pair = get_datum_at::<Self>(self.index, index + 1, self.entities)?;
            get_item(pair.0, self.query.as_ref(), self.world)
        }
    }

    impl<I: Item, F: Filter> Next for Parent<'_, I, F> {
        #[inline]
        fn next(datum: &Datum) -> u32 {
            datum.parent
        }
    }

    impl<I: Item + 'static, F: Filter> fmt::Debug for Parent<'_, I, F>
    where
        for<'a> <I::State as At<'a>>::Item: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(type_name::<Self>())?;
            f.debug_list().entries(self).finish()
        }
    }

    unsafe impl<I: Item + 'static, F: Filter> Item for Parent<'_, I, F> {
        type State = ParentState<I, F>;

        fn initialize(mut context: ItemContext) -> Option<Self::State> {
            Some(ParentState {
                entity: <Read<Entity> as Item>::initialize(context.owned())?,
                entities: <Read<Entities> as Inject>::initialize(None, context.owned().into())?,
                query: <Query<I, F> as Inject>::initialize((), context.into())?,
            })
        }

        #[inline]
        fn update(
            ParentState {
                entity,
                entities,
                query,
            }: &mut Self::State,
            mut context: ItemContext,
        ) {
            <Read<Entity> as Item>::update(entity, context.owned());
            <Read<Entities> as Inject>::update(entities, context.owned().into());
            <Query<I, F> as Inject>::update(query, context.into());
        }
    }

    impl<'a, I: Item, F: Filter> At<'a> for ParentState<I, F> {
        type Item = Parent<'a, I, F>;

        #[inline]
        fn at(&'a self, index: usize, world: &'a World) -> Self::Item {
            Parent {
                index: self.entity.at(index, world).index,
                entities: self.entities.as_ref(),
                query: self.query.inner.clone(),
                world,
            }
        }
    }

    unsafe impl<I: Item, F: Filter> Depend for ParentState<I, F> {
        fn depend(&self, world: &World) -> Vec<Dependency> {
            let mut dependencies = self.entity.depend(world);
            dependencies.append(&mut self.entities.depend(world));
            dependencies.append(&mut self.query.inner.depend(world));
            dependencies
        }
    }

    impl<'a, I: Item + 'static, F: Filter> IntoIterator for &'a Parent<'a, I, F> {
        type Item = <I::State as At<'a>>::Item;
        type IntoIter = LinkIterator<'a, I, F, Self>;
        fn into_iter(self) -> Self::IntoIter {
            LinkIterator {
                index: self.entities.data.0[self.index as usize].parent,
                query: self.query.as_ref(),
                entities: self.entities,
                world: self.world,
                _marker: PhantomData,
            }
        }
    }

    impl<'a, I: Item, F: Filter, N: Next> Iterator for LinkIterator<'a, I, F, N> {
        type Item = <I::State as At<'a>>::Item;

        fn next(&mut self) -> Option<Self::Item> {
            while let Some(datum) = self.entities.data.0.get(self.index as usize) {
                self.index = N::next(datum);
                if let Some(item) = get_item(datum, self.query, self.world) {
                    return Some(item);
                }
            }
            None
        }
    }

    #[inline]
    fn get_datum_at<N: Next>(
        mut index: u32,
        mut at: usize,
        entities: &Entities,
    ) -> Option<(&Datum, u32)> {
        let mut datum = entities.data.0.get(index as usize)?;
        while at > 0 {
            at -= 1;
            index = N::next(datum);
            datum = entities.data.0.get(index as usize)?;
        }
        Some((datum, index))
    }

    #[inline]
    fn get_item<'a, I: Item, F: Filter>(
        datum: &Datum,
        inner: &'a query::Inner<I, F>,
        world: &'a World,
    ) -> Option<<I::State as At<'a>>::Item> {
        let state = inner.segments[datum.segment_index as usize]?;
        Some(inner.states[state].0.at(datum.store_index as usize, world))
    }
}

use std::{iter::from_fn, marker::PhantomData};

use crate::{
    component::Component,
    depend::{Depend, Dependency},
    entity::Entity,
    filter::Filter,
    inject::Inject,
    item::{At, Item, ItemContext},
    query::{self, Query},
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

    pub enum Direction {
        TopDown,
        BottomUp,
    }

    #[derive(Clone)]
    pub struct EntityIndices {
        pub segment: usize,
        pub offset: usize,
        pub parent: Option<usize>,
        pub sibling: Option<usize>,
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

        pub fn children(&self) -> impl Iterator<Item = Family<'a>> {
            let local = self.clone();
            let parent = Some(self.entity_index);
            let mut next = parent.map(|index| index + 1).filter(|&index| {
                index < self.entity_indices.len() && self.entity_indices[index].parent == parent
            });

            from_fn(move || {
                let current = next?;
                next = local.entity_indices[current].sibling;
                Some(local.with(current))
            })
        }

        pub fn descend(&self, direction: Direction, mut each: impl FnMut(Self)) {
            fn top_down<'a>(parent: &Family<'a>, each: &mut impl FnMut(Family<'a>)) {
                for child in parent.children() {
                    each(child.clone());
                    top_down(&child, each);
                }
            }

            fn bottom_up<'a>(parent: &Family<'a>, each: &mut impl FnMut(Family<'a>)) {
                for child in parent.children() {
                    bottom_up(&child, each);
                    each(child);
                }
            }

            match direction {
                Direction::TopDown => top_down(self, &mut each),
                Direction::BottomUp => bottom_up(self, &mut each),
            }
        }

        pub fn all(&self, direction: Direction) -> impl Iterator<Item = Family<'a>> {
            let root = self.root();
            match direction {
                Direction::TopDown => Some(root.clone())
                    .into_iter()
                    .chain(root.descendants(direction))
                    .chain(None),
                Direction::BottomUp => None
                    .into_iter()
                    .chain(root.descendants(direction))
                    .chain(Some(root.clone())),
            }
        }

        pub fn descendants(&self, direction: Direction) -> impl Iterator<Item = Family<'a>> {
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

        pub fn left(&self) -> impl Iterator<Item = Family<'a>> {
            let entity_index = self.entity_index;
            self.parent()
                .map(|parent| parent.children())
                .into_iter()
                .flatten()
                .take_while(move |child| child.entity_index != entity_index)
        }

        pub fn right(&self) -> impl Iterator<Item = Family<'a>> {
            let local = self.clone();
            let mut next = local.entity_indices[self.entity_index].sibling;

            from_fn(move || {
                let current = next?;
                next = local.entity_indices[current].sibling;
                Some(local.with(current))
            })
        }

        pub fn siblings(&self) -> impl Iterator<Item = Family<'a>> {
            let entity_index = self.entity_index;
            self.parent()
                .map(|parent| parent.children())
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
            let children: Vec<_> = self.children().map(|child| child.entity()).collect();
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

    pub struct Child<'a, I: Item, F: Filter = ()>(
        Option<(usize, usize)>,
        write::State<query::Inner<I, F>>,
        &'a World,
    );

    pub struct Parent<'a, I: Item, F: Filter = ()>(
        Option<(usize, usize)>,
        write::State<query::Inner<I, F>>,
        &'a World,
    );

    pub struct Link {
        pub(crate) parent: Option<(usize, usize)>,
        pub(crate) child: Option<(usize, usize)>,
        pub(crate) next: Option<(usize, usize)>,
    }

    pub struct ChildState<I: Item + 'static, F: Filter>(
        <Read<Link> as Item>::State,
        query::State<I, F>,
    );

    pub struct ParentState<I: Item + 'static, F: Filter>(
        <Read<Link> as Item>::State,
        query::State<I, F>,
    );

    pub struct LinkIterator<'a, I: Item, F: Filter, N: Next>(
        Option<(usize, usize)>,
        &'a query::Inner<I, F>,
        &'a World,
        PhantomData<N>,
    );

    pub trait Next {
        fn next(link: &Link) -> Option<(usize, usize)>;
    }

    impl Component for Link {}

    impl<N: Next> Next for &N {
        #[inline]
        fn next(link: &Link) -> Option<(usize, usize)> {
            N::next(link)
        }
    }

    impl<N: Next> Next for &mut N {
        #[inline]
        fn next(link: &Link) -> Option<(usize, usize)> {
            N::next(link)
        }
    }

    impl<I: Item + 'static, F: Filter> Child<'_, I, F> {
        pub fn get<'a>(&'a self, index: usize) -> Option<<I::State as At<'a>>::Item>
        where
            I::State: At<'a>,
        {
            let current = get_link::<Self>(self.0?, index, self.2)?;
            get_item(current, self.1.as_ref(), self.2)
        }
    }

    impl<I: Item, F: Filter> Next for Child<'_, I, F> {
        #[inline]
        fn next(link: &Link) -> Option<(usize, usize)> {
            link.next
        }
    }

    unsafe impl<I: Item + 'static, F: Filter> Item for Child<'_, I, F> {
        type State = ChildState<I, F>;

        fn initialize(mut context: ItemContext) -> Option<Self::State> {
            Some(ChildState(
                <Read<Link> as Item>::initialize(context.owned())?,
                <Query<I, F> as Inject>::initialize((), context.into())?,
            ))
        }

        fn update(ChildState(read, query): &mut Self::State, mut context: ItemContext) {
            <Read<Link> as Item>::update(read, context.owned());
            <Query<I, F> as Inject>::update(query, context.into());
        }
    }

    impl<'a, I: Item<State = impl At<'a> + 'static>, F: Filter> At<'a> for ChildState<I, F> {
        type Item = Child<'a, I, F>;

        #[inline]
        fn at(&'a self, index: usize, world: &'a World) -> Self::Item {
            Child(self.0.at(index, world).child, self.1.inner.clone(), world)
        }
    }

    unsafe impl<I: Item, F: Filter> Depend for ChildState<I, F> {
        fn depend(&self, world: &World) -> Vec<Dependency> {
            let mut dependencies = self.0.depend(world);
            dependencies.append(&mut self.1.inner.depend(world));
            dependencies
        }
    }

    impl<'a, I: Item<State = impl At<'a> + 'a> + 'static, F: Filter> IntoIterator
        for &'a Child<'a, I, F>
    {
        type Item = <I::State as At<'a>>::Item;
        type IntoIter = LinkIterator<'a, I, F, Self>;
        fn into_iter(self) -> Self::IntoIter {
            LinkIterator(self.0, self.1.as_ref(), self.2, PhantomData)
        }
    }

    impl<I: Item + 'static, F: Filter> Parent<'_, I, F> {
        pub fn get<'a>(&'a self, index: usize) -> Option<<I::State as At<'a>>::Item>
        where
            I::State: At<'a>,
        {
            let current = get_link::<Self>(self.0?, index, self.2)?;
            get_item(current, self.1.as_ref(), self.2)
        }
    }

    impl<I: Item, F: Filter> Next for Parent<'_, I, F> {
        #[inline]
        fn next(link: &Link) -> Option<(usize, usize)> {
            link.parent
        }
    }

    unsafe impl<I: Item + 'static, F: Filter> Item for Parent<'_, I, F> {
        type State = ParentState<I, F>;

        fn initialize(mut context: ItemContext) -> Option<Self::State> {
            Some(ParentState(
                <Read<Link> as Item>::initialize(context.owned())?,
                <Query<I, F> as Inject>::initialize((), context.into())?,
            ))
        }

        fn update(ParentState(read, query): &mut Self::State, mut context: ItemContext) {
            <Read<Link> as Item>::update(read, context.owned());
            <Query<I, F> as Inject>::update(query, context.into());
        }
    }

    impl<'a, I: Item, F: Filter> At<'a> for ParentState<I, F> {
        type Item = Parent<'a, I, F>;

        #[inline]
        fn at(&'a self, index: usize, world: &'a World) -> Self::Item {
            Parent(self.0.at(index, world).parent, self.1.inner.clone(), world)
        }
    }

    unsafe impl<I: Item, F: Filter> Depend for ParentState<I, F> {
        fn depend(&self, world: &World) -> Vec<Dependency> {
            let mut dependencies = self.0.depend(world);
            dependencies.append(&mut self.1.inner.depend(world));
            dependencies
        }
    }

    impl<'a, I: Item + 'static, F: Filter> IntoIterator for &'a Parent<'a, I, F> {
        type Item = <I::State as At<'a>>::Item;
        type IntoIter = LinkIterator<'a, I, F, Self>;
        fn into_iter(self) -> Self::IntoIter {
            LinkIterator(self.0, self.1.as_ref(), self.2, PhantomData)
        }
    }

    impl<'a, I: Item, F: Filter, N: Next> Iterator for LinkIterator<'a, I, F, N> {
        type Item = <I::State as At<'a>>::Item;

        fn next(&mut self) -> Option<Self::Item> {
            let LinkIterator(current, inner, world, _) = self;
            while let Some(pair) = *current {
                // 'Family' segment pointers must point to segments that have both an 'Entity' and 'Family' store.
                *current = get_link::<N>(pair, 1, world);
                if let Some(item) = get_item(pair, inner, world) {
                    return Some(item);
                }
            }
            None
        }
    }

    #[inline]
    fn get_link<N: Next>(
        mut current: (usize, usize),
        mut index: usize,
        world: &World,
    ) -> Option<(usize, usize)> {
        while index > 0 {
            index -= 1;
            current =
                N::next(unsafe { world.segments[current.0].stores[1].get::<Link>(current.1) })?;
        }
        Some(current)
    }

    #[inline]
    fn get_item<'a, I: Item, F: Filter>(
        (segment, index): (usize, usize),
        inner: &'a query::Inner<I, F>,
        world: &'a World,
    ) -> Option<<I::State as At<'a>>::Item> {
        let state = inner.segments[segment]?;
        Some(inner.states[state].0.at(index, world))
    }
}

use crate::{
    depend::Dependency,
    entities::Entities,
    entity::{self, Entity},
    error,
    inject::{Adapt, Context, Inject},
    item::{At, Item},
    resource,
    segment::Segment,
};
use entia_core::FullIterator;
use std::{
    fmt,
    iter::from_fn,
    ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
};

// Do not replace '&'a Entities' by 'Read<Entities>' to remove the lifetime. This would allow users store a module that has
// dependencies which would effectively hide those dependencies and potentially cause non-deterministic behaviours.
#[derive(Clone, Copy)]
pub struct Family<'a>(Entity, &'a Entities);
pub struct State(entity::State, resource::Read<Entities>);

impl<'a> Family<'a> {
    #[inline]
    pub const fn new(entity: Entity, entities: &'a Entities) -> Self {
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
    pub fn children(&self) -> impl FullIterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .children(entity)
            .map(move |child| Self(child, entities))
    }

    #[inline]
    pub fn siblings(&self) -> impl FullIterator<Item = Family<'a>> {
        let Self(entity, entities) = *self;
        entities
            .siblings(entity)
            .map(move |sibling| Self(sibling, entities))
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
    pub fn ascend<U: FnMut(Entity), D: FnMut(Entity)>(&self, up: U, down: D) {
        self.1.ascend(self.0, up, down)
    }

    #[inline]
    pub fn try_ascend<
        S,
        E,
        U: FnMut(Entity, S) -> Result<S, E>,
        D: FnMut(Entity, S) -> Result<S, E>,
    >(
        &self,
        state: S,
        up: U,
        down: D,
    ) -> Result<S, E> {
        self.1.try_ascend(self.0, state, up, down)
    }

    #[inline]
    pub fn descend<D: FnMut(Entity), U: FnMut(Entity)>(&self, down: D, up: U) {
        self.1.descend(self.0, down, up)
    }

    #[inline]
    pub fn try_descend<
        S,
        E,
        D: FnMut(Entity, S) -> Result<S, E>,
        U: FnMut(Entity, S) -> Result<S, E>,
    >(
        &self,
        state: S,
        down: D,
        up: U,
    ) -> Result<S, E> {
        self.1.try_descend(self.0, state, down, up)
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

    fn initialize<A: Adapt<Self::State>>(
        segment: &Segment,
        mut context: Context<Self::State, A>,
    ) -> error::Result<Self::State> {
        Ok(State(
            Entity::initialize(segment, context.map(|state| &mut state.0))?,
            resource::Read::initialize(None, context.map(|state| &mut state.1))?,
        ))
    }

    fn depend(state: &Self::State) -> Vec<Dependency> {
        let mut dependencies = Entity::depend(&state.0);
        dependencies.extend(resource::Read::depend(&state.1));
        dependencies
    }
}

pub struct FamilyChunk<'a>(&'a [Entity], &'a Entities);

impl<'a> At<'a> for State {
    type State = (<entity::State as At<'a>>::State, &'a Entities);
    type Ref = Family<'a>;
    type Mut = Self::Ref;

    #[inline]
    fn get(&'a self, segment: &Segment) -> Option<Self::State> {
        Some((<entity::State as At<'a>>::get(&self.0, segment)?, &self.1))
    }

    #[inline]
    unsafe fn at_ref(state: &Self::State, index: usize) -> Self::Ref {
        Family::new(<entity::State as At<'a>>::at_ref(&state.0, index), state.1)
    }

    #[inline]
    unsafe fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut {
        Self::at_ref(state, index)
    }
}

macro_rules! at {
    ($r:ty) => {
        impl<'a> At<'a, $r> for State {
            type State = (<entity::State as At<'a, $r>>::State, &'a Entities);
            type Ref = FamilyChunk<'a>;
            type Mut = Self::Ref;

            #[inline]
            fn get(&'a self, segment: &Segment) -> Option<Self::State> {
                Some((<entity::State as At<'a, $r>>::get(&self.0, segment)?, &self.1))
            }

            #[inline]
            unsafe fn at_ref(state: &Self::State, index: $r) -> Self::Ref {
                FamilyChunk(entity::State::at_ref(&state.0, index), state.1)
            }

            #[inline]
            unsafe fn at_mut(state: &mut Self::State, index: $r) -> Self::Mut {
                Self::at_ref(state, index)
            }
        }
    };
    ($($r:ty,)*) => { $(at!($r);)* };
}

at!(
    RangeFull,
    Range<usize>,
    RangeInclusive<usize>,
    RangeFrom<usize>,
    RangeTo<usize>,
    RangeToInclusive<usize>,
);

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

    impl Into<Entity> for Family<'_> {
        #[inline]
        fn into(self) -> Entity {
            self.entity()
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

        pub fn roots(&self) -> impl FullIterator<Item = Family<'a>> {
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

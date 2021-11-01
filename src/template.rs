use crate::{
    entities::Entities,
    entity::Entity,
    family::template::{EntityIndices, Family, SegmentIndices},
    world::{segment::Segment, store::Store, Meta, World},
};
use std::{array::IntoIter, collections::HashMap, marker::PhantomData, sync::Arc};

pub struct GetMeta(fn(&mut World) -> Arc<Meta>);

pub struct DeclareContext<'a> {
    metas_index: usize,
    segment_metas: &'a mut Vec<Vec<Arc<Meta>>>,
    world: &'a mut World,
}

pub struct InitializeContext<'a> {
    segment_index: usize,
    segment_indices: &'a [SegmentIndices],
    metas_to_segment: &'a HashMap<usize, usize>,
    world: &'a World,
}

pub struct CountContext<'a> {
    segment_index: usize,
    segment_indices: &'a mut [SegmentIndices],
    entity_index: usize,
    entity_parent: Option<usize>,
    entity_previous: &'a mut Option<usize>,
    entity_indices: &'a mut Vec<EntityIndices>,
}

pub struct ApplyContext<'a> {
    entity_root: usize,
    entity_index: usize,
    entity_parent: Option<u32>,
    entity_previous_sibling: &'a mut Option<u32>,
    entity_count: &'a mut usize,
    entity_instances: &'a [Entity],
    entity_indices: &'a [EntityIndices],
    entities: &'a mut Entities,
    store_index: usize,
    segment_indices: &'a [SegmentIndices],
}

pub trait Template {
    type Input;
    type Declare;
    type State;

    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare;
    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State;
    fn static_count(state: &Self::State, context: CountContext) -> bool;
    fn dynamic_count(&self, state: &Self::State, context: CountContext);
    fn apply(self, state: &Self::State, context: ApplyContext);
}

/// Implementors of this trait must guarantee that the 'static_count' function always succeeds.
pub unsafe trait StaticTemplate: Template {}
/// Implementors of this trait must guarantee that they will not declare any child.
pub unsafe trait LeafTemplate: Template {}

impl<'a> DeclareContext<'a> {
    pub(crate) fn new(
        metas_index: usize,
        segment_metas: &'a mut Vec<Vec<Arc<Meta>>>,
        world: &'a mut World,
    ) -> Self {
        Self {
            metas_index,
            segment_metas,
            world,
        }
    }

    pub fn owned(&mut self) -> DeclareContext {
        self.with(self.metas_index)
    }

    pub fn with(&mut self, metas_index: usize) -> DeclareContext {
        DeclareContext::new(metas_index, self.segment_metas, self.world)
    }

    pub fn meta(&mut self, get: GetMeta) -> Arc<Meta> {
        let meta = get.get(self.world);
        self.segment_metas[self.metas_index].push(meta.clone());
        meta
    }

    pub fn child<T>(&mut self, scope: impl FnOnce(usize, DeclareContext) -> T) -> T {
        let metas_index = self.segment_metas.len();
        self.segment_metas
            .push(vec![self.world.get_or_add_meta::<Entity>()]);
        scope(metas_index, self.with(metas_index))
    }
}

impl<'a> InitializeContext<'a> {
    pub(crate) const fn new(
        segment_index: usize,
        segment_indices: &'a [SegmentIndices],
        metas_to_segment: &'a HashMap<usize, usize>,
        world: &'a World,
    ) -> Self {
        Self {
            segment_index,
            segment_indices,
            metas_to_segment,
            world,
        }
    }

    pub fn segment(&self) -> &Segment {
        &self.world.segments[self.segment_indices[self.segment_index].segment]
    }

    pub fn owned(&mut self) -> InitializeContext {
        self.with(self.segment_index)
    }

    pub fn with(&mut self, segment_index: usize) -> InitializeContext {
        InitializeContext::new(
            segment_index,
            self.segment_indices,
            self.metas_to_segment,
            self.world,
        )
    }

    pub fn child<T>(
        &mut self,
        meta_index: usize,
        scope: impl FnOnce(usize, InitializeContext) -> T,
    ) -> T {
        let segment_index = self.metas_to_segment[&meta_index];
        scope(segment_index, self.with(segment_index))
    }
}

impl<'a> CountContext<'a> {
    pub(crate) fn new(
        segment_index: usize,
        segment_indices: &'a mut [SegmentIndices],
        entity_index: usize,
        entity_parent: Option<usize>,
        entity_previous: &'a mut Option<usize>,
        entity_indices: &'a mut Vec<EntityIndices>,
    ) -> Self {
        Self {
            segment_index,
            segment_indices,
            entity_index,
            entity_parent,
            entity_previous,
            entity_indices,
        }
    }

    pub fn owned(&mut self) -> CountContext {
        CountContext::new(
            self.segment_index,
            self.segment_indices,
            self.entity_index,
            self.entity_parent,
            self.entity_previous,
            self.entity_indices,
        )
    }

    pub fn with<'b>(
        &'b mut self,
        segment_index: usize,
        entity_index: usize,
        entity_parent: Option<usize>,
        entity_previous: &'b mut Option<usize>,
    ) -> CountContext {
        CountContext::new(
            segment_index,
            self.segment_indices,
            entity_index,
            entity_parent,
            entity_previous,
            self.entity_indices,
        )
    }

    pub fn child<T>(&mut self, segment_index: usize, scope: impl FnOnce(CountContext) -> T) -> T {
        let entity_index = self.entity_indices.len();
        let segment_indices = &mut self.segment_indices[segment_index];
        self.entity_indices.push(EntityIndices {
            segment: segment_index,
            offset: segment_indices.count,
            parent: self.entity_parent,
            previous_sibling: *self.entity_previous,
            next_sibling: None,
        });

        if let Some(previous) = self.entity_previous.replace(entity_index) {
            self.entity_indices[previous].next_sibling = Some(entity_index);
        }

        segment_indices.count += 1;
        scope(self.with(
            segment_index,
            entity_index,
            Some(self.entity_index),
            &mut None,
        ))
    }
}

impl<'a> ApplyContext<'a> {
    pub(crate) fn new(
        entity_root: usize,
        entity_index: usize,
        entity_parent: Option<u32>,
        entity_previous_sibling: &'a mut Option<u32>,
        entity_count: &'a mut usize,
        entity_instances: &'a [Entity],
        entity_indices: &'a [EntityIndices],
        entities: &'a mut Entities,
        store_index: usize,
        segment_indices: &'a [SegmentIndices],
    ) -> Self {
        Self {
            entity_root,
            entity_index,
            entity_parent,
            entity_previous_sibling,
            entity_count,
            entity_instances,
            entity_indices,
            entities,
            store_index,
            segment_indices,
        }
    }

    #[inline]
    pub fn entity(&self) -> Entity {
        self.family().entity()
    }

    #[inline]
    pub const fn family(&self) -> Family {
        Family::new(
            self.entity_root,
            self.entity_index,
            self.entity_instances,
            self.entity_indices,
            self.segment_indices,
        )
    }

    #[inline]
    pub const fn store_index(&self) -> usize {
        self.store_index
    }

    pub fn owned(&mut self) -> ApplyContext {
        ApplyContext::new(
            self.entity_root,
            self.entity_index,
            self.entity_parent,
            self.entity_previous_sibling,
            self.entity_count,
            self.entity_instances,
            self.entity_indices,
            self.entities,
            self.store_index,
            self.segment_indices,
        )
    }

    pub fn with<'b>(
        &'b mut self,
        entity_index: usize,
        entity_parent: Option<u32>,
        entity_previous_sibling: &'b mut Option<u32>,
        store_index: usize,
    ) -> ApplyContext {
        ApplyContext::new(
            self.entity_root,
            entity_index,
            entity_parent,
            entity_previous_sibling,
            self.entity_count,
            self.entity_instances,
            self.entity_indices,
            self.entities,
            store_index,
            self.segment_indices,
        )
    }

    pub(crate) fn child<T>(&mut self, scope: impl FnOnce(ApplyContext) -> T) -> T {
        let entity_index = *self.entity_count;
        let entity_indices = &self.entity_indices[entity_index];
        let segment_indices = &self.segment_indices[entity_indices.segment];
        let segment_offset = segment_indices.count * self.entity_root + entity_indices.offset;
        let instance_index = segment_indices.index + segment_offset;
        let store_index = segment_indices.store + segment_offset;
        let segment_index = segment_indices.segment;
        let entity_instance = self.entity_instances[instance_index];
        let entity_parent = self.entity_parent;
        let entity_previous_sibling = *self.entity_previous_sibling;

        *self.entity_count += 1;
        *self.entity_previous_sibling = Some(entity_instance.index());

        if let Some(previous) = entity_previous_sibling {
            let previous = self.entities.get_datum_at_mut(previous).unwrap();
            previous.next_sibling = entity_instance.index();
        }

        if let Some(parent) = entity_parent {
            let parent = &mut self.entities.get_datum_at_mut(parent).unwrap();
            if entity_previous_sibling.is_none() {
                parent.first_child = entity_instance.index();
            }
            if entity_indices.next_sibling.is_none() {
                parent.last_child = entity_instance.index();
            }
        }

        self.entities
            .get_datum_at_mut(entity_instance.index())
            .unwrap()
            .initialize(
                entity_instance.generation(),
                store_index as u32,
                segment_index as u32,
                entity_parent,
                None,
                None,
                entity_previous_sibling,
                None,
            );
        scope(self.with(
            entity_index,
            Some(entity_instance.index()),
            &mut None,
            store_index,
        ))
    }
}

impl GetMeta {
    pub fn new<T: Send + Sync + 'static>() -> Self {
        Self(|world| world.get_or_add_meta::<T>())
    }

    pub fn get(&self, world: &mut World) -> Arc<Meta> {
        (self.0)(world)
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Add<T>(T);

#[inline]
pub fn add<C: Send + Sync + 'static>(component: C) -> Add<C> {
    Add(component)
}

#[inline]
pub fn add_default<C: Default + Send + Sync + 'static>() -> Add<C> {
    add(C::default())
}

impl<T: Send + Sync + 'static> Template for Add<T> {
    type Input = ();
    type Declare = Arc<Meta>;
    type State = Arc<Store>;

    #[inline]
    fn declare(_: Self::Input, mut context: DeclareContext) -> Self::Declare {
        context.meta(GetMeta::new::<T>())
    }

    #[inline]
    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        context.segment().store(&state).unwrap()
    }

    #[inline]
    fn static_count(_: &Self::State, _: CountContext) -> bool {
        true
    }

    #[inline]
    fn dynamic_count(&self, _: &Self::State, _: CountContext) {}

    #[inline]
    fn apply(self, state: &Self::State, context: ApplyContext) {
        unsafe { state.set(context.store_index(), self.0) }
    }
}

unsafe impl<T: Send + Sync + 'static> StaticTemplate for Add<T> {}
unsafe impl<T: Send + Sync + 'static> LeafTemplate for Add<T> {}

impl<T: Template> Template for Vec<T> {
    type Input = T::Input;
    type Declare = T::Declare;
    type State = T::State;

    #[inline]
    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        T::declare(input, context)
    }

    #[inline]
    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        T::initialize(state, context)
    }

    #[inline]
    fn static_count(_: &Self::State, _: CountContext) -> bool {
        false
    }

    #[inline]
    fn dynamic_count(&self, state: &Self::State, mut context: CountContext) {
        for value in self {
            value.dynamic_count(state, context.owned());
        }
    }

    #[inline]
    fn apply(self, state: &Self::State, mut context: ApplyContext) {
        for value in self {
            value.apply(state, context.owned());
        }
    }
}

unsafe impl<T: LeafTemplate> StaticTemplate for Vec<T> {}
unsafe impl<T: LeafTemplate> LeafTemplate for Vec<T> {}

impl<T: Template, const N: usize> Template for [T; N] {
    type Input = T::Input;
    type Declare = T::Declare;
    type State = T::State;

    #[inline]
    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        T::declare(input, context)
    }

    #[inline]
    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        T::initialize(state, context)
    }

    #[inline]
    fn static_count(state: &Self::State, mut context: CountContext) -> bool {
        (0..N).all(|_| T::static_count(state, context.owned()))
    }

    #[inline]
    fn dynamic_count(&self, state: &Self::State, mut context: CountContext) {
        self.iter()
            .for_each(|value| value.dynamic_count(state, context.owned()));
    }

    #[inline]
    fn apply(self, state: &Self::State, mut context: ApplyContext) {
        IntoIter::new(self).for_each(|value| value.apply(state, context.owned()))
    }
}

unsafe impl<T: StaticTemplate, const N: usize> StaticTemplate for [T; N] {}
unsafe impl<T: LeafTemplate, const N: usize> LeafTemplate for [T; N] {}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct With<F, T>(F, PhantomData<T>);

#[inline]
pub fn with<F: FnOnce(Family) -> T, T: StaticTemplate>(with: F) -> With<F, T> {
    With(with, PhantomData)
}

impl<F: FnOnce(Family) -> T, T: StaticTemplate> Template for With<F, T> {
    type Input = T::Input;
    type Declare = T::Declare;
    type State = T::State;

    #[inline]
    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        T::declare(input, context)
    }

    #[inline]
    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        T::initialize(state, context)
    }

    #[inline]
    fn static_count(state: &Self::State, context: CountContext) -> bool {
        T::static_count(state, context)
    }

    #[inline]
    fn dynamic_count(&self, state: &Self::State, context: CountContext) {
        Self::static_count(state, context);
    }

    #[inline]
    fn apply(self, store: &Self::State, context: ApplyContext) {
        self.0(context.family()).apply(store, context)
    }
}

unsafe impl<F: FnOnce(Family) -> T, T: StaticTemplate> StaticTemplate for With<F, T> {}
unsafe impl<F: FnOnce(Family) -> T, T: StaticTemplate + LeafTemplate> LeafTemplate for With<F, T> {}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Spawn<T>(T);

#[inline]
pub fn spawn<T: Template>(child: T) -> Spawn<T> {
    Spawn(child)
}

impl<T: Template> Template for Spawn<T> {
    type Input = T::Input;
    type Declare = (usize, T::Declare);
    type State = (usize, T::State);

    fn declare(input: Self::Input, mut context: DeclareContext) -> Self::Declare {
        context.child(|index, context| (index, T::declare(input, context)))
    }

    fn initialize((index, state): Self::Declare, mut context: InitializeContext) -> Self::State {
        context.child(index, |index, context| {
            (index, T::initialize(state, context))
        })
    }

    fn static_count((index, state): &Self::State, mut context: CountContext) -> bool {
        context.child(*index, |context| T::static_count(state, context))
    }

    fn dynamic_count(&self, (index, state): &Self::State, mut context: CountContext) {
        context.child(*index, |context| self.0.dynamic_count(state, context))
    }

    fn apply(self, (_, state): &Self::State, mut context: ApplyContext) {
        context.child(|context| self.0.apply(state, context))
    }
}

unsafe impl<T: StaticTemplate> StaticTemplate for Spawn<T> {}

macro_rules! template {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Template,)*> Template for ($($t,)*) {
            type Input = ($($t::Input,)*);
            type Declare = ($($t::Declare,)*);
            type State = ($($t::State,)*);

            #[inline]
            fn declare(($($p,)*): Self::Input, mut _context: DeclareContext) -> Self::Declare {
                ($($t::declare($p, _context.owned()),)*)
            }

            #[inline]
            fn initialize(($($p,)*): Self::Declare, mut _context: InitializeContext) -> Self::State {
                ($($t::initialize($p, _context.owned()),)*)
            }

            #[inline]
            fn static_count(($($t,)*): &Self::State, mut _context: CountContext) -> bool {
                $($t::static_count($t, _context.owned()) &&)* true
            }

            #[inline]
            fn dynamic_count(&self, ($($t,)*): &Self::State, mut _context: CountContext) {
                let ($($p,)*) = self;
                $($p.dynamic_count($t, _context.owned());)*
            }

            #[inline]
            fn apply(self, ($($t,)*): &Self::State, mut _context: ApplyContext) {
                let ($($p,)*) = self;
                $($p.apply($t, _context.owned());)*
            }
        }

        unsafe impl<$($t: StaticTemplate,)*> StaticTemplate for ($($t,)*) {}
        unsafe impl<$($t: LeafTemplate,)*> LeafTemplate for ($($t,)*) {}
    };
}

entia_macro::recurse_64!(template);

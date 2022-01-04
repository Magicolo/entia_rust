use entia_core::Marker;

use crate::{
    entities::Entities,
    entity::Entity,
    error::{Error, Result},
    family::template::{EntityIndices, Family, SegmentIndices},
    world::{meta::Meta, segment::Segment, store::Store, World},
};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};

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
    fn static_count(state: &Self::State, context: CountContext) -> Result<bool>;
    fn dynamic_count(&self, state: &Self::State, context: CountContext);
    fn apply(self, state: &Self::State, context: ApplyContext);
}

/// SAFETY: Implementors of this trait must guarantee that the 'Template::static_count' function always succeeds.
pub unsafe trait StaticTemplate: Template {}
/// SAFETY: Implementors of this trait must guarantee that they will not declare any child.
pub unsafe trait LeafTemplate: Template {}
/// SAFETY: Implementors of this trait must guarantee that they will do nothing else other than spawn a child.
/// A wrong implementation of this trait can lead to uninitialized data.
pub unsafe trait SpawnTemplate: Template {}

/// Serves only as a hack to allow 'trivial_bounds' in the 'Template' derive macro.
pub struct StaticMarker;
impl<T: StaticTemplate> Marker<T> for StaticMarker {}
/// Serves only as a hack to allow 'trivial_bounds' in the 'Template' derive macro.
pub struct LeafMarker;
impl<T: LeafTemplate> Marker<T> for LeafMarker {}
/// Serves only as a hack to allow 'trivial_bounds' in the 'Template' derive macro.
pub struct SpawnMarker;
impl<T: SpawnTemplate> Marker<T> for SpawnMarker {}

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

    pub fn meta<T: Send + Sync + 'static>(&mut self) -> Arc<Meta> {
        let meta = self.world.get_or_add_meta::<T>();
        self.segment_metas[self.metas_index].push(meta.clone());
        meta
    }

    pub fn child<T>(&mut self, scope: impl FnOnce(usize, DeclareContext) -> T) -> T {
        let metas_index = self.segment_metas.len();
        self.segment_metas.push(Vec::new());
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
            .expect("Entity index must be in range.")
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

unsafe impl<T: SpawnTemplate> SpawnTemplate for Option<T> {}
unsafe impl<T: SpawnTemplate + StaticTemplate> StaticTemplate for Option<T> {}
unsafe impl<T: SpawnTemplate + LeafTemplate> LeafTemplate for Option<T> {}

impl<T: SpawnTemplate> Template for Option<T> {
    type Input = T::Input;
    type Declare = T::Declare;
    type State = T::State;

    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        T::declare(input, context)
    }

    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        T::initialize(state, context)
    }

    fn static_count(_: &Self::State, _: CountContext) -> Result<bool> {
        Ok(false)
    }

    #[inline]
    fn dynamic_count(&self, state: &Self::State, context: CountContext) {
        if let Some(value) = self {
            value.dynamic_count(state, context);
        }
    }

    #[inline]
    fn apply(self, state: &Self::State, context: ApplyContext) {
        if let Some(value) = self {
            value.apply(state, context);
        }
    }
}

unsafe impl<T: SpawnTemplate> SpawnTemplate for Vec<T> {}
unsafe impl<T: SpawnTemplate + StaticTemplate> StaticTemplate for Vec<T> {}
unsafe impl<T: SpawnTemplate + LeafTemplate> LeafTemplate for Vec<T> {}

impl<T: SpawnTemplate> Template for Vec<T> {
    type Input = T::Input;
    type Declare = T::Declare;
    type State = T::State;

    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        T::declare(input, context)
    }

    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        T::initialize(state, context)
    }

    fn static_count(_: &Self::State, _: CountContext) -> Result<bool> {
        Ok(false)
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

unsafe impl<T: SpawnTemplate, const N: usize> SpawnTemplate for [T; N] {}
unsafe impl<T: SpawnTemplate + StaticTemplate, const N: usize> StaticTemplate for [T; N] {}
unsafe impl<T: SpawnTemplate + LeafTemplate, const N: usize> LeafTemplate for [T; N] {}

impl<T: SpawnTemplate, const N: usize> Template for [T; N] {
    type Input = T::Input;
    type Declare = T::Declare;
    type State = T::State;

    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        T::declare(input, context)
    }

    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        T::initialize(state, context)
    }

    fn static_count(state: &Self::State, mut context: CountContext) -> Result<bool> {
        for _ in 0..N {
            if !T::static_count(state, context.owned())? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    #[inline]
    fn dynamic_count(&self, state: &Self::State, mut context: CountContext) {
        self.iter()
            .for_each(|value| value.dynamic_count(state, context.owned()));
    }

    #[inline]
    fn apply(self, state: &Self::State, mut context: ApplyContext) {
        self.into_iter()
            .for_each(|value| value.apply(state, context.owned()))
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Add<T>(T);

unsafe impl<T: Send + Sync + 'static> StaticTemplate for Add<T> {}
unsafe impl<T: Send + Sync + 'static> LeafTemplate for Add<T> {}

impl<T> Add<T> {
    #[inline]
    pub const fn new(component: T) -> Self {
        Self(component)
    }
}

impl<T> From<T> for Add<T> {
    #[inline]
    fn from(component: T) -> Self {
        Add::new(component)
    }
}

impl<T: Send + Sync + 'static> Template for Add<T> {
    type Input = ();
    type Declare = Arc<Meta>;
    type State = Arc<Store>;

    fn declare(_: Self::Input, mut context: DeclareContext) -> Self::Declare {
        context.meta::<T>()
    }

    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        context.segment().component_store(&state).unwrap()
    }

    fn static_count(_: &Self::State, _: CountContext) -> Result<bool> {
        Ok(true)
    }

    #[inline]
    fn dynamic_count(&self, _: &Self::State, _: CountContext) {}

    #[inline]
    fn apply(self, state: &Self::State, context: ApplyContext) {
        unsafe { state.set(context.store_index(), self.0) }
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct With<T, F = fn(Family) -> T>(F, PhantomData<T>);

unsafe impl<T: StaticTemplate + SpawnTemplate, F: FnOnce(Family) -> T> SpawnTemplate
    for With<T, F>
{
}
unsafe impl<T: StaticTemplate, F: FnOnce(Family) -> T> StaticTemplate for With<T, F> {}
unsafe impl<T: StaticTemplate + LeafTemplate, F: FnOnce(Family) -> T> LeafTemplate for With<T, F> {}

impl<T, F: FnOnce(Family) -> T> With<T, F> {
    #[inline]
    pub fn new(with: F) -> Self {
        Self(with, PhantomData)
    }
}

impl<T, F: FnOnce(Family) -> T> From<F> for With<T, F> {
    #[inline]
    fn from(with: F) -> Self {
        With::new(with)
    }
}

impl<T: StaticTemplate, F: FnOnce(Family) -> T> Template for With<T, F> {
    type Input = T::Input;
    type Declare = T::Declare;
    type State = T::State;

    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        T::declare(input, context)
    }

    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        T::initialize(state, context)
    }

    fn static_count(state: &Self::State, context: CountContext) -> Result<bool> {
        if T::static_count(state, context)? {
            Ok(true)
        } else {
            Err(Error::StaticCountMustBeTrue)
        }
    }

    #[inline]
    fn dynamic_count(&self, state: &Self::State, context: CountContext) {
        T::static_count(state, context).expect("'static_count' must succeed.");
    }

    #[inline]
    fn apply(self, store: &Self::State, context: ApplyContext) {
        self.0(context.family()).apply(store, context)
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Spawn<T>(T);

unsafe impl<T: Template> SpawnTemplate for Spawn<T> {}
unsafe impl<T: StaticTemplate> StaticTemplate for Spawn<T> {}

impl<T> Spawn<T> {
    #[inline]
    pub const fn new(child: T) -> Self {
        Self(child)
    }
}

impl<T> From<T> for Spawn<T> {
    #[inline]
    fn from(child: T) -> Self {
        Spawn::new(child)
    }
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

    fn static_count((index, state): &Self::State, mut context: CountContext) -> Result<bool> {
        context.child(*index, |context| T::static_count(state, context))
    }

    fn dynamic_count(&self, (index, state): &Self::State, mut context: CountContext) {
        context.child(*index, |context| self.0.dynamic_count(state, context))
    }

    fn apply(self, (_, state): &Self::State, mut context: ApplyContext) {
        context.child(|context| self.0.apply(state, context))
    }
}

impl<T> Template for PhantomData<T> {
    type Input = <() as Template>::Input;
    type Declare = <() as Template>::Declare;
    type State = <() as Template>::State;

    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        <() as Template>::declare(input, context)
    }
    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        <() as Template>::initialize(state, context)
    }
    fn static_count(state: &Self::State, context: CountContext) -> Result<bool> {
        <() as Template>::static_count(state, context)
    }
    #[inline]
    fn dynamic_count(&self, state: &Self::State, context: CountContext) {
        ().dynamic_count(state, context)
    }
    #[inline]
    fn apply(self, state: &Self::State, context: ApplyContext) {
        ().apply(state, context)
    }
}

macro_rules! template {
    ($($p:ident, $t:ident),*) => {
        unsafe impl<$($t: SpawnTemplate,)*> SpawnTemplate for ($($t,)*) {}
        unsafe impl<$($t: StaticTemplate,)*> StaticTemplate for ($($t,)*) {}
        unsafe impl<$($t: LeafTemplate,)*> LeafTemplate for ($($t,)*) {}

        impl<$($t: Template,)*> Template for ($($t,)*) {
            type Input = ($($t::Input,)*);
            type Declare = ($($t::Declare,)*);
            type State = ($($t::State,)*);

            fn declare(($($p,)*): Self::Input, mut _context: DeclareContext) -> Self::Declare {
                ($($t::declare($p, _context.owned()),)*)
            }

            fn initialize(($($p,)*): Self::Declare, mut _context: InitializeContext) -> Self::State {
                ($($t::initialize($p, _context.owned()),)*)
            }

            fn static_count(($($p,)*): &Self::State, mut _context: CountContext) -> Result<bool> {
                Ok($($t::static_count($p, _context.owned())? &&)* true)
            }

            #[inline]
            fn dynamic_count(&self, ($($p,)*): &Self::State, mut _context: CountContext) {
                let ($($t,)*) = self;
                $($t.dynamic_count($p, _context.owned());)*
            }

            #[inline]
            fn apply(self, ($($p,)*): &Self::State, mut _context: ApplyContext) {
                let ($($t,)*) = self;
                $($t.apply($p, _context.owned());)*
            }
        }
    };
}

entia_macro::recurse_16!(template);

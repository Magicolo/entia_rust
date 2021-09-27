use std::{
    array::IntoIter,
    collections::HashMap,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    component::Component,
    entity::Entity,
    family::{
        initial::{EntityIndices, Family, SegmentIndices},
        item,
    },
    segment::{Segment, Store},
    world::{Meta, World},
};

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
    entity_parent: Option<(usize, usize)>,
    entity_count: &'a mut usize,
    entity_instances: &'a [Entity],
    entity_indices: &'a [EntityIndices],
    segment_indices: &'a [SegmentIndices],
}

pub unsafe trait Initial: Send + 'static {
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
pub unsafe trait StaticInitial: Initial {}
/// Implementors of this trait must guarantee that they will not declare any child.
pub unsafe trait LeafInitial: Initial {}

impl<'a> DeclareContext<'a> {
    pub fn new(
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
        self.segment_metas.push(Vec::new());
        scope(metas_index, self.with(metas_index))
    }
}

impl<'a> InitializeContext<'a> {
    pub const fn new(
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
    pub fn new(
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

    pub fn with<'b, 'c: 'b>(
        &'b mut self,
        segment_index: usize,
        entity_index: usize,
        entity_parent: Option<usize>,
        entity_previous: &'c mut Option<usize>,
    ) -> CountContext<'b> {
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
            next: None,
        });

        if let Some(previous) = self.entity_previous.replace(entity_index) {
            self.entity_indices[previous].next = Some(entity_index);
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
    pub fn new(
        entity_root: usize,
        entity_index: usize,
        entity_parent: Option<(usize, usize)>,
        entity_count: &'a mut usize,
        entity_instances: &'a [Entity],
        entity_indices: &'a [EntityIndices],
        segment_indices: &'a [SegmentIndices],
    ) -> Self {
        Self {
            entity_root,
            entity_index,
            entity_parent,
            entity_count,
            entity_instances,
            entity_indices,
            segment_indices,
        }
    }

    pub fn entity(&self) -> Entity {
        self.family().entity()
    }

    pub fn family(&self) -> Family {
        Family::new(
            self.entity_root,
            self.entity_index,
            self.entity_instances,
            self.entity_indices,
            self.segment_indices,
        )
    }

    pub fn store_index(&self) -> Option<usize> {
        Some(self.entity_parent?.1)
    }

    pub fn owned(&mut self) -> ApplyContext {
        self.with(self.entity_index, self.entity_parent)
    }

    pub fn with(
        &mut self,
        entity_index: usize,
        entity_parent: Option<(usize, usize)>,
    ) -> ApplyContext {
        ApplyContext::new(
            self.entity_root,
            entity_index,
            entity_parent,
            self.entity_count,
            self.entity_instances,
            self.entity_indices,
            self.segment_indices,
        )
    }

    pub(crate) fn child<T>(
        &mut self,
        scope: impl FnOnce(Entity, item::Link, ApplyContext) -> T,
    ) -> T {
        let entity_index = *self.entity_count;
        let (entity_parent, entity_next) = self.get_indices(self.entity_index);
        let entity_next = entity_next.map(|next| self.get_indices(next).0);
        *self.entity_count += 1;

        scope(
            self.entity_instances[entity_index],
            item::Link {
                parent: self.entity_parent,
                child: Some(entity_parent),
                next: entity_next,
            },
            self.with(entity_index, Some(entity_parent)),
        )
    }

    fn get_indices(&self, entity_index: usize) -> ((usize, usize), Option<usize>) {
        let entity_indices = &self.entity_indices[entity_index];
        let segment_indices = &self.segment_indices[entity_indices.segment];
        let segment_offset = segment_indices.count * self.entity_root + entity_indices.offset;
        let store_index = segment_indices.store + segment_offset;
        ((entity_indices.segment, store_index), entity_indices.next)
    }
}

impl GetMeta {
    pub fn new<T: 'static>() -> Self {
        Self(|world| world.get_or_add_meta::<T>())
    }

    pub fn get(&self, world: &mut World) -> Arc<Meta> {
        (self.0)(world)
    }
}

unsafe impl<C: Component> Initial for C {
    type Input = ();
    type Declare = Arc<Meta>;
    type State = Arc<Store>;

    fn declare(_: Self::Input, mut context: DeclareContext) -> Self::Declare {
        context.meta(GetMeta::new::<C>())
    }

    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        context.segment().store(&state).unwrap()
    }

    fn static_count(_: &Self::State, _: CountContext) -> bool {
        true
    }

    fn dynamic_count(&self, _: &Self::State, _: CountContext) {}

    fn apply(self, state: &Self::State, context: ApplyContext) {
        unsafe { state.set(context.store_index().unwrap(), self) }
    }
}

unsafe impl<C: Component> StaticInitial for C {}
unsafe impl<C: Component> LeafInitial for C {}

unsafe impl<I: Initial> Initial for Vec<I> {
    type Input = I::Input;
    type Declare = I::Declare;
    type State = I::State;

    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        I::declare(input, context)
    }

    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        I::initialize(state, context)
    }

    fn static_count(_: &Self::State, _: CountContext) -> bool {
        false
    }

    fn dynamic_count(&self, state: &Self::State, mut context: CountContext) {
        for value in self {
            value.dynamic_count(state, context.owned());
        }
    }

    fn apply(self, state: &Self::State, mut context: ApplyContext) {
        for value in self {
            value.apply(state, context.owned());
        }
    }
}

unsafe impl<I: LeafInitial> StaticInitial for Vec<I> {}
unsafe impl<I: LeafInitial> LeafInitial for Vec<I> {}

unsafe impl<I: Initial, const N: usize> Initial for [I; N] {
    type Input = I::Input;
    type Declare = I::Declare;
    type State = I::State;

    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        I::declare(input, context)
    }

    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        I::initialize(state, context)
    }

    fn static_count(state: &Self::State, mut context: CountContext) -> bool {
        (0..N).all(|_| I::static_count(state, context.owned()))
    }

    fn dynamic_count(&self, state: &Self::State, mut context: CountContext) {
        self.iter()
            .for_each(|value| value.dynamic_count(state, context.owned()));
    }

    fn apply(self, state: &Self::State, mut context: ApplyContext) {
        IntoIter::new(self).for_each(|value| value.apply(state, context.owned()))
    }
}

unsafe impl<I: StaticInitial, const N: usize> StaticInitial for [I; N] {}
unsafe impl<I: LeafInitial, const N: usize> LeafInitial for [I; N] {}

pub struct With<T, F>(F, PhantomData<T>);

unsafe impl<I: StaticInitial, F: FnOnce(Family) -> I + Send + 'static> Initial for With<I, F> {
    type Input = I::Input;
    type Declare = I::Declare;
    type State = I::State;

    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        I::declare(input, context)
    }

    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        I::initialize(state, context)
    }

    fn static_count(state: &Self::State, context: CountContext) -> bool {
        I::static_count(state, context)
    }

    fn dynamic_count(&self, _: &Self::State, _: CountContext) {}

    fn apply(self, store: &Self::State, context: ApplyContext) {
        self.0(context.family()).apply(store, context)
    }
}

unsafe impl<I: StaticInitial, F: FnOnce(Family) -> I + Send + 'static> StaticInitial
    for With<I, F>
{
}

unsafe impl<I: StaticInitial + LeafInitial, F: FnOnce(Family) -> I + Send + 'static> LeafInitial
    for With<I, F>
{
}

impl<I, F: Clone> Clone for With<I, F> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

#[inline]
pub fn with<I: StaticInitial, F: FnOnce(Family) -> I + Send + 'static>(with: F) -> With<I, F> {
    With(with, PhantomData)
}

#[inline]
pub fn spawn<I: Initial>(initial: I) -> Spawn<I> {
    Spawn(initial)
}

pub struct Spawn<T>(T);

unsafe impl<I: Initial> Initial for Spawn<I> {
    type Input = I::Input;
    type Declare = (usize, Arc<Meta>, Arc<Meta>, I::Declare);
    type State = (usize, Arc<Store>, Arc<Store>, I::State);

    fn declare(input: Self::Input, mut context: DeclareContext) -> Self::Declare {
        context.child(|index, mut context| {
            (
                index,
                context.meta(GetMeta::new::<Entity>()),
                context.meta(GetMeta::new::<item::Link>()),
                I::declare(input, context),
            )
        })
    }

    fn initialize(
        (index, entity_meta, family_meta, state): Self::Declare,
        mut context: InitializeContext,
    ) -> Self::State {
        context.child(index, |index, context| {
            let segment = context.segment();
            (
                index,
                segment.store(&entity_meta).unwrap(),
                segment.store(&family_meta).unwrap(),
                I::initialize(state, context),
            )
        })
    }

    fn static_count((index, _, _, state): &Self::State, mut context: CountContext) -> bool {
        context.child(*index, |context| I::static_count(state, context))
    }

    fn dynamic_count(&self, (index, _, _, state): &Self::State, mut context: CountContext) {
        context.child(*index, |context| self.0.dynamic_count(state, context))
    }

    fn apply(
        self,
        (_, entity_store, family_store, state): &Self::State,
        mut context: ApplyContext,
    ) {
        context.child(|entity, family, context| {
            let index = context.store_index().unwrap();
            unsafe { entity_store.set(index, entity) };
            unsafe { family_store.set(index, family) };
            self.0.apply(state, context)
        })
    }
}

unsafe impl<I: StaticInitial> StaticInitial for Spawn<I> {}

impl<T: Clone> Clone for Spawn<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Deref for Spawn<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for Spawn<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T> AsRef<T> for Spawn<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsMut<T> for Spawn<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

macro_rules! modify {
    ($($p:ident, $t:ident),*) => {
        unsafe impl<$($t: Initial,)*> Initial for ($($t,)*) {
            type Input = ($($t::Input,)*);
            type Declare = ($($t::Declare,)*);
            type State = ($($t::State,)*);

            fn declare(($($p,)*): Self::Input, mut _context: DeclareContext) -> Self::Declare {
                ($($t::declare($p, _context.owned()),)*)
            }

            fn initialize(($($p,)*): Self::Declare, mut _context: InitializeContext) -> Self::State {
                ($($t::initialize($p, _context.owned()),)*)
            }

            fn static_count(($($t,)*): &Self::State, mut _context: CountContext) -> bool {
                $($t::static_count($t, _context.owned()) &&)* true
            }

            fn dynamic_count(&self, ($($t,)*): &Self::State, mut _context: CountContext) {
                let ($($p,)*) = self;
                $($p.dynamic_count($t, _context.owned());)*
            }

            fn apply(self, ($($t,)*): &Self::State, mut _context: ApplyContext) {
                let ($($p,)*) = self;
                $($p.apply($t, _context.owned());)*
            }
        }

        unsafe impl<$($t: StaticInitial,)*> StaticInitial for ($($t,)*) {}
        unsafe impl<$($t: LeafInitial,)*> LeafInitial for ($($t,)*) {}
    };
}

entia_macro::recurse_32!(modify);

/*
impl Initial for Box<dyn Any + Send> {
    type Input = GetMeta;
    type Declare = Arc<Meta>;
    type State = Arc<Store>;

    fn declare(input: Self::Input, mut context: DeclareContext) -> Self::Declare {
        context.meta(input)
    }

    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State {
        context.segment().store(&state).unwrap()
    }

    fn static_count(_: &Self::State, _: CountContext) -> bool {
        true
    }

    fn dynamic_count(&self, _: &Self::State, _: CountContext) {}

    fn apply(self, state: &Self::State, context: ApplyContext) {
        unsafe { state.set_any(context.store_index(), self) }
    }
}

unsafe impl StaticInitial for Box<dyn Any + Send> {}
unsafe impl LeafInitial for Box<dyn Any + Send> {}

pub struct Dynamic<T> {
    provide: fn(T) -> Box<dyn Any>,
    declare: fn(DeclareContext) -> Box<dyn Any>,
    initialize: fn(Box<dyn Any>, InitializeContext) -> Box<dyn Any>,
    static_count: fn(&Box<dyn Any>, CountContext) -> bool,
    dynamic_count: fn(&Box<dyn Any + Send>, &Box<dyn Any>, CountContext),
    apply: fn(Box<dyn Any + Send>, &Box<dyn Any>, ApplyContext),
}

impl<T> Initial for T {
    type Input = Dynamic<T>;
    type Declare = (Box<dyn Any>, Dynamic<T>);
    type State = (Box<dyn Any>, Dynamic<T>);

    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare {
        ((input.declare)(context), input)
    }

    fn initialize((state, dynamic): Self::Declare, context: InitializeContext) -> Self::State {
        ((dynamic.initialize)(state, context), dynamic)
    }

    fn static_count((state, dynamic): &Self::State, context: CountContext) -> bool {
        (dynamic.static_count)(state, context)
    }

    fn dynamic_count(&self, (state, dynamic): &Self::State, context: CountContext) {
        (dynamic.dynamic_count)(self, state, context)
    }

    fn apply(self, (state, dynamic): &Self::State, context: ApplyContext) {
        (dynamic.apply)(self, state, context)
    }
}

pub fn dynamic<T, I: Initial<Input = impl Default>>(provide: fn(T) -> I) -> Dynamic<T> {
    Dynamic {
        provide,
        declare: |context| Box::new(I::declare(I::Input::default(), context)),
        initialize: |state, context| Box::new(I::initialize(*state.downcast().unwrap(), context)),
        static_count: |state, context| I::static_count(state.downcast_ref().unwrap(), context),
        dynamic_count: |initial, state, context| {
            initial
                .downcast_ref::<I>()
                .unwrap()
                .dynamic_count(state.downcast_ref().unwrap(), context)
        },
        apply: |initial, state, context| {
            initial
                .downcast::<I>()
                .unwrap()
                .apply(state.downcast_ref().unwrap(), context)
        },
    }
}


INJECT
Family: [Defer(Entity*)]
Destroy: [Defer(Entity*)]

QUERY
Root<I>
Parent<I>
Child<I>
Children<I>
Sibling<I>
Siblings<I>
Ancestor<I>
Ancestors<I>
Descendant<I>
Descendants<I>


Create<With<(Insect, Child<Head>, Child<Torso>, Child<Abdomen>)>>
Create<_>

struct Insect(Entity, Entity, Entity);
struct Head;
struct Torso(usize);
struct Abdomen(usize);
struct Antenna;
struct Leg;
impl Component for Insect, Head, Torso, Abdomen, Antenna, Leg {}

fn insect(height: usize, antennas: usize) -> impl Initial {
    with(|family| {
        let entity = family.entity();
        let parent = family.parent();
        let root = family.root();
        let ancestors = family.ancestors();
        let descendants = family.descendants();
        let children = family.children();
        let siblings = family.siblings();
        (
            Insect(children[0], children[1], children[2]),
            [(Head, vec![Antenna; antennas])],
            [(Torso(height / 2), [Leg; 4])],
            [(Abdomen(height / 2), [Leg; 2])]
        )
    })
}
*/

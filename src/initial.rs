use std::{any::Any, array::IntoIter, collections::HashMap, marker::PhantomData, sync::Arc};

use crate::{
    component::Component,
    create::Family,
    entity::Entity,
    segment::{Segment, Store},
    world::{Meta, World},
};

pub struct GetMeta(fn(&mut World) -> Arc<Meta>);

pub struct Indices {
    pub offset: usize,
    pub segment: usize,
    pub parent: Option<usize>,
    pub next: Option<usize>,
}

pub struct DeclareContext<'a> {
    meta_index: usize,
    segment_metas: &'a mut Vec<Vec<Arc<Meta>>>,
    world: &'a mut World,
}

pub struct InitializeContext<'a> {
    segment_index: usize,
    segments: &'a Vec<usize>,
    metas_to_segment: &'a HashMap<usize, usize>,
    world: &'a World,
}

pub struct CountContext<'a> {
    segment_index: usize,
    segment_counts: &'a mut Vec<usize>,
    entity_index: usize,
    entity_parent: Option<usize>,
    entity_previous: Option<usize>,
    entity_indices: &'a mut Vec<Indices>,
}

// TODO: can this context have access to the current entity and/or the whole hierarchy?
pub struct ApplyContext<'a> {
    segment_index: usize,
    segment_indices: &'a Vec<usize>,
    store_indices: &'a Vec<usize>,
    entity_index: usize,
    entity_count: &'a mut usize,
    entity_indices: &'a [Indices],
    entity_instances: &'a [Entity],
}

pub trait Initial: Send + 'static {
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
pub unsafe trait Static: Initial {}
/// Implementors of this trait must guarantee that they will not declare any child.
pub unsafe trait Leaf: Initial {}

impl<'a> DeclareContext<'a> {
    pub fn new(
        segment_index: usize,
        segment_metas: &'a mut Vec<Vec<Arc<Meta>>>,
        world: &'a mut World,
    ) -> Self {
        Self {
            meta_index: segment_index,
            segment_metas,
            world,
        }
    }

    pub fn owned(&mut self) -> DeclareContext {
        self.with(self.meta_index)
    }

    pub fn with(&mut self, meta_index: usize) -> DeclareContext {
        DeclareContext::new(meta_index, self.segment_metas, self.world)
    }

    pub fn meta(&mut self, get: GetMeta) -> Arc<Meta> {
        let meta = get.get(self.world);
        self.segment_metas[self.meta_index].push(meta.clone());
        meta
    }

    pub fn child<T>(&mut self, scope: impl FnOnce(DeclareContext) -> T) -> (usize, T) {
        let meta_index = self.segment_metas.len();
        let metas = vec![self.world.get_or_add_meta::<Entity>()];
        self.segment_metas.push(metas);
        (meta_index, scope(self.with(meta_index)))
    }
}

impl<'a> InitializeContext<'a> {
    pub const fn new(
        segment_index: usize,
        segments: &'a Vec<usize>,
        metas_to_segment: &'a HashMap<usize, usize>,
        world: &'a World,
    ) -> Self {
        Self {
            segment_index,
            segments,
            metas_to_segment,
            world,
        }
    }

    pub fn segment(&self) -> &Segment {
        &self.world.segments[self.segments[self.segment_index]]
    }

    pub fn owned(&mut self) -> InitializeContext {
        self.with(self.segment_index)
    }

    pub fn with(&mut self, segment_index: usize) -> InitializeContext {
        InitializeContext::new(
            segment_index,
            self.segments,
            self.metas_to_segment,
            self.world,
        )
    }

    pub fn child<T>(
        &mut self,
        meta_index: usize,
        scope: impl FnOnce(InitializeContext) -> T,
    ) -> (usize, T) {
        let segment_index = self.metas_to_segment[&meta_index];
        (segment_index, scope(self.with(segment_index)))
    }
}

impl<'a> CountContext<'a> {
    pub fn new(
        segment_index: usize,
        segment_counts: &'a mut Vec<usize>,
        entity_index: usize,
        entity_parent: Option<usize>,
        entity_previous: Option<usize>,
        entity_indices: &'a mut Vec<Indices>,
    ) -> Self {
        Self {
            segment_index,
            segment_counts,
            entity_index,
            entity_parent,
            entity_previous,
            entity_indices,
        }
    }

    pub fn owned(&mut self) -> CountContext {
        self.with(
            self.segment_index,
            self.entity_index,
            self.entity_parent,
            self.entity_previous,
        )
    }

    pub fn with(
        &mut self,
        segment_index: usize,
        entity_index: usize,
        entity_parent: Option<usize>,
        entity_previous: Option<usize>,
    ) -> CountContext {
        CountContext::new(
            segment_index,
            self.segment_counts,
            entity_index,
            entity_parent,
            entity_previous,
            self.entity_indices,
        )
    }

    pub fn child<T>(&mut self, segment_index: usize, scope: impl FnOnce(CountContext) -> T) -> T {
        let entity_index = self.entity_indices.len();
        self.entity_indices.push(Indices {
            offset: self.segment_counts[segment_index],
            segment: segment_index,
            parent: self.entity_parent,
            next: None,
        });
        if let Some(previous) = self.entity_previous {
            self.entity_indices[previous].next = Some(entity_index);
        }
        self.entity_previous = Some(entity_index);
        self.segment_counts[segment_index] += 1;
        scope(self.with(segment_index, entity_index, Some(self.entity_index), None))
    }
}

impl<'a> ApplyContext<'a> {
    pub fn new(
        segment_index: usize,
        segment_indices: &'a Vec<usize>,
        store_indices: &'a Vec<usize>,
        entity_index: usize,
        entity_count: &'a mut usize,
        entity_indices: &'a [Indices],
        entity_instances: &'a [Entity],
    ) -> Self {
        Self {
            segment_index,
            segment_indices,
            store_indices,
            entity_index,
            entity_count,
            entity_indices,
            entity_instances,
        }
    }

    pub fn entity(&self) -> Entity {
        self.family().entity()
    }

    pub fn family(&self) -> Family {
        Family::new(
            self.entity_index,
            self.entity_indices,
            self.entity_instances,
            self.segment_indices,
        )
    }

    pub fn store_index(&self) -> usize {
        let indices = &self.entity_indices[self.entity_index];
        self.store_indices[self.segment_index] + indices.offset
    }

    pub fn owned(&mut self) -> ApplyContext {
        self.with(self.segment_index, self.entity_index)
    }

    pub fn with(&mut self, segment_index: usize, entity_index: usize) -> ApplyContext {
        ApplyContext::new(
            segment_index,
            self.segment_indices,
            self.store_indices,
            entity_index,
            self.entity_count,
            self.entity_indices,
            self.entity_instances,
        )
    }

    pub fn child<T>(&mut self, segment_index: usize, scope: impl FnOnce(ApplyContext) -> T) -> T {
        let entity_index = *self.entity_count;
        *self.entity_count += 1;
        scope(self.with(segment_index, entity_index))
    }
}

impl GetMeta {
    pub fn new<C: Component>() -> Self {
        Self(|world| world.get_or_add_meta::<C>())
    }

    pub fn get(&self, world: &mut World) -> Arc<Meta> {
        (self.0)(world)
    }
}

impl<C: Component> Initial for C {
    type Input = ();
    type Declare = Arc<Meta>;
    type State = Arc<Store>;

    fn declare(_: Self::Input, mut context: DeclareContext) -> Self::Declare {
        context.meta(GetMeta::new::<C>())
    }

    fn initialize(meta: Self::Declare, context: InitializeContext) -> Self::State {
        context.segment().store(&meta).unwrap()
    }

    fn static_count(_: &Self::State, _: CountContext) -> bool {
        true
    }

    fn dynamic_count(&self, _: &Self::State, _: CountContext) {}

    fn apply(self, store: &Self::State, context: ApplyContext) {
        unsafe { store.set(context.store_index(), self) }
    }
}

unsafe impl<C: Component> Static for C {}
unsafe impl<C: Component> Leaf for C {}

impl Initial for Box<dyn Any + Send> {
    type Input = GetMeta;
    type Declare = Arc<Meta>;
    type State = Arc<Store>;

    fn declare(input: Self::Input, mut context: DeclareContext) -> Self::Declare {
        context.meta(input)
    }

    fn initialize(meta: Self::Declare, context: InitializeContext) -> Self::State {
        context.segment().store(&meta).unwrap()
    }

    fn static_count(_: &Self::State, _: CountContext) -> bool {
        true
    }

    fn dynamic_count(&self, _: &Self::State, _: CountContext) {}

    fn apply(self, store: &Self::State, context: ApplyContext) {
        unsafe { store.set_any(context.store_index(), self) }
    }
}

unsafe impl Static for Box<dyn Any + Send> {}
unsafe impl Leaf for Box<dyn Any + Send> {}

impl<I: Initial> Initial for Vec<I> {
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

unsafe impl<I: Leaf> Static for Vec<I> {}
unsafe impl<I: Leaf> Leaf for Vec<I> {}

impl<I: Initial, const N: usize> Initial for [I; N] {
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

unsafe impl<I: Static, const N: usize> Static for [I; N] {}
unsafe impl<I: Leaf, const N: usize> Leaf for [I; N] {}

pub struct With<I: Initial, F: FnOnce(Family) -> I>(F, PhantomData<I>);

impl<I: Static, F: FnOnce(Family) -> I + Send + 'static> Initial for With<I, F> {
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

unsafe impl<I: Static, F: FnOnce(Family) -> I + Send + 'static> Static for With<I, F> {}
unsafe impl<I: Static + Leaf, F: FnOnce(Family) -> I + Send + 'static> Leaf for With<I, F> {}

pub fn with<I: Initial, F: FnOnce(Family) -> I>(with: F) -> With<I, F> {
    With(with, PhantomData)
}

pub struct Child<I: Initial>(I);

impl<I: Initial> Initial for Child<I> {
    type Input = I::Input;
    type Declare = (usize, I::Declare);
    type State = (usize, I::State);

    fn declare(input: Self::Input, mut context: DeclareContext) -> Self::Declare {
        context.child(|context| I::declare(input, context))
    }

    fn initialize((index, state): Self::Declare, mut context: InitializeContext) -> Self::State {
        context.child(index, |context| I::initialize(state, context))
    }

    fn static_count((index, state): &Self::State, mut context: CountContext) -> bool {
        context.child(*index, |context| I::static_count(state, context))
    }

    fn dynamic_count(&self, (index, state): &Self::State, mut context: CountContext) {
        context.child(*index, |context| self.0.dynamic_count(state, context))
    }

    fn apply(self, (index, state): &Self::State, mut context: ApplyContext) {
        context.child(*index, |context| self.0.apply(state, context))
    }
}

unsafe impl<I: Static> Static for Child<I> {}

pub fn child<I: Initial>(initial: I) -> Child<I> {
    Child(initial)
}

macro_rules! modify {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Initial,)*> Initial for ($($t,)*) {
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

        unsafe impl<$($t: Static,)*> Static for ($($t,)*) {}
        unsafe impl<$($t: Leaf,)*> Leaf for ($($t,)*) {}
    };
}

entia_macro::recurse_32!(modify);

/*
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

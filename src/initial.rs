use std::{any::Any, array::IntoIter, sync::Arc};

use crate::{
    component::Component,
    entity::Entity,
    segment::{Segment, Store},
    world::{Meta, World},
};

pub struct GetMeta(fn(&mut World) -> Arc<Meta>);

pub struct DeclareContext<'a> {
    index: usize,
    metas: &'a mut Vec<Vec<Arc<Meta>>>,
    world: &'a mut World,
}

pub struct InitializeContext<'a> {
    index: usize,
    segments: &'a Vec<usize>,
    world: &'a World,
}

pub struct CountContext<'a> {
    index: usize,
    count: &'a mut usize,
    counts: &'a mut Vec<usize>,
}

// TODO: can this context have access to the current entity and/or the whole hierarchy?
pub struct ApplyContext<'a> {
    index: usize,
    offset: usize,
    indices: &'a Vec<usize>,
    counts: &'a Vec<usize>,
    entities: &'a Vec<Entity>,
}

pub struct Child<I: Initial>(I);

pub trait Initial: Send + 'static {
    type Input;
    type Declare;
    type State;

    fn declare(input: Self::Input, context: DeclareContext) -> Self::Declare;
    fn initialize(state: Self::Declare, context: InitializeContext) -> Self::State;
    fn count(&self, state: &Self::State, context: CountContext);
    fn apply(self, state: &Self::State, context: ApplyContext);
}

impl<'a> DeclareContext<'a> {
    #[inline]
    pub fn new(index: usize, metas: &'a mut Vec<Vec<Arc<Meta>>>, world: &'a mut World) -> Self {
        Self {
            index,
            metas,
            world,
        }
    }

    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn owned(&mut self) -> DeclareContext {
        DeclareContext {
            index: self.index,
            metas: self.metas,
            world: self.world,
        }
    }

    #[inline]
    pub fn meta(&mut self, get: GetMeta) -> Arc<Meta> {
        let meta = get.get(self.world);
        self.metas[self.index].push(meta.clone());
        meta
    }

    #[inline]
    pub fn child<T>(&mut self, scope: impl FnOnce(DeclareContext) -> T) -> (usize, T) {
        let index = self.metas.len();
        let metas = vec![self.world.get_or_add_meta::<Entity>()];
        self.metas.push(metas);

        let mut context = self.owned();
        context.index = index;
        (index, scope(context))
    }
}

impl<'a> InitializeContext<'a> {
    #[inline]
    pub const fn new(index: usize, segments: &'a Vec<usize>, world: &'a World) -> Self {
        Self {
            index,
            segments,
            world,
        }
    }

    #[inline]
    pub fn segment(&self) -> &Segment {
        &self.world.segments[self.segments[self.index]]
    }

    #[inline]
    pub fn owned(&mut self) -> InitializeContext {
        InitializeContext {
            index: self.index,
            segments: self.segments,
            world: self.world,
        }
    }

    #[inline]
    pub fn child<T>(
        &mut self,
        index: usize,
        scope: impl FnOnce(InitializeContext) -> T,
    ) -> (usize, T) {
        // TODO: If there are duplicate segments in 'self.segments', give out the index that represent the earliest apperance
        // of 'self.segments[index]'.
        let mut context = self.owned();
        context.index = index;
        (index, scope(context))
    }
}

impl<'a> CountContext<'a> {
    #[inline]
    pub fn new(index: usize, count: &'a mut usize, counts: &'a mut Vec<usize>) -> Self {
        Self {
            index,
            count,
            counts,
        }
    }

    #[inline]
    pub fn owned(&mut self) -> CountContext {
        CountContext {
            index: self.index,
            count: self.count,
            counts: self.counts,
        }
    }

    #[inline]
    pub fn child<T>(&mut self, index: usize, scope: impl FnOnce(CountContext) -> T) -> T {
        *self.count += 1;
        self.counts[index] += 1;

        let mut context = self.owned();
        context.index = index;
        scope(context)
    }
}

impl<'a> ApplyContext<'a> {
    #[inline]
    pub const fn new(
        index: usize,
        offset: usize, // TODO: this is probably wrong...
        indices: &'a Vec<usize>,
        counts: &'a Vec<usize>,
        entities: &'a Vec<Entity>,
    ) -> Self {
        Self {
            index,
            offset,
            indices,
            counts,
            entities,
        }
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.indices[self.index]
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.counts[self.index]
    }

    #[inline]
    pub fn owned(&mut self) -> ApplyContext {
        ApplyContext {
            index: self.index,
            offset: self.offset,
            indices: self.indices,
            counts: self.counts,
            entities: self.entities,
        }
    }

    #[inline]
    pub fn child<T>(&mut self, index: usize, scope: impl FnOnce(ApplyContext) -> T) -> T {
        let mut context = self.owned();
        context.index = index;
        scope(context)
    }
}

impl GetMeta {
    #[inline]
    pub fn new<C: Component>() -> Self {
        Self(|world| world.get_or_add_meta::<C>())
    }

    #[inline]
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

    fn count(&self, _: &Self::State, _: CountContext) {}

    fn apply(self, store: &Self::State, context: ApplyContext) {
        unsafe { store.set(context.index(), self) }
    }
}

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

    fn count(&self, _: &Self::State, _: CountContext) {}

    fn apply(self, store: &Self::State, context: ApplyContext) {
        unsafe { store.set_any(context.index(), self) }
    }
}

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

    fn count(&self, state: &Self::State, mut context: CountContext) {
        for value in self {
            value.count(state, context.owned());
        }
    }

    fn apply(self, state: &Self::State, mut context: ApplyContext) {
        for value in self {
            value.apply(state, context.owned());
        }
    }
}

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

    fn count(&self, state: &Self::State, mut context: CountContext) {
        for value in self {
            value.count(state, context.owned());
        }
    }

    fn apply(self, state: &Self::State, mut context: ApplyContext) {
        for value in IntoIter::new(self) {
            value.apply(state, context.owned());
        }
    }
}

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

    fn count(&self, (index, state): &Self::State, mut context: CountContext) {
        context.child(*index, |context| self.0.count(state, context))
    }

    fn apply(self, (index, state): &Self::State, mut context: ApplyContext) {
        context.child(*index, |context| self.0.apply(state, context))
    }
}

#[inline]
pub fn child<I: Initial>(initial: I) -> Child<I> {
    Child(initial)
}

macro_rules! modify {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Initial,)*> Initial for ($($t,)*) {
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
            fn count(&self, ($($t,)*): &Self::State, mut _context: CountContext) {
                let ($($p,)*) = self;
                $($p.count($t, _context.owned());)*
            }

            #[inline]
            fn apply(self, ($($t,)*): &Self::State, mut _context: ApplyContext) {
                let ($($p,)*) = self;
                $($p.apply($t, _context.owned());)*
            }
        }
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

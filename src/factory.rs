use std::{
    any::TypeId, array::IntoIter, cmp::min, convert::TryInto, iter::once, mem::replace, sync::Arc,
};

use entia_core::{Append, Chop};

use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    inject::{Context, Get, Inject},
    prelude::Component,
    segment::{Segment, Store},
    world::{Meta, World},
    write::{self, Write},
};

type Initialize = Arc<dyn Fn(&mut World) -> Arc<Meta>>;
type Apply<I> = Arc<dyn Fn(&I, &Store, usize, usize)>;

#[derive(Clone)]
pub struct Template<I: 'static = ()> {
    initializers: Vec<(TypeId, Initialize, Apply<I>)>,
    children: Vec<Template<I>>,
}

pub struct Factory<'a, I: 'static = ()> {
    defer: &'a mut Vec<Defer<I>>,
    segment: &'a Segment,
    applies: &'a Vec<(Arc<Store>, Apply<I>)>,
    entities: &'a mut Entities,
    buffer: &'a mut Vec<Entity>,
}

pub struct State<I> {
    defer: Vec<Defer<I>>,
    segment: usize,
    applies: Vec<(Arc<Store>, Apply<I>)>,
    entities: write::State<Entities>,
    buffer: Vec<Entity>,
}

#[derive(Clone)]
pub struct Input<'a, I: Clone> {
    value: I,
    entity: Entity,
    parent: Option<Entity>,
    children: &'a [Entity],
}

struct Defer<I> {
    entities: Vec<Entity>,
    inputs: Vec<I>,
    index: usize,
    ready: usize,
}

impl<I: Clone + 'static> Factory<'_, I> {
    pub fn all(&mut self, mut inputs: impl ExactSizeIterator<Item = I>) -> &[Entity] {
        let count = inputs.len();
        if count == 0 {
            return &[];
        }

        self.buffer.resize(count, Entity::ZERO);
        let valid = self.entities.reserve(self.buffer);
        let pair = self.segment.reserve(count);
        let ready = min(valid, pair.1);

        if ready > 0 {
            unsafe { self.segment.stores[0].set_all(pair.0, &self.buffer[..ready]) };
            for i in 0..ready {
                let input = inputs.next().unwrap();
                let entity = self.buffer[i];
                let index = pair.0 + i;
                let datum = self.entities.get_datum_mut_unchecked(entity);
                for (store, apply) in self.applies {
                    apply(&input, store.as_ref(), index, 1);
                }
                datum.initialize(entity.generation, index as u32, self.segment.index as u32);
            }
        }

        if ready < count {
            self.defer.push(Defer {
                entities: replace(self.buffer, Vec::new()),
                inputs: inputs.collect(),
                index: pair.0,
                ready,
            });
            &self.defer.last().unwrap().entities
        } else {
            &self.buffer
        }
    }

    #[inline]
    pub fn one(&mut self, initial: I) -> Entity {
        self.all(IntoIter::new([initial]))[0]
    }

    #[inline]
    pub fn exact<const N: usize>(&mut self, initials: [I; N]) -> &[Entity; N] {
        self.all(IntoIter::new(initials)).try_into().unwrap()
    }

    #[inline]
    pub fn clones(&mut self, input: I, count: usize) -> &[Entity]
    where
        I: Clone,
    {
        self.all((0..count).map(move |_| input.clone()))
    }

    #[inline]
    pub fn defaults(&mut self, count: usize) -> &[Entity]
    where
        I: Default,
    {
        self.all((0..count).map(|_| I::default()))
    }
}

impl<I: Clone + 'static> Template<I> {
    pub fn new() -> Self {
        Self {
            initializers: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn keys(&self) -> impl ExactSizeIterator<Item = &TypeId> {
        self.initializers.iter().map(|(key, _, _)| key)
    }

    pub fn family(&self) -> impl Iterator<Item = &Self> {
        once(self).chain(self.descendants())
    }

    pub fn children(&self) -> impl ExactSizeIterator<Item = &Self> {
        self.children.iter()
    }

    pub fn descendants(&self) -> impl Iterator<Item = &Self> {
        // TODO: this is incorrect
        self.children().flat_map(|child| child.children())
    }

    pub fn adapt<T: Clone + 'static>(self, adapt: impl Fn(T) -> I + 'static) -> Template<T> {
        fn descend<S, T: Clone>(
            template: Template<S>,
            adapt: Arc<impl Fn(T) -> S + 'static>,
        ) -> Template<T> {
            let mut initializers = Vec::with_capacity(template.initializers.len());
            let mut children = Vec::with_capacity(template.children.len());
            for (key, initialize, apply) in template.initializers {
                let adapt = adapt.clone();
                let apply: Apply<T> = Arc::new(move |input, store, index, count| {
                    let adapted = adapt(input.clone());
                    apply(&adapted, store, index, count);
                });
                initializers.push((key, initialize, apply));
            }

            for child in template.children {
                children.push(descend(child, adapt.clone()));
            }
            Template {
                initializers,
                children,
            }
        }
        descend(self, Arc::new(adapt))
    }

    pub fn add<C: Component>(self) -> Template<<I as Append<C>>::Target>
    where
        I: Append<C>,
        <I as Append<C>>::Target: Clone,
    {
        self.adapt(|input: <I as Append<C>>::Target| input.chop().1)
            .add_with(|input| input.chop().0)
    }

    pub fn add_clone<C: Component + Clone>(self, component: C) -> Self {
        self.add_with(move |_| component.clone())
    }

    pub fn add_default<C: Component + Default>(self) -> Self {
        self.add_with(|_| C::default())
    }

    pub fn add_with<C: Component>(self, provide: impl Fn(I) -> C + 'static) -> Self {
        self.add_key(
            TypeId::of::<C>(),
            Arc::new(|world| world.get_or_add_meta::<C>()),
            Arc::new(move |input, store, index, count| {
                for i in 0..count {
                    unsafe { store.set(index + i, provide(input.clone())) };
                }
            }),
        )
    }

    pub fn add_template(mut self, mut template: Self) -> Self {
        for (key, initialize, apply) in template.initializers {
            self = self.add_key(key, initialize, apply);
        }
        self.children.append(&mut template.children);
        self
    }

    pub fn remove<C: Component>(self) -> Self {
        self.remove_key(TypeId::of::<C>())
    }

    pub fn remove_key(mut self, key: TypeId) -> Self {
        self.initializers.retain(|pair| pair.0 != key);
        self
    }

    pub fn adopt(mut self, template: Self) -> Self {
        self.children.push(template);
        self
    }

    fn add_key(mut self, key: TypeId, initialize: Initialize, apply: Apply<I>) -> Self {
        self = self.remove_key(key);
        self.initializers.push((key, initialize, apply));
        self
    }
}

impl<I: Clone + 'static> Inject for Factory<'_, I> {
    type Input = Template<I>;
    type State = State<I>;

    fn initialize(input: Self::Input, context: &Context, world: &mut World) -> Option<Self::State> {
        let mut pairs = Vec::new();
        for (_, initialize, apply) in input.initializers {
            pairs.push((initialize(world), apply));
        }
        pairs.sort_by_key(|pair| pair.0.index);

        let metas: Vec<_> = pairs.iter().map(|(meta, _)| meta.clone()).collect();
        let segment = world.get_or_add_segment_by_metas(&metas);
        let applies = pairs
            .drain(..)
            .filter_map(|(meta, apply)| Some((segment.store(&meta)?, apply)))
            .collect();
        let segment = segment.index;
        let entities = <Write<Entities> as Inject>::initialize(None, context, world)?;
        Some(State {
            defer: Vec::new(),
            buffer: Vec::new(),
            applies,
            segment,
            entities,
        })
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        let entities = state.entities.as_mut();
        let segment = &mut world.segments[state.segment];
        entities.resolve();
        segment.resolve();

        for mut defer in state.defer.drain(..) {
            if defer.entities.len() == 0 || defer.inputs.len() == 0 {
                continue;
            }

            let index = defer.index + defer.ready;
            unsafe { segment.stores[0].set_all(index, &defer.entities[defer.ready..]) };
            for (i, input) in defer.inputs.drain(..).enumerate() {
                let index = index + i;
                let entity = defer.entities[defer.ready + i];
                let datum = entities.get_datum_mut_unchecked(entity);
                for (store, apply) in state.applies.iter() {
                    apply(&input, store.as_ref(), index, 1);
                }
                datum.initialize(entity.generation, index as u32, state.segment as u32);
            }
        }
    }
}

impl<'a, I: Clone + 'static> Get<'a> for State<I> {
    type Item = Factory<'a, I>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Factory {
            defer: &mut self.defer,
            segment: &world.segments[self.segment],
            applies: &self.applies,
            entities: self.entities.get(world),
            buffer: &mut self.buffer,
        }
    }
}

unsafe impl<I> Depend for State<I> {
    fn depend(&self, _: &World) -> Vec<crate::depend::Dependency> {
        vec![Dependency::Defer(self.segment, TypeId::of::<Entity>())]
    }
}

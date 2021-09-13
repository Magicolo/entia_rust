use std::{any::TypeId, array::IntoIter, convert::TryInto, iter::from_fn, mem::replace};

use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    initial::{
        child, ApplyContext, Child, CountContext, DeclareContext, Initial, InitializeContext,
    },
    inject::{Context, Get, Inject},
    segment::Segment,
    world::World,
    write::{self, Write},
};

pub struct Create<'a, I: Initial> {
    defer: &'a mut Vec<Defer<I>>,
    entities: &'a mut Entities,
    world: &'a World,
    segments: &'a Vec<usize>,
    segment_counts: &'a mut Vec<usize>,
    segment_indices: &'a mut Vec<usize>,
    entity_parents: &'a mut Vec<usize>,
    entity_instances: &'a mut Vec<Entity>,
    initial_state: &'a <Child<I> as Initial>::State,
    initial_roots: &'a mut Vec<Child<I>>,
}

pub struct State<I: Initial> {
    defer: Vec<Defer<I>>,
    entities: write::State<Entities>,
    segment_indices: Vec<usize>,
    segment_counts: Vec<usize>,
    segments: Vec<usize>,
    entity_parents: Vec<usize>,
    entity_instances: Vec<Entity>,
    initial_state: <Child<I> as Initial>::State,
    initial_roots: Vec<Child<I>>,
}

#[derive(Clone)]
pub struct Family<'a> {
    index: usize,
    parents: &'a Vec<usize>,
    entities: &'a Vec<Entity>,
}

impl<'a> Family<'a> {
    #[inline]
    pub fn entity(&self) -> Entity {
        self.entities[self.index]
    }

    pub fn parent(&self) -> Option<Self> {
        let parent = self.parents[self.index];
        if parent == self.index {
            None
        } else {
            Some(self.with(parent))
        }
    }

    pub fn children(&self) -> impl Iterator<Item = Family> {
        let mut last = self.index;
        from_fn(move || {
            last += 1;
            for (i, &parent) in self.parents[last..].iter().enumerate() {
                if parent == self.index {
                    last += i;
                    return Some(self.with(last));
                }
            }
            None
        })
    }

    pub fn root(&self) -> Self {
        // Do not assume that index '0' is the root since there might be multiple roots.
        match self.parent() {
            Some(parent) => parent.root(),
            None => self.clone(),
        }
    }

    fn with(&self, index: usize) -> Self {
        Self {
            index,
            parents: self.parents,
            entities: self.entities,
        }
    }
}

struct Defer<I: Initial> {
    index: usize,
    entities: Vec<Entity>,
    indices: Vec<usize>,
    counts: Vec<usize>,
    roots: Vec<Child<I>>,
}

impl<I: Initial> Create<'_, I> {
    // Returns the roots of the hierarchies.
    pub fn all(&mut self, initials: impl Iterator<Item = I>) -> &[Entity] {
        // TODO: Maybe there is a better way to reset those?
        for count in self.segment_counts.iter_mut() {
            *count = 0;
        }
        for index in self.segment_indices.iter_mut() {
            *index = 0;
        }

        let mut total = 0;
        for initial in initials {
            let root = child(initial);
            let mut count = 0;
            root.count(
                self.initial_state,
                CountContext::new(0, &mut count, self.segment_counts),
            );
            total += count;
            self.initial_roots.push(root);
        }

        if total == 0 {
            self.initial_roots.clear();
            return &[];
        }

        self.entity_instances.resize(total, Entity::ZERO);
        let valid = self.entities.reserve(self.entity_instances);
        let mut head = 0;

        for (i, count) in self.segment_counts.iter_mut().enumerate() {
            if *count == 0 {
                continue;
            }

            let segment = &self.world.segments[self.segments[i]];
            let pair = segment.reserve(*count);
            self.segment_indices[i] = pair.0;

            if pair.1 == *count {
                let tail = head + *count;
                set_and_initialize_entities(
                    pair.0,
                    &self.entity_instances[head..tail],
                    segment,
                    self.entities,
                );
                head = tail;
            }
        }

        let roots = self.initial_roots.len();
        if head == valid && valid == total {
            for (i, root) in self.initial_roots.drain(..).enumerate() {
                root.apply(
                    self.initial_state,
                    ApplyContext::new(
                        0,
                        i,
                        self.segment_indices,
                        self.segment_counts,
                        &self.entity_instances,
                    ),
                );
            }
            &self.entity_instances[..roots]
        } else {
            self.defer.push(Defer {
                index: head,
                entities: replace(self.entity_instances, Vec::new()),
                indices: replace(self.segment_indices, vec![0; self.segment_indices.len()]),
                counts: replace(self.segment_counts, vec![0; self.segment_counts.len()]),
                roots: replace(self.initial_roots, Vec::new()),
            });
            &self.defer.last().unwrap().entities[..roots]
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
    pub fn clones(&mut self, initial: I, count: usize) -> &[Entity]
    where
        I: Clone,
    {
        self.all((0..count).map(move |_| initial.clone()))
    }

    #[inline]
    pub fn defaults(&mut self, count: usize) -> &[Entity]
    where
        I: Default,
    {
        self.all((0..count).map(|_| I::default()))
    }
}

impl<I: Initial> Inject for Create<'_, I> {
    type Input = I::Input;
    type State = State<I>;

    fn initialize(input: Self::Input, context: &Context, world: &mut World) -> Option<Self::State> {
        let entities = <Write<Entities> as Inject>::initialize(None, context, world)?;
        let mut metas = Vec::new();
        let declare = Child::<I>::declare(input, DeclareContext::new(0, &mut metas, world));
        let segments = metas
            .drain(..)
            .map(|metas| world.get_or_add_segment_by_metas(&metas).index)
            .collect();
        let state = Child::<I>::initialize(declare, InitializeContext::new(0, &segments, world));

        Some(State {
            defer: Vec::new(),
            entities,
            initial_state: state,
            initial_roots: Vec::new(),
            entity_parents: Vec::new(),
            entity_instances: Vec::new(),
            segment_indices: vec![0; segments.len()],
            segment_counts: vec![0; segments.len()],
            segments,
        })
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        let entities = state.entities.as_mut();
        entities.resolve();

        for &segment in state.segments.iter() {
            world.segments[segment].resolve();
        }

        for mut defer in state.defer.drain(..) {
            if defer.entities.len() == 0 || defer.roots.len() == 0 {
                continue;
            }

            let mut head = defer.index;
            for (i, count) in defer.counts.iter().enumerate() {
                if *count == 0 {
                    continue;
                }

                let tail = head + count;
                set_and_initialize_entities(
                    defer.indices[i],
                    &defer.entities[head..tail],
                    &world.segments[state.segments[i]],
                    entities,
                );
                head = tail;
            }

            for (i, root) in defer.roots.drain(..).enumerate() {
                root.apply(
                    &state.initial_state,
                    ApplyContext::new(0, i, &defer.indices, &defer.counts, &defer.entities),
                );
            }
        }
    }
}

fn set_and_initialize_entities(
    index: usize,
    buffer: &[Entity],
    segment: &Segment,
    entities: &mut Entities,
) {
    // The first store of a segment with entities is always the entity store.
    unsafe { segment.stores[0].set_all(index, buffer) };
    for i in 0..buffer.len() {
        let entity = buffer[i];
        let index = index + i;
        let datum = entities.get_datum_mut_unchecked(entity);
        datum.initialize(entity.generation, index as u32, segment.index as u32);
    }
}

impl<'a, I: Initial> Get<'a> for State<I> {
    type Item = Create<'a, I>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Create {
            defer: &mut self.defer,
            entities: self.entities.get(world),
            world,
            initial_state: &self.initial_state,
            initial_roots: &mut self.initial_roots,
            entity_parents: &mut self.entity_parents,
            entity_instances: &mut self.entity_instances,
            segment_indices: &mut self.segment_indices,
            segment_counts: &mut self.segment_counts,
            segments: &self.segments,
        }
    }
}

unsafe impl<I: Initial> Depend for State<I> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        self.segments
            .iter()
            .map(|&segment| Dependency::Defer(segment, TypeId::of::<Entity>()))
            .collect()
    }
}

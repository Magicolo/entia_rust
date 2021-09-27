use std::{any::TypeId, array::IntoIter, collections::HashMap};

use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    family::initial::{EntityIndices, Families, Family, SegmentIndices},
    initial::{
        spawn, ApplyContext, CountContext, DeclareContext, Initial, InitializeContext, Spawn,
    },
    inject::{Get, Inject, InjectContext},
    segment::Segment,
    world::World,
    write::{self, Write},
};

pub struct Create<'a, I: Initial> {
    count: Option<usize>,
    defer: &'a mut Vec<Defer<I>>,
    entities: &'a mut Entities,
    world: &'a World,
    segment_indices: &'a mut Vec<SegmentIndices>,
    entity_indices: &'a mut Vec<EntityIndices>,
    entity_instances: &'a mut Vec<Entity>,
    entity_roots: &'a mut Vec<(usize, usize)>,
    initial_state: &'a <Spawn<I> as Initial>::State,
    initial_roots: &'a mut Vec<Spawn<I>>,
}

pub struct State<I: Initial> {
    count: Option<usize>,
    defer: Vec<Defer<I>>,
    entities: write::State<Entities>,
    segment_indices: Vec<SegmentIndices>,
    entity_indices: Vec<EntityIndices>,
    entity_instances: Vec<Entity>,
    entity_roots: Vec<(usize, usize)>,
    initial_state: <Spawn<I> as Initial>::State,
    initial_roots: Vec<Spawn<I>>,
}

struct Defer<I: Initial> {
    index: usize,
    initial_roots: Vec<Spawn<I>>,
    entity_roots: Vec<(usize, usize)>,
    entity_instances: Vec<Entity>,
    entity_indices: Vec<EntityIndices>,
    segment_indices: Vec<SegmentIndices>,
}

impl<I: Initial> Create<'_, I> {
    pub fn all(&mut self, initials: impl Iterator<Item = I>) -> Families {
        match self.count {
            Some(count) => self.all_static(count, initials),
            None => self.all_dynamic(initials),
        }
    }

    #[inline]
    pub fn one(&mut self, initial: I) -> Family {
        self.all(IntoIter::new([initial]))
            .into_iter()
            .next()
            .unwrap()
    }

    #[inline]
    pub fn clones(&mut self, initial: I, count: usize) -> Families
    where
        I: Clone,
    {
        self.all((0..count).map(move |_| initial.clone()))
    }

    #[inline]
    pub fn defaults(&mut self, count: usize) -> Families
    where
        I: Default,
    {
        self.all((0..count).map(|_| I::default()))
    }

    fn all_static(&mut self, count: usize, initials: impl Iterator<Item = I>) -> Families {
        self.initial_roots.clear();
        self.entity_roots.clear();

        for initial in initials {
            self.initial_roots.push(spawn(initial));
            self.entity_roots.push((self.entity_roots.len(), 0));
        }

        if self.initial_roots.len() == 0 {
            return Families::EMPTY;
        }

        let (index, success) = self.reserve(count * self.initial_roots.len());
        if success {
            apply(
                self.initial_state,
                self.initial_roots.drain(..),
                self.entity_roots,
                self.entity_instances,
                self.entity_indices,
                self.segment_indices,
            );
        } else {
            self.defer(index);
        }

        self.families()
    }

    fn all_dynamic(&mut self, initials: impl Iterator<Item = I>) -> Families {
        self.entity_roots.clear();
        self.entity_indices.clear();
        self.segment_indices
            .iter_mut()
            .for_each(|indices| indices.count = 0);

        for initial in initials {
            self.entity_roots.push((0, self.entity_indices.len()));
            let root = spawn(initial);
            root.dynamic_count(
                self.initial_state,
                CountContext::new(
                    0,
                    self.segment_indices,
                    self.entity_indices.len(),
                    None,
                    &mut None,
                    self.entity_indices,
                ),
            );
            self.initial_roots.push(root);
        }

        let count = self.entity_indices.len();
        if count == 0 {
            return Families::EMPTY;
        }

        let (index, success) = self.reserve(count);
        if success {
            apply(
                self.initial_state,
                self.initial_roots.drain(..),
                self.entity_roots,
                self.entity_instances,
                self.entity_indices,
                self.segment_indices,
            );
        } else {
            self.defer(index);
        }

        self.families()
    }

    fn reserve(&mut self, count: usize) -> (usize, bool) {
        self.entity_instances.resize(count, Entity::ZERO);
        let ready = self.entities.reserve(self.entity_instances);
        let mut index = 0;
        let mut head = 0;
        let mut success = true;
        let multiplier = count / self.entity_indices.len();

        for (i, segment_indices) in self.segment_indices.iter_mut().enumerate() {
            segment_indices.index = head;
            let segment_count = segment_indices.count * multiplier;
            if segment_count == 0 {
                continue;
            }

            let segment = &self.world.segments[segment_indices.segment];
            let pair = segment.reserve(segment_count);
            segment_indices.store = pair.0;

            let tail = head + segment_count;
            if success && tail <= ready && pair.1 == segment_count {
                index = i;
                initialize(
                    pair.0,
                    &self.entity_instances[head..tail],
                    segment,
                    self.entities,
                );
            } else {
                success = false;
            }
            head = tail;
        }

        (index, success && ready == count)
    }

    fn families(&self) -> Families {
        Families::new(
            self.entity_roots,
            self.entity_instances,
            self.entity_indices,
            self.segment_indices,
        )
    }

    fn defer(&mut self, index: usize) {
        self.defer.push(Defer {
            index,
            initial_roots: self.initial_roots.drain(..).collect(),
            entity_roots: self.entity_roots.clone(),
            entity_instances: self.entity_instances.clone(),
            entity_indices: self.entity_indices.clone(),
            segment_indices: self.segment_indices.clone(),
        });
    }
}

unsafe impl<I: Initial> Inject for Create<'_, I> {
    type Input = I::Input;
    type State = State<I>;

    fn initialize(input: Self::Input, mut context: InjectContext) -> Option<Self::State> {
        let entities = <Write<Entities> as Inject>::initialize(None, context.owned())?;
        let world = context.world();
        let mut segment_metas = Vec::new();
        let declare = Spawn::<I>::declare(input, DeclareContext::new(0, &mut segment_metas, world));
        let mut segment_to_index = HashMap::new();
        let mut metas_to_segment = HashMap::new();
        let mut segment_indices = Vec::with_capacity(segment_metas.len());
        for (i, metas) in segment_metas.into_iter().enumerate() {
            let segment = world.get_or_add_segment_by_metas(&metas).index;
            let index = match segment_to_index.get(&segment) {
                Some(&index) => index,
                None => {
                    segment_to_index.insert(segment, segment_indices.len());
                    segment_indices.push(SegmentIndices {
                        segment,
                        count: 0,
                        index: 0,
                        store: 0,
                    });
                    i
                }
            };
            metas_to_segment.insert(i, index);
        }

        let state = Spawn::<I>::initialize(
            declare,
            InitializeContext::new(0, &segment_indices, &metas_to_segment, world),
        );

        let mut entity_indices = Vec::new();
        let count = if Spawn::<I>::static_count(
            &state,
            CountContext::new(
                0,
                &mut segment_indices,
                entity_indices.len(),
                None,
                &mut None,
                &mut entity_indices,
            ),
        ) {
            Some(entity_indices.len())
        } else {
            None
        };

        Some(State {
            count,
            defer: Vec::new(),
            entities,
            initial_state: state,
            initial_roots: Vec::new(),
            entity_indices,
            entity_instances: Vec::new(),
            entity_roots: Vec::new(),
            segment_indices,
        })
    }

    fn resolve(state: &mut Self::State, mut context: InjectContext) {
        let world = context.world();
        let entities = state.entities.as_mut();
        entities.resolve();

        // Must resolve unconditionally all segments *even* if nothing was deferred.
        for segment_indices in state.segment_indices.iter() {
            world.segments[segment_indices.segment].resolve();
        }

        for defer in state.defer.drain(..) {
            let multiplier = defer.entity_instances.len() / defer.entity_indices.len();
            for segment_indices in defer.segment_indices[defer.index..].iter() {
                let count = segment_indices.count * multiplier;
                if count == 0 {
                    continue;
                }

                let head = segment_indices.index;
                let tail = head + count;
                initialize(
                    segment_indices.store,
                    &defer.entity_instances[head..tail],
                    &world.segments[segment_indices.segment],
                    entities,
                );
            }

            apply(
                &state.initial_state,
                defer.initial_roots.into_iter(),
                &defer.entity_roots,
                &defer.entity_instances,
                &defer.entity_indices,
                &defer.segment_indices,
            );
        }
    }
}

fn apply<I: Initial>(
    initial_state: &<Spawn<I> as Initial>::State,
    initial_roots: impl Iterator<Item = Spawn<I>>,
    entity_roots: &[(usize, usize)],
    entity_instances: &[Entity],
    entity_indices: &[EntityIndices],
    segment_indices: &[SegmentIndices],
) {
    for (i, root) in initial_roots.enumerate() {
        let (entity_root, mut entity_count) = entity_roots[i];
        root.apply(
            initial_state,
            ApplyContext::new(
                entity_root,
                0,
                None,
                &mut entity_count,
                entity_instances,
                entity_indices,
                segment_indices,
            ),
        );
    }
}

fn initialize(index: usize, instances: &[Entity], segment: &Segment, entities: &mut Entities) {
    for i in 0..instances.len() {
        let entity = instances[i];
        let index = index + i;
        let datum = entities.get_datum_mut_unchecked(entity);
        datum.initialize(entity.generation, index as u32, segment.index as u32);
    }
}

impl<'a, I: Initial> Get<'a> for State<I> {
    type Item = Create<'a, I>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Create {
            count: self.count,
            defer: &mut self.defer,
            entities: self.entities.get(world),
            world,
            initial_state: &self.initial_state,
            initial_roots: &mut self.initial_roots,
            entity_indices: &mut self.entity_indices,
            entity_instances: &mut self.entity_instances,
            entity_roots: &mut self.entity_roots,
            segment_indices: &mut self.segment_indices,
        }
    }
}

unsafe impl<I: Initial> Depend for State<I> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        self.segment_indices
            .iter()
            .map(|indices| Dependency::Defer(indices.segment, TypeId::of::<Entity>()))
            .collect()
    }
}

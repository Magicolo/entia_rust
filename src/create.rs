use std::{any::TypeId, collections::HashMap, iter::once};

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
    inner: &'a mut Inner<I>,
    entities: &'a mut Entities,
    world: &'a World,
}

pub struct State<I: Initial> {
    inner: Inner<I>,
    entities: write::State<Entities>,
}

struct Inner<I: Initial> {
    count: Option<usize>,
    defer: Vec<Defer<I>>,
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
        match self.inner.count {
            Some(count) => self
                .inner
                .all_static(count, initials, self.entities, self.world),
            None => self.inner.all_dynamic(initials, self.entities, self.world),
        }
    }

    #[inline]
    pub fn one(&mut self, initial: I) -> Family {
        self.all(once(initial)).into_iter().next().unwrap()
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
}

impl<I: Initial> Inner<I> {
    fn all_static(
        &mut self,
        count: usize,
        initials: impl Iterator<Item = I>,
        entities: &mut Entities,
        world: &World,
    ) -> Families {
        self.initial_roots.clear();
        self.entity_roots.clear();

        for initial in initials {
            self.initial_roots.push(spawn(initial));
            self.entity_roots.push((self.entity_roots.len(), 0));
        }

        if self.initial_roots.len() == 0 {
            return Families::EMPTY;
        }

        let (index, success) = self.reserve(count * self.initial_roots.len(), entities, world);
        if success {
            apply(
                &self.initial_state,
                self.initial_roots.drain(..),
                &self.entity_roots,
                &self.entity_instances,
                &self.entity_indices,
                entities,
                &self.segment_indices,
            );
        } else {
            self.defer(index);
        }

        self.families()
    }

    fn all_dynamic(
        &mut self,
        initials: impl Iterator<Item = I>,
        entities: &mut Entities,
        world: &World,
    ) -> Families {
        self.entity_roots.clear();
        self.entity_indices.clear();
        self.segment_indices
            .iter_mut()
            .for_each(|indices| indices.count = 0);

        for initial in initials {
            self.entity_roots.push((0, self.entity_indices.len()));
            let root = spawn(initial);
            root.dynamic_count(
                &self.initial_state,
                CountContext::new(
                    0,
                    &mut self.segment_indices,
                    self.entity_indices.len(),
                    None,
                    &mut None,
                    &mut self.entity_indices,
                ),
            );
            self.initial_roots.push(root);
        }

        let count = self.entity_indices.len();
        if count == 0 {
            return Families::EMPTY;
        }

        let (index, success) = self.reserve(count, entities, world);
        if success {
            apply(
                &self.initial_state,
                self.initial_roots.drain(..),
                &self.entity_roots,
                &self.entity_instances,
                &self.entity_indices,
                entities,
                &self.segment_indices,
            );
        } else {
            self.defer(index);
        }

        self.families()
    }

    fn reserve(&mut self, count: usize, entities: &mut Entities, world: &World) -> (usize, bool) {
        self.entity_instances.resize(count, Entity::ZERO);
        let ready = entities.reserve(&mut self.entity_instances);
        let mut last = 0;
        let mut segment_index = 0;
        let mut success = true;
        let multiplier = self.multiplier();

        for (i, segment_indices) in self.segment_indices.iter_mut().enumerate() {
            segment_indices.index = segment_index;
            let segment_count = segment_indices.count * multiplier;
            if segment_count == 0 {
                continue;
            }

            let segment = &world.segments[segment_indices.segment];
            let pair = segment.reserve(segment_count);
            segment_indices.store = pair.0;

            if success && segment_index <= ready && pair.1 == segment_count {
                last = i;
                initialize(
                    &self.entity_instances,
                    segment,
                    segment_index,
                    segment_count,
                    pair.0,
                );
            } else {
                success = false;
            }
            segment_index += segment_count;
        }

        (last, success && ready == count)
    }

    #[inline]
    fn multiplier(&self) -> usize {
        self.entity_instances.len() / self.entity_indices.len()
    }

    fn families(&self) -> Families {
        Families::new(
            &self.entity_roots,
            &self.entity_instances,
            &self.entity_indices,
            &self.segment_indices,
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
            inner: Inner {
                count,
                defer: Vec::new(),
                initial_state: state,
                initial_roots: Vec::new(),
                entity_indices,
                entity_instances: Vec::new(),
                entity_roots: Vec::new(),
                segment_indices,
            },
            entities,
        })
    }

    fn resolve(state: &mut Self::State, mut context: InjectContext) {
        let inner = &mut state.inner;
        let world = context.world();
        let entities = state.entities.as_mut();
        entities.resolve();

        // Must resolve unconditionally all segments *even* if nothing was deferred.
        for segment_indices in inner.segment_indices.iter() {
            world.segments[segment_indices.segment].resolve();
        }

        for defer in inner.defer.drain(..) {
            let multiplier = defer.entity_instances.len() / defer.entity_indices.len();
            for segment_indices in defer.segment_indices[defer.index..].iter() {
                let count = segment_indices.count * multiplier;
                if count == 0 {
                    continue;
                }

                initialize(
                    &defer.entity_instances,
                    &world.segments[segment_indices.segment],
                    segment_indices.index,
                    segment_indices.count * multiplier,
                    segment_indices.store,
                );
            }

            apply(
                &inner.initial_state,
                defer.initial_roots.into_iter(),
                &defer.entity_roots,
                &defer.entity_instances,
                &defer.entity_indices,
                entities,
                &defer.segment_indices,
            );
        }
    }
}

fn initialize(
    entity_instances: &[Entity],
    segment: &Segment,
    segment_index: usize,
    segment_count: usize,
    segment_store: usize,
) {
    let instances = &entity_instances[segment_index..segment_index + segment_count];
    unsafe { segment.stores[0].set_all(segment_store, instances) };
}

fn apply<I: Initial>(
    initial_state: &<Spawn<I> as Initial>::State,
    initial_roots: impl Iterator<Item = Spawn<I>>,
    entity_roots: &[(usize, usize)],
    entity_instances: &[Entity],
    entity_indices: &[EntityIndices],
    entities: &mut Entities,
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
                entities,
                0,
                segment_indices,
            ),
        );
    }
}

impl<'a, I: Initial> Get<'a> for State<I> {
    type Item = Create<'a, I>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Create {
            inner: &mut self.inner,
            entities: self.entities.get(world),
            world,
        }
    }
}

unsafe impl<I: Initial> Depend for State<I> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        self.inner
            .segment_indices
            .iter()
            .map(|indices| Dependency::Defer(indices.segment, TypeId::of::<Entity>()))
            .collect()
    }
}

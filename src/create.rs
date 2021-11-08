use std::{collections::HashMap, iter::once};

use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    family::template::{EntityIndices, Families, Family, SegmentIndices},
    inject::{Context, Get, Inject},
    template::{ApplyContext, CountContext, DeclareContext, InitializeContext, Spawn, Template},
    world::World,
    write::{self, Write},
    Result,
};

pub struct Create<'a, T: Template> {
    inner: &'a mut Inner<T>,
    entities: &'a mut Entities,
    world: &'a World,
}

pub struct State<T: Template> {
    inner: Inner<T>,
    entities: write::State<Entities>,
}

struct Inner<T: Template> {
    count: Option<usize>,
    defer: Vec<Defer<T>>,
    segment_indices: Vec<SegmentIndices>,
    entity_indices: Vec<EntityIndices>,
    entity_instances: Vec<Entity>,
    entity_roots: Vec<(usize, usize)>,
    initial_state: <Spawn<T> as Template>::State,
    initial_roots: Vec<Spawn<T>>,
}

struct Defer<T: Template> {
    index: usize,
    initial_roots: Vec<Spawn<T>>,
    entity_roots: Vec<(usize, usize)>,
    entity_instances: Vec<Entity>,
    entity_indices: Vec<EntityIndices>,
    segment_indices: Vec<SegmentIndices>,
}

impl<T: Template> Create<'_, T> {
    pub fn all(&mut self, templates: impl IntoIterator<Item = T>) -> Families {
        match self.inner.count {
            Some(count) => self
                .inner
                .all_static(count, templates, self.entities, self.world),
            None => self.inner.all_dynamic(templates, self.entities, self.world),
        }
    }

    #[inline]
    pub fn one(&mut self, template: T) -> Family {
        self.all(once(template)).roots().next().unwrap()
    }

    #[inline]
    pub fn clones(&mut self, count: usize, template: T) -> Families
    where
        T: Clone,
    {
        self.all((0..count).map(move |_| template.clone()))
    }

    #[inline]
    pub fn defaults(&mut self, count: usize) -> Families
    where
        T: Default,
    {
        self.all((0..count).map(|_| T::default()))
    }
}

impl<T: Template> Inner<T> {
    fn all_static(
        &mut self,
        count: usize,
        templates: impl IntoIterator<Item = T>,
        entities: &mut Entities,
        world: &World,
    ) -> Families {
        self.initial_roots.clear();
        for template in templates {
            self.initial_roots.push(Spawn::new(template));
        }

        if self.initial_roots.len() == 0 {
            return Families::EMPTY;
        }

        self.entity_roots.truncate(self.initial_roots.len());
        while self.entity_roots.len() < self.initial_roots.len() {
            self.entity_roots.push((self.entity_roots.len(), 0));
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
        templates: impl IntoIterator<Item = T>,
        entities: &mut Entities,
        world: &World,
    ) -> Families {
        self.entity_roots.clear();
        self.entity_indices.clear();
        self.segment_indices
            .iter_mut()
            .for_each(|indices| indices.count = 0);

        for template in templates {
            self.entity_roots.push((0, self.entity_indices.len()));
            let root = Spawn::new(template);
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

    fn reserve(&mut self, count: usize, entities: &Entities, world: &World) -> (usize, bool) {
        self.entity_instances.resize(count, Entity::NULL);
        let ready = entities.reserve(&mut self.entity_instances);
        let mut last = 0;
        let mut segment_index = 0;
        let mut success = true;
        let multiplier = self.entity_instances.len() / self.entity_indices.len();

        for (i, segment_indices) in self.segment_indices.iter_mut().enumerate() {
            segment_indices.index = segment_index;
            let segment_count = segment_indices.count * multiplier;
            if segment_count == 0 {
                continue;
            }

            let segment = &world.segments[segment_indices.segment];
            let pair = segment.reserve(segment_count);
            segment_indices.store = pair.0;
            success &= segment_index <= ready && pair.1 == segment_count;

            if success {
                last = i;
                let instances =
                    &self.entity_instances[segment_index..segment_index + segment_count];
                unsafe { segment.stores[0].set_all(pair.0, instances) };
            }

            segment_index += segment_count;
        }

        (last, success && ready == count)
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

impl<T: Template + 'static> Inject for Create<'_, T> {
    type Input = T::Input;
    type State = State<T>;

    fn initialize(input: Self::Input, mut context: Context) -> Result<Self::State> {
        let entities = <Write<Entities> as Inject>::initialize(None, context.owned())?;
        let world = context.world();
        let mut segment_metas = Vec::new();
        let declare = Spawn::<T>::declare(input, DeclareContext::new(0, &mut segment_metas, world));
        let mut segment_to_index = HashMap::new();
        let mut metas_to_segment = HashMap::new();
        let mut segment_indices = Vec::with_capacity(segment_metas.len());

        for (i, metas) in segment_metas.into_iter().enumerate() {
            let segment = world.get_or_add_segment(&metas).index;
            let index = match segment_to_index.get(&segment) {
                Some(&index) => index,
                None => {
                    let index = segment_indices.len();
                    segment_to_index.insert(segment, segment_indices.len());
                    segment_indices.push(SegmentIndices {
                        segment,
                        count: 0,
                        index: 0,
                        store: 0,
                    });
                    index
                }
            };
            metas_to_segment.insert(i, index);
        }

        let state = Spawn::<T>::initialize(
            declare,
            InitializeContext::new(0, &segment_indices, &metas_to_segment, world),
        );

        let mut entity_indices = Vec::new();
        let count = if Spawn::<T>::static_count(
            &state,
            CountContext::new(
                0,
                &mut segment_indices,
                entity_indices.len(),
                None,
                &mut None,
                &mut entity_indices,
            ),
        )? {
            Some(entity_indices.len())
        } else {
            None
        };

        Ok(State {
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

    fn resolve(state: &mut Self::State, mut context: Context) {
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
                let segment_count = segment_indices.count * multiplier;
                if segment_count == 0 {
                    return;
                }

                let segment = &world.segments[segment_indices.segment];
                let instances = &defer.entity_instances
                    [segment_indices.index..segment_indices.index + segment_count];
                unsafe { segment.stores[0].set_all(segment_indices.store, instances) };
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

fn apply<T: Template>(
    initial_state: &<Spawn<T> as Template>::State,
    initial_roots: impl Iterator<Item = Spawn<T>>,
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
                &mut None,
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

impl<'a, T: Template + 'a> Get<'a> for State<T> {
    type Item = Create<'a, T>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Create {
            inner: &mut self.inner,
            entities: self.entities.get(world),
            world,
        }
    }
}

unsafe impl<T: Template> Depend for State<T> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        let mut dependencies = vec![Dependency::defer::<Entities>()];
        for indices in self.inner.segment_indices.iter() {
            dependencies.push(Dependency::defer::<Entity>().at(indices.segment));
        }
        dependencies
    }
}

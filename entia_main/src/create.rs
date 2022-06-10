use std::{
    collections::HashMap,
    iter::{empty, once},
};

use crate::{
    defer::{self, Resolve},
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    error::Result,
    family::template::{EntityIndices, Families, Family, SegmentIndices},
    inject::{Context, Get, Inject},
    template::{ApplyContext, CountContext, DeclareContext, InitializeContext, Spawn, Template},
    world::World,
    write::Write,
};

pub struct Create<'a, T: Template + 'a> {
    defer: defer::Defer<'a, Outer<T>>,
    inner: &'a mut Inner<T>,
    entities: &'a mut Entities,
    world: &'a World,
}
pub struct State<T: Template>(defer::State<Outer<T>>);

struct Outer<T: Template> {
    inner: Inner<T>,
    entities: Write<Entities>,
}

struct Inner<T: Template> {
    count: Option<usize>,
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
            Some(count) => self.all_static(count, templates),
            None => self.all_dynamic(templates),
        }
    }

    #[inline]
    pub fn one(&mut self, template: T) -> Family {
        self.all(once(template))
            .get(0)
            .expect("There must be have at least one root.")
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

    fn all_static(&mut self, count: usize, templates: impl IntoIterator<Item = T>) -> Families {
        let Create {
            defer,
            inner,
            entities,
            world,
        } = self;
        // 'apply_or_defer' is responsible for clearing 'initial_roots'.
        for template in templates {
            inner.initial_roots.push(Spawn::new(template));
        }

        inner.entity_roots.truncate(inner.initial_roots.len());
        while inner.entity_roots.len() < inner.initial_roots.len() {
            inner.entity_roots.push((inner.entity_roots.len(), 0));
        }

        inner.apply_or_defer(count * inner.initial_roots.len(), defer, entities, world)
    }

    fn all_dynamic(&mut self, templates: impl IntoIterator<Item = T>) -> Families {
        let Create {
            defer,
            inner,
            entities,
            world,
        } = self;

        inner.entity_roots.clear();
        inner.entity_indices.clear();
        inner
            .segment_indices
            .iter_mut()
            .for_each(|indices| indices.count = 0);

        for template in templates {
            inner.entity_roots.push((0, inner.entity_indices.len()));
            let root = Spawn::new(template);
            root.dynamic_count(
                &inner.initial_state,
                CountContext::new(
                    0,
                    &mut inner.segment_indices,
                    inner.entity_indices.len(),
                    None,
                    &mut None,
                    &mut inner.entity_indices,
                ),
            );
            inner.initial_roots.push(root);
        }

        inner.apply_or_defer(inner.entity_indices.len(), defer, entities, world)
    }
}

impl<T: Template> Inner<T> {
    fn apply_or_defer(
        &mut self,
        count: usize,
        defer: &mut defer::Defer<Outer<T>>,
        entities: &mut Entities,
        world: &World,
    ) -> Families {
        if count == 0 {
            return Families::EMPTY;
        }

        match self.reserve(count, entities, world) {
            (_, true) => apply(
                &self.initial_state,
                self.initial_roots.drain(..),
                &self.entity_roots,
                &self.entity_instances,
                &self.entity_indices,
                entities,
                &self.segment_indices,
            ),
            (index, false) => defer.one(Defer {
                index,
                initial_roots: self.initial_roots.drain(..).collect(),
                entity_roots: self.entity_roots.clone(),
                entity_instances: self.entity_instances.clone(),
                entity_indices: self.entity_indices.clone(),
                segment_indices: self.segment_indices.clone(),
            }),
        };

        Families::new(
            &self.entity_roots,
            &self.entity_instances,
            &self.entity_indices,
            &self.segment_indices,
        )
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
                unsafe { segment.entity_store().set_all(pair.0, instances) };
            }

            segment_index += segment_count;
        }

        (last, success && ready == count)
    }
}

impl<T: Template + Send + Sync + 'static> Inject for Create<'_, T>
where
    T::State: Send + Sync,
{
    type Input = T::Input;
    type State = State<T>;

    fn initialize(input: Self::Input, mut context: Context) -> Result<Self::State> {
        let entities = Write::initialize(None, context.owned())?;
        let world = context.world();
        let mut segment_metas = Vec::new();
        let declare = Spawn::<T>::declare(input, DeclareContext::new(0, &mut segment_metas, world));
        let mut segment_to_index = HashMap::new();
        let mut metas_to_segment = HashMap::new();
        let mut segment_indices = Vec::with_capacity(segment_metas.len());

        for (i, metas) in segment_metas.into_iter().enumerate() {
            let segment = world.get_or_add_segment(metas).index();
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

        let state = Outer {
            inner: Inner {
                count,
                initial_state: state,
                initial_roots: Vec::new(),
                entity_indices,
                entity_instances: Vec::new(),
                entity_roots: Vec::new(),
                segment_indices,
            },
            entities,
        };
        Ok(State(defer::Defer::initialize(state, context)?))
    }

    fn resolve(State(state): &mut Self::State, mut context: Context) -> Result {
        // Must resolve unconditionally entities and segments *even* if nothing was deferred in the case where creation
        // was completed at run time.
        state.as_mut().resolve(empty(), context.world())?;
        defer::Defer::resolve(state, context.owned())
    }
}

impl<T: Template> Resolve for Outer<T> {
    type Item = Defer<T>;

    fn resolve(&mut self, items: impl Iterator<Item = Self::Item>, world: &mut World) -> Result {
        let inner = &mut self.inner;
        let entities = self.entities.as_mut();
        entities.resolve();

        for segment_indices in inner.segment_indices.iter() {
            world.segments[segment_indices.segment].resolve();
        }

        for defer in items {
            let multiplier = defer.entity_instances.len() / defer.entity_indices.len();
            for segment_indices in defer.segment_indices[defer.index..].iter() {
                let segment_count = segment_indices.count * multiplier;
                if segment_count == 0 {
                    continue;
                }

                let segment = &mut world.segments[segment_indices.segment];
                let instances = &defer.entity_instances
                    [segment_indices.index..segment_indices.index + segment_count];
                unsafe {
                    segment
                        .entity_store()
                        .set_all(segment_indices.store, instances)
                };
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

        Ok(())
    }
}

fn apply<T: Template>(
    initial_state: &<Spawn<T> as Template>::State,
    initial_roots: impl IntoIterator<Item = Spawn<T>>,
    entity_roots: &[(usize, usize)],
    entity_instances: &[Entity],
    entity_indices: &[EntityIndices],
    entities: &mut Entities,
    segment_indices: &[SegmentIndices],
) {
    for (root, &(entity_root, mut entity_count)) in initial_roots.into_iter().zip(entity_roots) {
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

impl<'a, T: Template + 'static> Get<'a> for State<T> {
    type Item = Create<'a, T>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        let (defer, state) = self.0.get(world);
        Create {
            defer,
            inner: &mut state.inner,
            entities: state.entities.get(world),
            world,
        }
    }
}

unsafe impl<T: Template + 'static> Depend for State<T> {
    fn depend(&self, world: &World) -> Vec<Dependency> {
        let mut dependencies = self.0.depend(world);
        let state = self.0.as_ref();
        dependencies.push(Dependency::defer::<Entities>());
        for indices in state.inner.segment_indices.iter() {
            dependencies.push(Dependency::defer::<Entity>().at(indices.segment));
        }
        dependencies
    }
}

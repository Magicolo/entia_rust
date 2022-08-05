use crate::{
    defer::{self, Resolve},
    depend::{Depend, Dependency},
    entities::{Datum, Entities},
    entity::Entity,
    error::Result,
    family::template::{EntityIndices, Families, Family, SegmentIndices},
    inject::{Get, Inject},
    meta::Metas,
    resource::Write,
    segment::Segments,
    template::{ApplyContext, CountContext, DeclareContext, InitializeContext, Spawn, Template},
    world::World,
};
use entia_core::FullIterator;
use std::{collections::HashMap, iter::empty};

pub struct Create<'a, T: Template + 'a> {
    defer: defer::Defer<'a, Outer<T>>,
    inner: &'a mut Inner<T>,
    entities: &'a Entities,
    segments: &'a Segments,
}
pub struct State<T: Template>(defer::State<Outer<T>>);

struct Outer<T: Template> {
    inner: Inner<T>,
    entities: Write<Entities>,
    segments: Write<Segments>,
}

struct Inner<T: Template> {
    count: Option<usize>,
    segment_indices: Box<[SegmentIndices]>,
    entity_indices: Vec<EntityIndices>,
    entity_instances: Vec<Entity>,
    entity_roots: Vec<(usize, usize)>,
    initial_state: <Spawn<T> as Template>::State,
    initial_roots: Vec<Spawn<T>>,
    initialize: Vec<(u32, Datum)>,
}

struct Defer<T: Template> {
    index: usize,
    initial_roots: Vec<Spawn<T>>,
    entity_roots: Vec<(usize, usize)>,
    entity_instances: Vec<Entity>,
    entity_indices: Vec<EntityIndices>,
    segment_indices: Box<[SegmentIndices]>,
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
        self.all([template])
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
            segments,
        } = self;
        // 'apply_or_defer' is responsible for clearing 'initial_roots'.
        inner
            .initial_roots
            .extend(templates.into_iter().map(Spawn::new));
        inner.entity_roots.truncate(inner.initial_roots.len());
        while inner.entity_roots.len() < inner.initial_roots.len() {
            inner.entity_roots.push((inner.entity_roots.len(), 0));
        }

        inner.apply_or_defer(count * inner.initial_roots.len(), defer, entities, segments)
    }

    fn all_dynamic(&mut self, templates: impl IntoIterator<Item = T>) -> Families {
        let Create {
            defer,
            inner,
            entities,
            segments,
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
                    &mut inner.segment_indices,
                    &mut None,
                    &mut inner.entity_indices,
                ),
            );
            inner.initial_roots.push(root);
        }

        inner.apply_or_defer(inner.entity_indices.len(), defer, entities, segments)
    }
}

impl<T: Template> Inner<T> {
    fn apply_or_defer(
        &mut self,
        count: usize,
        defer: &mut defer::Defer<Outer<T>>,
        entities: &Entities,
        segments: &Segments,
    ) -> Families {
        if count == 0 {
            return Families::EMPTY;
        }

        match self.reserve(count, entities, segments) {
            (_, true) => apply(
                &self.initial_state,
                self.initial_roots.drain(..),
                &self.entity_roots,
                &self.entity_instances,
                &self.entity_indices,
                &self.segment_indices,
                &mut self.initialize,
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

    fn reserve(&mut self, count: usize, entities: &Entities, segments: &Segments) -> (usize, bool) {
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

            let segment = &segments[segment_indices.segment];
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
    type Input = ();
    type State = State<T>;

    fn initialize(_: Self::Input, identifier: usize, world: &mut World) -> Result<Self::State> {
        let entities = Write::initialize(None, identifier, world)?;
        let mut metas = Write::<Metas>::initialize(None, identifier, world)?;
        let mut segments = Write::<Segments>::initialize(None, identifier, world)?;
        let mut segment_metas = Vec::new();
        let initial = Spawn::<T>::declare(DeclareContext::new(0, &mut segment_metas, &mut metas));
        let mut segment_to_index = HashMap::new();
        let mut metas_to_segment = HashMap::new();
        let mut segment_indices = Vec::with_capacity(segment_metas.len());
        let entity_meta = metas.entity();

        for (i, metas) in segment_metas.into_iter().enumerate() {
            let segment = segments.get_or_add(entity_meta.clone(), metas).index();
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

        let mut segment_indices = segment_indices.into_boxed_slice();
        let state = Spawn::<T>::initialize(
            initial,
            InitializeContext::new(0, &segment_indices, &metas_to_segment, &segments),
        );

        let mut entity_indices = Vec::new();
        let count = if Spawn::<T>::static_count(
            &state,
            CountContext::new(&mut segment_indices, &mut None, &mut entity_indices),
        )? {
            Some(entity_indices.len())
        } else {
            None
        };

        let inner = Inner {
            count,
            initial_state: state,
            initial_roots: Vec::new(),
            entity_indices,
            entity_instances: Vec::new(),
            entity_roots: Vec::new(),
            segment_indices,
            initialize: Vec::new(),
        };
        let outer = Outer {
            inner,
            entities,
            segments,
        };
        Ok(State(defer::Defer::initialize(outer, identifier, world)?))
    }

    fn resolve(State(state): &mut Self::State) -> Result {
        defer::Defer::resolve(state)?;

        // If entities have successfully been reserved at run time, no item would've been deferred, so resolution is triggered manually.
        let outer = state.as_mut();
        if outer.inner.initialize.len() > 0 {
            outer.resolve(empty())?;
        }

        debug_assert_eq!(outer.inner.initialize.len(), 0);
        Ok(())
    }
}

impl<T: Template> Resolve for Outer<T> {
    type Item = Defer<T>;

    fn resolve(&mut self, items: impl FullIterator<Item = Self::Item>) -> Result {
        let inner = &mut self.inner;
        let segments = &mut self.segments;
        self.entities.resolve();

        for segment_indices in inner.segment_indices.iter() {
            segments[segment_indices.segment].resolve();
        }

        for defer in items {
            let multiplier = defer.entity_instances.len() / defer.entity_indices.len();
            for segment_indices in defer.segment_indices[defer.index..].iter() {
                let segment_count = segment_indices.count * multiplier;
                if segment_count == 0 {
                    continue;
                }

                let segment = &mut segments[segment_indices.segment];
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
                defer.initial_roots,
                &defer.entity_roots,
                &defer.entity_instances,
                &defer.entity_indices,
                &defer.segment_indices,
                &mut inner.initialize,
            );
        }

        for (index, datum) in inner.initialize.drain(..) {
            self.entities.initialize(index, datum);
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
    segment_indices: &[SegmentIndices],
    initialize: &mut Vec<(u32, Datum)>,
) {
    for (root, &entity_root) in initial_roots.into_iter().zip(entity_roots) {
        root.apply(
            initial_state,
            ApplyContext::new(
                entity_root,
                entity_instances,
                entity_indices,
                segment_indices,
                initialize,
            ),
        );
    }
}

impl<'a, T: Template + 'static> Get<'a> for State<T> {
    type Item = Create<'a, T>;

    #[inline]
    unsafe fn get(&'a mut self) -> Self::Item {
        let (defer, outer) = self.0.get();
        Create {
            defer,
            inner: &mut outer.inner,
            entities: &outer.entities,
            segments: &outer.segments,
        }
    }
}

unsafe impl<T: Template + 'static> Depend for State<T> {
    fn depend(&self) -> Vec<Dependency> {
        let mut dependencies = self.0.depend();
        let state = self.0.as_ref();
        dependencies.push(Dependency::defer::<Entities>());
        for indices in state.inner.segment_indices.iter() {
            dependencies.push(Dependency::defer::<Entity>().at(indices.segment));
        }
        dependencies
    }
}

use crate::{
    defer::{self, Resolve},
    depend::Dependency,
    entities::{Datum, Entities},
    entity::Entity,
    error::Result,
    family::template::{EntityIndices, Families, Family, SegmentIndices},
    inject::{Adapt, Context, Get, Inject},
    meta::Metas,
    resource::{Read, Write},
    segment::Segments,
    template::{ApplyContext, CountContext, DeclareContext, InitializeContext, Spawn, Template},
};
use entia_core::FullIterator;
use std::collections::HashMap;

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
    metas: Write<Metas>,
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

/*
TODO: Add a resolve parameter to `Create<'a, T, R = Early>` to allow resolving creation of entities at the end.
- `Create<'a, T, Late>` would have its own versions of creation functions (`all`, `one`, etc.) that does not give out entities.
    - This would prevent other systems from observing the uninitialized entity.
*/

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
                &mut self.initial_roots,
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

unsafe impl<T: Template + Send + Sync + 'static> Inject for Create<'_, T>
where
    T::State: Send + Sync,
{
    type Input = ();
    type State = State<T>;

    fn initialize<A: Adapt<Self::State>>(
        _: Self::Input,
        mut context: Context<Self::State, A>,
    ) -> Result<Self::State> {
        let entities =
            Write::initialize(None, context.map(|state| &mut state.0.as_mut().entities))?;
        let mut metas = Write::initialize(None, context.map(|state| &mut state.0.as_mut().metas))?;
        let mut segments =
            Write::initialize(None, context.map(|state| &mut state.0.as_mut().segments))?;
        let mut segment_metas = Vec::new();
        let initial = Spawn::<T>::declare(DeclareContext::new(0, &mut segment_metas, &mut metas));
        let mut segment_to_index = HashMap::new();
        let mut metas_to_segment = HashMap::new();
        let mut segment_indices = Vec::with_capacity(segment_metas.len());

        for (i, component_metas) in segment_metas.into_iter().enumerate() {
            let segment = segments.get_or_add(component_metas, &metas).index();
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
            metas,
            entities,
            segments,
        };
        Ok(State(defer::Defer::initialize(
            outer,
            context.map(|state| &mut state.0),
        )?))
    }

    fn depend(State(state): &Self::State) -> Vec<Dependency> {
        let Outer {
            entities,
            segments,
            inner,
            ..
        } = state.as_ref();
        let mut dependencies = defer::Defer::depend(state);
        dependencies.extend(Read::depend(&entities.read()));
        dependencies.extend(Read::depend(&segments.read()));
        for &SegmentIndices { segment, .. } in inner.segment_indices.iter() {
            dependencies.push(Dependency::read_at(segments[segment].identifier()));
        }
        dependencies
    }
}

unsafe impl<T: Template> Resolve for Outer<T> {
    type Item = Defer<T>;

    fn pre(&mut self) -> Result {
        self.entities.resolve();
        for &SegmentIndices { segment, .. } in self.inner.segment_indices.iter() {
            self.segments[segment].resolve();
        }
        Ok(())
    }

    fn post(&mut self) -> Result {
        for (index, datum) in self.inner.initialize.drain(..) {
            self.entities.initialize(index, datum);
        }
        Ok(())
    }

    fn resolve(&mut self, items: impl FullIterator<Item = Self::Item>) -> Result {
        let inner = &mut self.inner;
        let segments = &mut self.segments;

        for mut defer in items {
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
                &mut defer.initial_roots,
                &defer.entity_roots,
                &defer.entity_instances,
                &defer.entity_indices,
                &defer.segment_indices,
                &mut inner.initialize,
            );
        }
        Ok(())
    }

    fn depend(&self) -> Vec<Dependency> {
        let Outer {
            inner,
            entities,
            segments,
            ..
        } = self;
        let mut dependencies = Write::depend(entities);
        dependencies.extend(Read::depend(&segments.read()));
        for &SegmentIndices { segment, .. } in inner.segment_indices.iter() {
            dependencies.push(Dependency::write_at(segments[segment].identifier()));
        }
        dependencies
    }
}

fn apply<T: Template>(
    initial_state: &<Spawn<T> as Template>::State,
    initial_roots: &mut Vec<Spawn<T>>,
    entity_roots: &[(usize, usize)],
    entity_instances: &[Entity],
    entity_indices: &[EntityIndices],
    segment_indices: &[SegmentIndices],
    initialize: &mut Vec<(u32, Datum)>,
) {
    for (root, &entity_root) in initial_roots.drain(..).zip(entity_roots) {
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

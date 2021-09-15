use std::{any::TypeId, array::IntoIter, collections::HashMap, iter::from_fn, mem::replace};

use crate::{
    depend::{Depend, Dependency},
    entities::Entities,
    entity::Entity,
    initial::{
        child, ApplyContext, Child, CountContext, DeclareContext, Indices, Initial,
        InitializeContext,
    },
    inject::{Context, Get, Inject},
    segment::Segment,
    world::World,
    write::{self, Write},
};

pub struct Create<'a, I: Initial> {
    count: Option<usize>,
    defer: &'a mut Vec<Defer<I>>,
    entities: &'a mut Entities,
    world: &'a World,
    segments: &'a Vec<usize>,
    segment_counts: &'a mut Vec<usize>,
    segment_indices: &'a mut Vec<usize>,
    store_indices: &'a mut Vec<usize>,
    entity_indices: &'a mut Vec<Indices>,
    entity_instances: &'a mut Vec<Entity>,
    entity_roots: &'a mut Vec<usize>,
    initial_state: &'a <Child<I> as Initial>::State,
    initial_roots: &'a mut Vec<Child<I>>,
}

pub struct State<I: Initial> {
    count: Option<usize>,
    defer: Vec<Defer<I>>,
    entities: write::State<Entities>,
    segments: Vec<usize>,
    segment_counts: Vec<usize>,
    segment_indices: Vec<usize>,
    store_indices: Vec<usize>,
    entity_indices: Vec<Indices>,
    entity_instances: Vec<Entity>,
    entity_roots: Vec<usize>,
    initial_state: <Child<I> as Initial>::State,
    initial_roots: Vec<Child<I>>,
}

#[derive(Clone, Copy)]
pub struct Family<'a> {
    entity_index: usize,
    entity_indices: &'a [Indices],
    entity_instances: &'a [Entity],
    segment_indices: &'a [usize],
}

pub struct Families<'a> {
    indices: &'a [usize],
    entity_indices: &'a [Indices],
    entity_instances: &'a [Entity],
    segment_indices: &'a [usize],
}

pub struct FamiliesIterator<'a, 'b> {
    index: usize,
    families: &'b Families<'a>,
}

struct Defer<I: Initial> {
    head: usize,
    entity_indices: Vec<Indices>,
    entity_instances: Vec<Entity>,
    store_indices: Vec<usize>,
    segment_indices: Vec<usize>,
    segment_counts: Vec<usize>,
    initial_roots: Vec<Child<I>>,
}

impl<'a> Family<'a> {
    pub const EMPTY: Self = Self {
        entity_index: 0,
        entity_indices: &[],
        entity_instances: &[],
        segment_indices: &[],
    };

    #[inline]
    pub const fn new(
        entity_index: usize,
        entity_indices: &'a [Indices],
        entity_instances: &'a [Entity],
        segment_indices: &'a [usize],
    ) -> Self {
        Self {
            entity_index,
            entity_indices,
            entity_instances,
            segment_indices,
        }
    }

    #[inline]
    pub fn entity(&self) -> Entity {
        let indices = &self.entity_indices[self.entity_index];
        let index = self.segment_indices[indices.segment] + indices.offset;
        self.entity_instances[index]
    }

    pub fn parent(&self) -> Option<Self> {
        Some(self.with(self.entity_indices[self.entity_index].parent?))
    }

    pub fn root(&self) -> Self {
        // Do not assume that index '0' is the root since there might be multiple roots.
        self.parent().map(|parent| parent.root()).unwrap_or(*self)
    }

    pub fn next(&self) -> impl Iterator<Item = Family<'a>> + '_ {
        let mut next = self.entity_indices[self.entity_index].next;
        from_fn(move || {
            let current = next?;
            next = self.entity_indices[current].next;
            Some(self.with(current))
        })
    }

    pub fn children(&self) -> impl Iterator<Item = Family<'a>> + '_ {
        let parent = Some(self.entity_index);
        let mut next = parent.map(|index| index + 1).filter(|&index| {
            index < self.entity_indices.len() && self.entity_indices[index].parent == parent
        });

        from_fn(move || {
            let current = next?;
            next = self.entity_indices[current].next;
            Some(self.with(current))
        })
    }

    pub fn ancestors(&self) -> impl Iterator<Item = Family<'a>> + '_ {
        let mut next = self.entity_indices[self.entity_index].parent;
        from_fn(move || {
            let current = next?;
            next = self.entity_indices[current].parent;
            Some(self.with(current))
        })
    }

    pub fn siblings(&self) -> impl Iterator<Item = Family<'a>> + '_ {
        let parent = self.entity_indices[self.entity_index].parent;
        let mut next = parent.map(|index| index + 1).filter(|&index| {
            index < self.entity_indices.len() && self.entity_indices[index].parent == parent
        });

        from_fn(move || {
            while let Some(current) = next {
                next = self.entity_indices[current].next;
                if current != self.entity_index {
                    return Some(self.with(current));
                }
            }
            None
        })
    }

    fn with(&self, entity_index: usize) -> Self {
        Self::new(
            entity_index,
            self.entity_indices,
            self.entity_instances,
            self.segment_indices,
        )
    }
}

impl Families<'_> {
    pub const EMPTY: Self = Self {
        entity_indices: &[],
        entity_instances: &[],
        indices: &[],
        segment_indices: &[],
    };

    pub fn root(&self, index: usize) -> Option<Entity> {
        self.roots().nth(index)
    }

    pub fn roots(&self) -> impl ExactSizeIterator<Item = Entity> + '_ {
        self.into_iter().map(|family| family.entity())
    }
}

impl<'a, 'b> IntoIterator for &'b Families<'a> {
    type Item = Family<'a>;
    type IntoIter = FamiliesIterator<'a, 'b>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            index: 0,
            families: self,
        }
    }
}

impl<'a, 'b> Iterator for FamiliesIterator<'a, 'b> {
    type Item = Family<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let root = *self.families.indices.get(self.index)?;
        self.index += 1;
        Some(Family::new(
            root,
            self.families.entity_indices,
            self.families.entity_instances,
            self.families.segment_indices,
        ))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'a, 'b> ExactSizeIterator for FamiliesIterator<'a, 'b> {
    fn len(&self) -> usize {
        self.families.indices.len() - self.index
    }
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
        self.initial_roots.extend(initials.map(child));
        if self.initial_roots.len() == 0 {
            return Families::EMPTY;
        }

        let total = self.initial_roots.len() * count;
        self.entity_instances.resize(total, Entity::ZERO);
        let ready = self.entities.reserve(self.entity_instances);
        let mut head = 0;

        for i in 0..self.segments.len() {
            // 'segment_indices' is not constant because it depends on 'initial_roots.len()' which may vary
            self.segment_indices[i] = head;
            let segment_count = self.segment_counts[i] * self.initial_roots.len();
            let segment = &self.world.segments[self.segments[i]];
            let pair = segment.reserve(segment_count);
            self.store_indices[i] = pair.0;

            if pair.1 == segment_count {
                let tail = head + segment_count;
                set_and_initialize(
                    pair.0,
                    &self.entity_instances[head..tail],
                    segment,
                    self.entities,
                );
                head = tail;
            }
        }

        if head == ready && ready == total {
            for root in self.initial_roots.drain(..) {
                root.apply(
                    self.initial_state,
                    ApplyContext::new(
                        0,
                        self.segment_indices,
                        self.store_indices,
                        0,
                        &mut 0,
                        &self.entity_indices,
                        &self.entity_instances,
                    ),
                );
            }
            Families {
                entity_indices: self.entity_indices,
                entity_instances: self.entity_instances,
                indices: self.entity_roots,
                segment_indices: self.segment_indices,
            }
        } else {
            self.defer.push(Defer {
                head,
                entity_indices: replace(self.entity_indices, Vec::new()),
                entity_instances: replace(self.entity_instances, Vec::new()),
                store_indices: replace(self.store_indices, vec![0; self.store_indices.len()]),
                segment_indices: replace(self.segment_indices, vec![0; self.segment_indices.len()]),
                segment_counts: replace(self.segment_counts, vec![0; self.segment_counts.len()]),
                initial_roots: replace(self.initial_roots, Vec::new()),
            });
            let defer = self.defer.last().unwrap();
            Families {
                entity_indices: &defer.entity_indices,
                entity_instances: &defer.entity_instances,
                indices: &self.entity_roots,
                segment_indices: &defer.segment_indices,
            }
        }
    }

    fn all_dynamic(&mut self, initials: impl Iterator<Item = I>) -> Families {
        self.entity_roots.clear();
        self.entity_indices.clear();
        self.segment_counts.iter_mut().for_each(|value| *value = 0);

        for initial in initials {
            self.entity_roots.push(self.entity_indices.len());
            let root = child(initial);
            root.dynamic_count(
                self.initial_state,
                CountContext::new(0, self.segment_counts, 0, None, None, self.entity_indices),
            );
            self.initial_roots.push(root);
        }

        let total = self.entity_indices.len();
        if total == 0 {
            return Families::EMPTY;
        }

        self.entity_instances.resize(total, Entity::ZERO);
        let ready = self.entities.reserve(self.entity_instances);
        let mut head = 0;

        for i in 0..self.segments.len() {
            self.segment_indices[i] = head;
            let count = self.segment_counts[i];
            if count == 0 {
                continue;
            }

            let segment = &self.world.segments[self.segments[i]];
            let pair = segment.reserve(count);
            self.store_indices[i] = pair.0;

            if pair.1 == count {
                let tail = head + count;
                set_and_initialize(
                    pair.0,
                    &self.entity_instances[head..tail],
                    segment,
                    self.entities,
                );
                head = tail;
            }
        }

        if head == ready && ready == total {
            let mut entity_count = 0;
            for root in self.initial_roots.drain(..) {
                root.apply(
                    self.initial_state,
                    ApplyContext::new(
                        0,
                        self.segment_indices,
                        self.store_indices,
                        0,
                        &mut entity_count,
                        &self.entity_indices,
                        &self.entity_instances,
                    ),
                );
            }
            Families {
                entity_indices: self.entity_indices,
                entity_instances: self.entity_instances,
                indices: self.entity_roots,
                segment_indices: self.segment_indices,
            }
        } else {
            self.defer.push(Defer {
                head,
                entity_indices: replace(self.entity_indices, Vec::new()),
                entity_instances: replace(self.entity_instances, Vec::new()),
                store_indices: replace(self.store_indices, vec![0; self.store_indices.len()]),
                segment_indices: replace(self.segment_indices, vec![0; self.segment_indices.len()]),
                segment_counts: replace(self.segment_counts, vec![0; self.segment_counts.len()]),
                initial_roots: replace(self.initial_roots, Vec::new()),
            });
            let defer = self.defer.last().unwrap();
            Families {
                entity_indices: &defer.entity_indices,
                entity_instances: &defer.entity_instances,
                indices: &self.entity_roots,
                segment_indices: &defer.segment_indices,
            }
        }
    }
}

/*
child(
    Head,
    child(
        Body,
        child(
            child(Leg),
            child(Leg),
            child(Leg),
        )
    ),
    child(Body,
        child(Leg),
        child(Leg),
    ),
)

child(
    Head,
    child(
        Body,
        child(
            child(Leg),
            child(Leg),
        ),
    ),
    child(Leg),
    child(Leg),
)

Head1                Head2
|                    |
Body1-Body2          Body3-Leg7-Leg8
|              |     |
Leg1-Leg2-Leg3 Leg4  Leg5-Leg6

entities: [Head1, Head2, Body1, Body2, Body3, Leg1, Leg2, Leg3, Leg4, Leg5, Leg6, Leg7, Leg8]
segment_counts: [Head: 2, Body: 3, Leg: 8]
- with 'entities' and 'segment_counts', 'segment.reserve(...)' and 'segment.stores[0].set_all(...)' can be resolved.
segment_indices: [0, 2, 5]
- represents the start indices of entities for each segment

store_indices: [Head: 0, Body: 0, Leg: 0]
- the indices must be incremented by 'Child<I>'

entity_indices: [Head1: 0, Body1: 0, Leg1: 0, Leg2: 1, Leg3: 2, Body2: 1, Leg4: 3, Head2: 1, Body3: 2, Leg5: 4, Leg6: 5, Leg7: 6, Leg8: 7]
entity_parents: [None, None, 0, 0, 1, ]
entity_indices: [Head1: (0, None), Body1: (2, Some(0)), Leg1, Leg2, Leg3, Body2, Leg4, Head2, Body3, Leg5, Leg6, Leg7, Leg8]


// TODO: investigate if some buffers can be eliminated or shrinked with this arrangement (such as 'entity_roots')
entity_indices: [
    Head1(0, None, 1, 2),
    [Body1(2, Some(0), 3, 3), Body2(3, Some(0), 6, 1)],
    [Leg1, Leg2, Leg3],
    Leg4,
    Head2(1, None, 8, 3),
    [Body3, Leg7, Leg8],
    [Leg5, Leg6]
]
*/

impl<I: Initial> Inject for Create<'_, I> {
    type Input = I::Input;
    type State = State<I>;

    fn initialize(input: Self::Input, context: &Context, world: &mut World) -> Option<Self::State> {
        let entities = <Write<Entities> as Inject>::initialize(None, context, world)?;
        let mut segment_metas = Vec::new();
        let declare = Child::<I>::declare(input, DeclareContext::new(0, &mut segment_metas, world));

        let mut segment_to_index = HashMap::new();
        let mut metas_to_segment = HashMap::new();
        let mut segments = Vec::with_capacity(segment_metas.len());
        for (i, metas) in segment_metas.drain(..).enumerate() {
            let segment = world.get_or_add_segment_by_metas(&metas).index;
            let index = match segment_to_index.get(&segment) {
                Some(&index) => index,
                None => {
                    segment_to_index.insert(segment, segments.len());
                    segments.push(segment);
                    i
                }
            };
            metas_to_segment.insert(i, index);
        }

        let state = Child::<I>::initialize(
            declare,
            InitializeContext::new(0, &segments, &metas_to_segment, world),
        );

        let mut entity_indices = Vec::new();
        let mut segment_counts = vec![0; segments.len()];
        let count = if Child::<I>::static_count(
            &state,
            CountContext::new(0, &mut segment_counts, 0, None, None, &mut entity_indices),
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
            entity_indices: Vec::new(),
            entity_instances: Vec::new(),
            entity_roots: Vec::new(),
            store_indices: vec![0; segments.len()],
            segment_indices: vec![0; segments.len()],
            segment_counts: vec![0; segments.len()],
            segments,
        })
    }

    fn resolve(state: &mut Self::State, world: &mut World) {
        let entities = state.entities.as_mut();
        entities.resolve();

        for mut defer in state.defer.drain(..) {
            if defer.entity_instances.len() == 0 || defer.initial_roots.len() == 0 {
                continue;
            }

            let mut head = 0;
            for i in 0..state.segments.len() {
                let count = defer.segment_counts[i];
                if head < defer.head || count == 0 {
                    head += count;
                    continue;
                }

                let segment = &mut world.segments[state.segments[i]];
                segment.resolve();

                let tail = head + count;
                set_and_initialize(
                    defer.store_indices[i],
                    &defer.entity_instances[head..tail],
                    segment,
                    entities,
                );
                head = tail;
            }

            let mut count = 0;
            for root in defer.initial_roots.drain(..) {
                root.apply(
                    &state.initial_state,
                    ApplyContext::new(
                        0,
                        &defer.segment_indices,
                        &defer.store_indices,
                        0,
                        &mut count,
                        &defer.entity_indices,
                        &defer.entity_instances,
                    ),
                );
            }
        }
    }
}

#[inline]
fn set_and_initialize(
    index: usize,
    instances: &[Entity],
    segment: &Segment,
    entities: &mut Entities,
) {
    // The first store of a segment with entities is always the entity store.
    unsafe { segment.stores[0].set_all(index, instances) };
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
            store_indices: &mut self.store_indices,
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

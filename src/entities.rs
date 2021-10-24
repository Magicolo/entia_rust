use self::family::*;
use crate::{
    depend::{Depend, Dependency},
    entity::Entity,
    inject::{Get, Inject, InjectContext},
    query::item::{At, Item, ItemContext},
    read::{self, Read},
    resource::Resource,
    world::World,
    write::{self, Write},
};
use std::{
    cmp::{max, min},
    fmt,
    iter::from_fn,
    sync::atomic::{AtomicIsize, AtomicUsize, Ordering},
    vec,
};

pub struct Entities {
    pub(crate) free: (Vec<Entity>, AtomicIsize),
    pub(crate) data: (Vec<Datum>, AtomicUsize),
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Datum {
    pub(crate) generation: u32,
    pub(crate) store_index: u32,
    pub(crate) segment_index: u32,
    pub(crate) parent: u32,
    pub(crate) first_child: u32,
    pub(crate) last_child: u32,
    pub(crate) previous_sibling: u32,
    pub(crate) next_sibling: u32,
}

impl Resource for Entities {}

impl Datum {
    pub const DEFAULT: Datum = Datum {
        generation: 0,
        store_index: u32::MAX,
        segment_index: u32::MAX,
        parent: u32::MAX,
        first_child: u32::MAX,
        last_child: u32::MAX,
        previous_sibling: u32::MAX,
        next_sibling: u32::MAX,
    };

    #[inline]
    pub fn initialize(
        &mut self,
        generation: u32,
        store_index: u32,
        segment_index: u32,
        parent: Option<u32>,
        first_child: Option<u32>,
        last_child: Option<u32>,
        previous_sibling: Option<u32>,
        next_sibling: Option<u32>,
    ) -> bool {
        if self.released() {
            *self = Datum {
                generation,
                store_index,
                segment_index,
                parent: parent.unwrap_or(u32::MAX),
                first_child: first_child.unwrap_or(u32::MAX),
                last_child: last_child.unwrap_or(u32::MAX),
                previous_sibling: previous_sibling.unwrap_or(u32::MAX),
                next_sibling: next_sibling.unwrap_or(u32::MAX),
            };
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn update(&mut self, store_index: u32, segment_index: u32) -> bool {
        if self.initialized() {
            self.store_index = store_index;
            self.segment_index = segment_index;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn release(&mut self) -> bool {
        if self.initialized() {
            self.store_index = u32::MAX;
            self.segment_index = u32::MAX;
            true
        } else {
            false
        }
    }

    #[inline]
    pub const fn valid(&self, generation: u32) -> bool {
        self.generation == generation && self.initialized()
    }

    #[inline]
    pub const fn initialized(&self) -> bool {
        self.store_index < u32::MAX && self.segment_index < u32::MAX
    }

    #[inline]
    pub const fn released(&self) -> bool {
        self.store_index == u32::MAX && self.segment_index == u32::MAX
    }
}

impl Default for Datum {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Entities {
    pub fn new(capacity: usize) -> Self {
        Self {
            free: (Vec::with_capacity(capacity), 0.into()),
            data: (Vec::with_capacity(capacity), 0.into()),
        }
    }

    pub fn has(&self, entity: Entity) -> bool {
        self.get_datum(entity).is_some()
    }

    pub(crate) fn reserve(&self, entities: &mut [Entity]) -> usize {
        if entities.len() == 0 {
            return 0;
        }

        let count = entities.len() as isize;
        let last = self.free.1.fetch_sub(count, Ordering::Relaxed);
        let count = max(min(count, last), 0) as usize;
        for i in 0..count {
            let index = last as usize - i - 1;
            let mut entity = self.free.0[index];
            // TODO: What to do if there is an overflow?
            // Overflow could be ignored since it is highly unlikely that entities of early generations are still stored somewhere,
            // but this fact could be exploited...
            // Also, at 'index == 0', a generation of 0 must not be allowed.
            entity.generation = self.data.0[entity.index as usize].generation + 1;
            entities[i] = entity;
        }

        let remaining = entities.len() - count;
        if remaining == 0 {
            return count;
        }

        // TODO: What to do if 'index + remaining >= u32::MAX'?
        // Note that 'u32::MAX' is used as a sentinel so it must be an invalid entity index.
        let index = self.data.1.fetch_add(remaining, Ordering::Relaxed);
        for i in 0..remaining {
            entities[count + i] = Entity {
                index: (index + i) as u32,
                generation: 0,
            };
        }
        count
    }

    pub(crate) fn resolve(&mut self) {
        self.data.0.resize(*self.data.1.get_mut(), Datum::DEFAULT);
        let free = self.free.1.get_mut();
        let count = max(*free, 0);
        self.free.0.truncate(count as usize);
        *free = self.free.0.len() as isize;
    }

    pub(crate) fn release(&mut self, entities: impl IntoIterator<Item = Entity>) {
        let index = self.free.0.len();
        self.free.0.extend(entities);
        for &entity in &self.free.0[index..] {
            self.data.0[entity.index as usize].release();
        }
        *self.free.1.get_mut() = self.free.0.len() as isize;
    }

    #[inline]
    pub(crate) fn get_datum(&self, entity: Entity) -> Option<&Datum> {
        self.data
            .0
            .get(entity.index as usize)
            .filter(|datum| datum.valid(entity.generation))
    }

    #[inline]
    pub(crate) fn get_datum_mut(&mut self, entity: Entity) -> Option<&mut Datum> {
        self.data
            .0
            .get_mut(entity.index as usize)
            .filter(|datum| datum.valid(entity.generation))
    }
}

impl Default for Entities {
    #[inline]
    fn default() -> Self {
        Self::new(32)
    }
}

pub mod family {
    use super::*;

    #[derive(Clone)]
    pub struct Family<'a>(Entity, &'a Entities);
    pub struct State(read::State<Entity>, read::State<Entities>);

    #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub enum Horizontal {
        FromLeft,
        FromRight,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
    pub enum Vertical {
        FromTop,
        FromBottom,
    }

    impl Entities {
        #[inline]
        pub const fn family(&self, entity: Entity) -> Family {
            Family(entity, self)
        }

        pub fn roots(&self) -> impl Iterator<Item = Entity> + '_ {
            self.data
                .0
                .iter()
                .enumerate()
                .filter_map(move |(index, datum)| {
                    if datum.initialized() && datum.parent == u32::MAX {
                        Some(Entity {
                            index: index as u32,
                            generation: datum.generation,
                        })
                    } else {
                        None
                    }
                })
        }

        pub fn root(&self, mut entity: Entity) -> Entity {
            // Only the entry entity needs to be validated; linked entities can be assumed to be valid.
            if let Some(datum) = self.get_datum(entity) {
                let mut index = datum.parent;
                while let Some(datum) = self.data.0.get(index as usize) {
                    entity = Entity {
                        index,
                        generation: datum.generation,
                    };
                    index = datum.parent;
                }
            }
            entity
        }

        pub fn parent(&self, entity: Entity) -> Option<Entity> {
            let datum = self.get_datum(entity)?;
            let parent = self.data.0.get(datum.parent as usize)?;
            Some(Entity {
                index: datum.parent,
                generation: parent.generation,
            })
        }

        pub fn children(
            &self,
            entity: Entity,
            direction: Horizontal,
        ) -> impl Iterator<Item = Entity> + '_ {
            match direction {
                Horizontal::FromLeft => self.link_iterator(
                    entity,
                    |parent| parent.first_child,
                    |child| child.next_sibling,
                ),
                Horizontal::FromRight => self.link_iterator(
                    entity,
                    |parent| parent.last_child,
                    |child| child.previous_sibling,
                ),
            }
        }

        pub fn ancestors(
            &self,
            entity: Entity,
            direction: Vertical,
        ) -> impl Iterator<Item = Entity> + '_ {
            enum Ancestors<'a> {
                FromTop(vec::IntoIter<Entity>),
                FromBottom(Entity, &'a Entities),
            }

            impl<'a> Iterator for Ancestors<'a> {
                type Item = Entity;

                fn next(&mut self) -> Option<Self::Item> {
                    match self {
                        Self::FromTop(entities) => entities.next(),
                        Self::FromBottom(entity, entities) => {
                            let parent = entities.parent(*entity)?;
                            *entity = parent;
                            Some(parent)
                        }
                    }
                }
            }

            match direction {
                Vertical::FromTop => {
                    let mut entities = Vec::new();
                    self.ascend(entity, direction, |parent| {
                        entities.push(parent);
                        true
                    });
                    Ancestors::FromTop(entities.into_iter())
                }
                Vertical::FromBottom => Ancestors::FromBottom(entity, self),
            }
        }

        /// Parameter 'each' takes the current descendant and returns a 'bool' that indicates if the descent should continue.
        /// Return value will be 'true' only if all descendants have been visited.
        pub fn descendants(
            &self,
            entity: Entity,
            direction: (Horizontal, Vertical),
        ) -> impl Iterator<Item = Entity> {
            let mut entities = Vec::new();
            self.descend(entity, direction, |child| {
                entities.push(child);
                true
            });
            entities.into_iter()
        }

        pub fn siblings(
            &self,
            entity: Entity,
            direction: Horizontal,
        ) -> impl Iterator<Item = Entity> + '_ {
            self.parent(entity)
                .map(|parent| {
                    self.children(parent, direction)
                        .filter(move |&child| child != entity)
                })
                .into_iter()
                .flatten()
        }

        /// Parameter 'each' takes the current ancestor and returns a 'bool' that indicates if the ascension should continue.
        /// Return value will be 'true' only if all ancestors have been visited.
        pub fn ascend(
            &self,
            entity: Entity,
            direction: Vertical,
            mut each: impl FnMut(Entity) -> bool,
        ) -> bool {
            fn from_top<'a>(
                entities: &Entities,
                entity: Entity,
                each: &mut impl FnMut(Entity) -> bool,
            ) -> bool {
                if let Some(parent) = entities.parent(entity) {
                    from_top(entities, parent, each) && each(parent)
                } else {
                    true
                }
            }

            fn from_bottom<'a>(
                entities: &Entities,
                entity: Entity,
                each: &mut impl FnMut(Entity) -> bool,
            ) -> bool {
                if let Some(parent) = entities.parent(entity) {
                    each(parent.clone()) && from_bottom(entities, parent, each)
                } else {
                    true
                }
            }

            match direction {
                Vertical::FromTop => from_top(self, entity, &mut each),
                Vertical::FromBottom => from_bottom(self, entity, &mut each),
            }
        }

        /// Parameter 'each' takes the current descendant and returns a 'bool' that indicates if the descent should continue.
        /// Return value will be 'true' only if all descendants have been visited.
        pub fn descend(
            &self,
            entity: Entity,
            direction: (Horizontal, Vertical),
            mut each: impl FnMut(Entity) -> bool,
        ) -> bool {
            fn from_top<'a>(
                entities: &Entities,
                entity: Entity,
                direction: Horizontal,
                each: &mut impl FnMut(Entity) -> bool,
            ) -> bool {
                for child in entities.children(entity, direction) {
                    if each(child.clone()) && from_top(entities, child, direction, each) {
                        continue;
                    } else {
                        return false;
                    }
                }
                true
            }

            fn from_bottom<'a>(
                entities: &Entities,
                entity: Entity,
                direction: Horizontal,
                each: &mut impl FnMut(Entity) -> bool,
            ) -> bool {
                for child in entities.children(entity, direction) {
                    if from_bottom(entities, child, direction, each) && each(child) {
                        continue;
                    } else {
                        return false;
                    }
                }
                true
            }

            match direction.1 {
                Vertical::FromTop => from_top(self, entity, direction.0, &mut each),
                Vertical::FromBottom => from_bottom(self, entity, direction.0, &mut each),
            }
        }

        pub fn adopt_first(&mut self, parent: Entity, child: Entity) -> Option<()> {
            let (_, parent_datum) = self.adopt_step1(parent, child)?;
            if let Some(first) = self.data.0.get_mut(parent_datum.first_child as usize) {
                first.previous_sibling = child.index;
            }

            let child_datum = self.data.0.get_mut(child.index as usize)?;
            child_datum.parent = parent.index;
            child_datum.previous_sibling = u32::MAX;
            child_datum.next_sibling = parent_datum.first_child;

            let parent_datum = self.data.0.get_mut(parent.index as usize)?;
            parent_datum.first_child = child.index;
            if parent_datum.last_child == u32::MAX {
                // Happens when the parent has no children.
                parent_datum.last_child = child.index;
            }

            Some(())
        }

        pub fn adopt_last(&mut self, parent: Entity, child: Entity) -> Option<()> {
            let (_, parent_datum) = self.adopt_step1(parent, child)?;
            if let Some(last) = self.data.0.get_mut(parent_datum.last_child as usize) {
                last.next_sibling = child.index;
            }

            let child_datum = self.data.0.get_mut(child.index as usize)?;
            child_datum.parent = parent.index;
            child_datum.previous_sibling = parent_datum.last_child;
            child_datum.next_sibling = u32::MAX;

            let parent_datum = self.data.0.get_mut(parent.index as usize)?;
            parent_datum.last_child = child.index;
            if parent_datum.first_child == u32::MAX {
                // Happens when the parent has no children.
                parent_datum.first_child = child.index;
            }

            Some(())
        }

        pub fn reject_first(&mut self, parent: Entity) -> Option<Entity> {
            let parent_datum = self.get_datum(parent)?;
            let child_index = parent_datum.first_child;
            let child_datum = self.data.0.get(child_index as usize)?.clone();
            if self.reject_step1((child_index, child_datum)) {
                Some(self.reject_step2(child_index))
            } else {
                None
            }
        }

        pub fn reject_last(&mut self, parent: Entity) -> Option<Entity> {
            let parent_datum = self.get_datum(parent)?;
            let child_index = parent_datum.last_child;
            let child_datum = self.data.0.get(child_index as usize)?.clone();
            if self.reject_step1((child_index, child_datum)) {
                Some(self.reject_step2(child_index))
            } else {
                None
            }
        }

        pub fn reject_at(
            &mut self,
            parent: Entity,
            index: usize,
            direction: Horizontal,
        ) -> Option<Entity> {
            let child = self.children(parent, direction).nth(index)?;
            let datum = self.data.0.get(child.index as usize)?.clone();
            if self.reject_step1((child.index, datum)) {
                Some(self.reject_step2(child.index))
            } else {
                None
            }
        }

        pub fn reject_filter(
            &mut self,
            parent: Entity,
            mut filter: impl FnMut(Entity) -> bool,
        ) -> Option<usize> {
            let mut count = 0;
            self.link_each(
                parent,
                |parent| parent.first_child,
                |child| child.next_sibling,
                |index, entities| {
                    let datum = entities.data.0[index as usize].clone();
                    let entity = Entity {
                        index,
                        generation: datum.generation,
                    };
                    if filter(entity) && entities.reject_step1((index, datum)) {
                        entities.reject_step2(index);
                        count += 1;
                    }
                },
            )?;
            Some(count)
        }

        pub fn reject_all(&mut self, parent: Entity) -> Option<usize> {
            let parent_datum = self.get_datum_mut(parent)?;
            parent_datum.first_child = u32::MAX;
            parent_datum.last_child = u32::MAX;

            let mut count = 0;
            self.link_each(
                parent,
                |parent| parent.first_child,
                |child| child.next_sibling,
                |index, entities| {
                    entities.reject_step2(index);
                    count += 1;
                },
            )?;
            Some(count)
        }

        pub fn reject(&mut self, child: Entity) -> Option<bool> {
            let datum = self.get_datum(child)?.clone();
            if self.reject_step1((child.index, datum)) {
                self.reject_step2(child.index);
                Some(true)
            } else {
                Some(false)
            }
        }

        pub(crate) fn adopt_step1(
            &mut self,
            parent: Entity,
            child: Entity,
        ) -> Option<(Datum, Datum)> {
            // A parent entity can adopt an entity that is already its child. In that case, that entity will simply be moved.

            if parent.index == child.index {
                // An entity cannot adopt itself.
                // If generations don't match, then one of the entities is invalid, thus adoption also fails.
                return None;
            }

            // As long as the entry point entities are validated, the linked ones can be assumed to be valid.
            let child_datum = self.get_datum(child)?.clone();
            let parent_datum = self.get_datum(parent)?.clone();

            if !self.ascend(parent, Vertical::FromBottom, |parent| parent != child) {
                // An entity cannot adopt an ancestor.
                return None;
            }

            // The 'reject' step fails when the entity is a root which is fine here.
            self.reject_step1((child.index, child_datum.clone()));
            Some((child_datum, parent_datum))
        }

        pub(crate) fn reject_step1(&mut self, child: (u32, Datum)) -> bool {
            if let Some(parent) = self.data.0.get_mut(child.1.parent as usize) {
                if parent.first_child == child.0 {
                    parent.first_child = child.1.next_sibling;
                }
                if parent.last_child == child.0 {
                    parent.last_child = child.1.previous_sibling;
                }
            } else {
                return false;
            }

            if let Some(previous) = self.data.0.get_mut(child.1.previous_sibling as usize) {
                previous.next_sibling = child.1.next_sibling;
            }

            if let Some(next) = self.data.0.get_mut(child.1.next_sibling as usize) {
                next.previous_sibling = child.1.previous_sibling;
            }

            true
        }

        pub(crate) fn reject_step2(&mut self, child_index: u32) -> Entity {
            let child_datum = &mut self.data.0[child_index as usize];
            child_datum.parent = u32::MAX;
            child_datum.previous_sibling = u32::MAX;
            child_datum.next_sibling = u32::MAX;
            Entity {
                index: child_index,
                generation: child_datum.generation,
            }
        }

        #[inline]
        fn link_iterator(
            &self,
            entity: Entity,
            first: fn(&Datum) -> u32,
            next: fn(&Datum) -> u32,
        ) -> impl Iterator<Item = Entity> + '_ {
            let mut index = self.get_datum(entity).map(first).unwrap_or(u32::MAX);
            from_fn(move || {
                let datum = self.data.0.get(index as usize)?;
                let entity = Entity {
                    index,
                    generation: datum.generation,
                };
                index = next(datum);
                Some(entity)
            })
        }

        #[inline]
        fn link_each(
            &mut self,
            entity: Entity,
            first: fn(&Datum) -> u32,
            next: fn(&Datum) -> u32,
            mut each: impl FnMut(u32, &mut Self),
        ) -> Option<()> {
            let mut index = first(self.get_datum(entity)?);
            while let Some(datum) = self.data.0.get(index as usize) {
                let next = next(datum);
                each(index, self);
                index = next;
            }
            Some(())
        }
    }

    impl<'a> Family<'a> {
        #[inline]
        pub const fn entity(&self) -> Entity {
            self.0
        }

        #[inline]
        pub fn root(&self) -> Self {
            self.1.family(self.1.root(self.0))
        }

        #[inline]
        pub fn parent(&self) -> Option<Self> {
            Some(self.1.family(self.1.parent(self.0)?))
        }

        #[inline]
        pub fn children(&self, direction: Horizontal) -> impl Iterator<Item = Family<'a>> {
            let Family(entity, entities) = *self;
            entities
                .children(entity, direction)
                .map(move |child| entities.family(child))
        }

        #[inline]
        pub fn ancestors(&self, direction: Vertical) -> impl Iterator<Item = Family<'a>> {
            let Family(entity, entities) = *self;
            entities
                .ancestors(entity, direction)
                .map(move |parent| entities.family(parent))
        }

        #[inline]
        pub fn descendants(
            &self,
            direction: (Horizontal, Vertical),
        ) -> impl Iterator<Item = Family<'a>> {
            let Family(entity, entities) = *self;
            entities
                .descendants(entity, direction)
                .map(move |child| entities.family(child))
        }

        #[inline]
        pub fn siblings(&self, direction: Horizontal) -> impl Iterator<Item = Family<'a>> {
            let Family(entity, entities) = *self;
            entities
                .siblings(entity, direction)
                .map(move |sibling| entities.family(sibling))
        }

        /// Parameter 'each' takes the current ancestor and returns a 'bool' that indicates if the ascension should continue.
        /// Return value will be 'true' only if all ancestors have been visited.
        #[inline]
        pub fn ascend(&self, direction: Vertical, mut each: impl FnMut(Self) -> bool) -> bool {
            self.1
                .ascend(self.0, direction, |parent| each(self.1.family(parent)))
        }

        /// Parameter 'each' takes the current descendant and returns a 'bool' that indicates if the descent should continue.
        /// Return value will be 'true' only if all descendants have been visited.
        #[inline]
        pub fn descend(
            &self,
            direction: (Horizontal, Vertical),
            mut each: impl FnMut(Self) -> bool,
        ) -> bool {
            self.1
                .descend(self.0, direction, |child| each(self.1.family(child)))
        }
    }

    impl Into<Entity> for Family<'_> {
        fn into(self) -> Entity {
            self.entity()
        }
    }

    impl fmt::Debug for Family<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct(&format!("{:?}", self.entity()))
                .field("Parent", &self.parent().map(|parent| parent.entity()))
                .field(
                    "Children",
                    &self
                        .children(Horizontal::FromLeft)
                        .map(|child| child.entity())
                        .collect::<Vec<_>>(),
                )
                .finish()
        }
    }

    unsafe impl Item for Family<'_> {
        type State = State;

        fn initialize(mut context: ItemContext) -> Option<Self::State> {
            Some(State(
                <Read<Entity> as Item>::initialize(context.owned())?,
                <Read<Entities> as Inject>::initialize(None, context.into())?,
            ))
        }
    }

    impl<'a> At<'a> for State {
        type Item = Family<'a>;

        fn at(&'a self, index: usize, world: &'a World) -> Self::Item {
            Family(*self.0.at(index, world), self.1.as_ref())
        }
    }

    unsafe impl Depend for State {
        fn depend(&self, world: &World) -> Vec<Dependency> {
            // 'Family' may read entities from any segment.
            let mut dependencies = self.1.depend(world);
            dependencies.push(Dependency::read::<Entity>());
            dependencies
        }
    }
}

pub mod families {
    use super::*;

    pub struct Families<'a>(&'a mut Vec<Defer>, &'a Entities);
    pub struct State(Vec<Defer>, write::State<Entities>);
    enum Defer {
        AdoptFirst(Entity, Entity),
        AdoptLast(Entity, Entity),
        Reject(Entity),
        RejectAll(Entity),
    }

    impl<'a> Families<'a> {
        #[inline]
        pub const fn family(&self, entity: Entity) -> Family<'a> {
            self.1.family(entity)
        }

        pub fn roots(&self) -> impl Iterator<Item = Family<'a>> {
            let entities = self.1;
            entities.roots().map(move |entity| entities.family(entity))
        }

        pub fn adopt_first(&mut self, parent: Entity, child: Entity) {
            self.0.push(Defer::AdoptFirst(parent, child));
        }

        pub fn adopt_last(&mut self, parent: Entity, child: Entity) {
            self.0.push(Defer::AdoptLast(parent, child));
        }

        pub fn reject_first(&mut self, parent: Entity) {
            if let Some(child) = self.1.children(parent, Horizontal::FromLeft).next() {
                self.reject(child);
            }
        }

        pub fn reject_last(&mut self, parent: Entity) {
            if let Some(child) = self.1.children(parent, Horizontal::FromRight).next() {
                self.reject(child);
            }
        }

        pub fn reject_at(&mut self, parent: Entity, index: usize, direction: Horizontal) {
            if let Some(child) = self.1.children(parent, direction).nth(index) {
                self.reject(child);
            }
        }

        pub fn reject(&mut self, child: Entity) {
            self.0.push(Defer::Reject(child));
        }

        pub fn reject_all(&mut self, parent: Entity) {
            Defer::RejectAll(parent);
        }
    }

    unsafe impl Inject for Families<'_> {
        type Input = ();
        type State = State;

        fn initialize(_: Self::Input, context: InjectContext) -> Option<Self::State> {
            Some(State(
                Vec::new(),
                <Write<Entities> as Inject>::initialize(None, context)?,
            ))
        }

        fn resolve(state: &mut Self::State, _: InjectContext) {
            let entities = state.1.as_mut();
            for defer in state.0.drain(..) {
                match defer {
                    Defer::AdoptFirst(parent, child) => {
                        entities.adopt_first(parent, child);
                    }
                    Defer::AdoptLast(parent, child) => {
                        entities.adopt_last(parent, child);
                    }
                    Defer::Reject(child) => {
                        if let Some(datum) = entities.get_datum(child).cloned() {
                            if entities.reject_step1((child.index, datum)) {
                                entities.reject_step2(child.index);
                            }
                        }
                    }
                    Defer::RejectAll(parent) => {
                        entities.reject_all(parent);
                    }
                }
            }
        }
    }

    impl<'a> Get<'a> for State {
        type Item = Families<'a>;

        fn get(&'a mut self, world: &'a World) -> Self::Item {
            Families(&mut self.0, self.1.get(world))
        }
    }

    unsafe impl Depend for State {
        fn depend(&self, _: &World) -> Vec<Dependency> {
            vec![
                Dependency::defer::<Entities>(),
                Dependency::read::<Entity>(),
            ]
        }
    }
}

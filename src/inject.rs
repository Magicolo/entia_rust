use crate::internal::*;
use crate::system::*;
use crate::world::*;
use crate::*;
use std::any::TypeId;
use std::sync::Arc;

pub trait Inject {
    type State;
    fn initialize(world: &World) -> Option<Self::State>;
    fn update(state: &mut Self::State) -> Vec<Dependency>;
    fn resolve(state: &Self::State);
    fn get(state: &Self::State) -> Self;
}

pub struct Group<Q: Query> {
    inner: Arc<WorldInner>,
    queries: Arc<Vec<(Q::State, Arc<SegmentInner>)>>,
}
pub struct GroupIterator<Q: Query> {
    segment: usize,
    index: usize,
    group: Group<Q>,
}

pub struct Defer {}

impl<Q: Query> Group<Q> {
    #[inline]
    pub fn each<O>(&self, each: impl Fn(Q) -> O) {
        for (query, segment) in self.queries.iter() {
            for i in 0..segment.entities.len() {
                each(Q::get(i, query));
            }
        }
    }
}

impl<Q: Query> Clone for Group<Q> {
    fn clone(&self) -> Self {
        Group {
            inner: self.inner.clone(),
            queries: self.queries.clone(),
        }
    }
}

impl<Q: Query> IntoIterator for Group<Q> {
    type Item = Q;
    type IntoIter = GroupIterator<Q>;

    fn into_iter(self) -> Self::IntoIter {
        GroupIterator {
            segment: 0,
            index: 0,
            group: self,
        }
    }
}

impl<Q: Query> IntoIterator for &Group<Q> {
    type Item = Q;
    type IntoIter = GroupIterator<Q>;

    fn into_iter(self) -> Self::IntoIter {
        GroupIterator {
            segment: 0,
            index: 0,
            group: self.clone(),
        }
    }
}

impl<Q: Query> Iterator for GroupIterator<Q> {
    type Item = Q;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((query, segment)) = self.group.queries.get(self.segment) {
            if self.index < segment.entities.len() {
                let query = Q::get(self.index, query);
                self.index += 1;
                return Some(query);
            } else {
                self.segment += 1;
                self.index = 0;
            }
        }
        None
    }
}

impl Defer {
    pub fn create<T>(&self, _entities: &mut [Entity], _template: Template<T>) {}
    pub fn destroy(&self, _entities: &[Entity]) {}
    pub fn add<C: Component>(&self, _entity: Entity, _component: C) {}
    pub fn remove<C: Component>(&self, _entity: Entity) {}
}

impl Inject for Defer {
    type State = ();

    fn initialize(_: &World) -> Option<Self::State> {
        Some(())
    }

    fn update(_: &mut Self::State) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State) {
        todo!()
    }

    #[inline]
    fn get(_: &Self::State) -> Self {
        todo!()
    }
}

impl<R: Resource> Inject for &R {
    type State = (Arc<SegmentInner>, Arc<Vec<Wrap<R>>>);

    fn initialize(world: &World) -> Option<Self::State> {
        let segment = world.find_segment(&[TypeId::of::<R>()])?;
        let store = segment.inner.stores[0].clone().downcast().ok()?;
        Some((segment.inner.clone(), store))
    }

    fn update((segment, _): &mut Self::State) -> Vec<Dependency> {
        vec![Dependency::Read(segment.index, TypeId::of::<R>())]
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get((_, store): &Self::State) -> Self {
        unsafe { &*store[0].0.get() }
    }
}

impl<R: Resource> Inject for &mut R {
    type State = (Arc<SegmentInner>, Arc<Vec<Wrap<R>>>);

    fn initialize(world: &World) -> Option<Self::State> {
        let segment = world.find_segment(&[TypeId::of::<R>()])?;
        let store = segment.inner.stores[0].clone().downcast().ok()?;
        Some((segment.inner.clone(), store))
    }

    fn update((segment, _): &mut Self::State) -> Vec<Dependency> {
        vec![Dependency::Write(segment.index, TypeId::of::<R>())]
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get((_, store): &Self::State) -> Self {
        unsafe { &mut *store[0].0.get() }
    }
}

impl Inject for () {
    type State = ();

    fn initialize(_: &World) -> Option<Self::State> {
        Some(())
    }

    fn update(_: &mut Self::State) -> Vec<Dependency> {
        Vec::new()
    }

    fn resolve(_: &Self::State) {}

    #[inline]
    fn get(_: &Self::State) -> Self {
        ()
    }
}

macro_rules! inject {
    ($($p:ident, $t:ident),+) => {
        impl<$($t: Inject),+> Inject for ($($t),+,) {
            type State = ($($t::State),+,);

            fn initialize(world: &World) -> Option<Self::State> {
                Some(($($t::initialize(world)?),+,))
            }

            fn update(($($p),+,): &mut Self::State) -> Vec<Dependency> {
                let mut dependencies = Vec::new();
                $(dependencies.append(&mut $t::update($p)));+;
                dependencies
            }

            fn resolve(($($p),+,): &Self::State) {
                $($t::resolve($p));+;
            }

            #[inline]
            fn get(($($p),+,): &Self::State) -> Self {
                ($($t::get($p)),+,)
            }
        }
    };
}

crate::recurse!(
    inject, inject1, I1, inject2, I2, inject3, I3, inject4, I4, inject5, I5, inject6, I6, inject7,
    I7, inject8, I8, inject9, I9, inject10, I10, inject11, I11, inject12, I12
);

impl<Q: Query> Inject for Group<Q> {
    type State = (
        usize,
        Arc<Vec<(Q::State, Arc<SegmentInner>)>>,
        Arc<WorldInner>,
    );

    fn initialize(world: &World) -> Option<Self::State> {
        Some((0, Arc::new(Vec::new()), world.inner.clone()))
    }

    fn update((index, queries, inner): &mut Self::State) -> Vec<Dependency> {
        // TODO: Ensure that a user cannot persist a 'Group<Q>' outside of the execution of a system.
        // - Otherwise, 'Arc::get_mut' will fail...
        let mut dependencies = Vec::new();
        if let Some(queries) = Arc::get_mut(queries) {
            while let Some(segment) = inner.segments.get(*index) {
                if let Some(query) = Q::initialize(&segment) {
                    queries.push((query, segment.inner.clone()))
                }
                *index += 1;
            }

            for (query, segment) in queries {
                dependencies.push(Dependency::Read(segment.index, TypeId::of::<Entity>()));
                dependencies.append(&mut Q::update(query));
            }
        }
        dependencies
    }

    fn resolve((_, queries, _): &Self::State) {
        for (query, _) in queries.iter() {
            Q::resolve(query);
        }
    }

    #[inline]
    fn get((_, queries, inner): &Self::State) -> Self {
        Group {
            inner: inner.clone(),
            queries: queries.clone(),
        }
    }
}

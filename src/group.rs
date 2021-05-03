use crate::system::*;
use crate::world::*;
use crate::*;
use std::sync::Arc;

pub struct Group<Q: Query> {
    queries: Arc<Vec<(Q::State, Arc<Store<Entity>>)>>,
}
pub struct GroupIterator<Q: Query> {
    segment: usize,
    index: usize,
    group: Group<Q>,
}

impl<Q: Query> Group<Q> {
    #[inline]
    pub fn each<O>(&self, each: impl Fn(Q) -> O) {
        for (query, store) in self.queries.iter() {
            for i in 0..unsafe { store.count() } {
                each(Q::query(i, query));
            }
        }
    }
}

impl<Q: Query> Clone for Group<Q> {
    fn clone(&self) -> Self {
        Group {
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
        while let Some((query, store)) = self.group.queries.get(self.segment) {
            if self.index < unsafe { store.count() } {
                let query = Q::query(self.index, query);
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

impl<Q: Query> Inject for Group<Q> {
    type State = (
        usize,
        Vec<Dependency>,
        Arc<Vec<(Q::State, Arc<Store<Entity>>)>>,
    );

    fn initialize(_: &mut World) -> Option<Self::State> {
        Some((0, Vec::new(), Arc::new(Vec::new())))
    }

    fn update(
        (index, dependencies, queries): &mut Self::State,
        world: &mut World,
    ) -> Vec<Dependency> {
        // TODO: Ensure that a user cannot persist a 'Group<Q>' outside of the execution of a system.
        // - Otherwise, 'Arc::get_mut' will fail...
        if let Some(queries) = Arc::get_mut(queries) {
            while let Some(segment) = world.segments.get(*index) {
                if let Some(mut pair) = Q::initialize(&segment) {
                    queries.push((pair.0, segment.entities.clone()));
                    dependencies.append(&mut pair.1);
                }
                *index += 1;
            }
        }
        dependencies.clone()
    }

    fn resolve(_: &Self::State, _: &mut World) {}

    #[inline]
    fn inject((_, _, queries): &Self::State, _: &World) -> Self {
        Group {
            queries: queries.clone(),
        }
    }
}

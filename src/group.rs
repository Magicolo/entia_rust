use crate::inject::*;
use crate::system::*;
use crate::world::*;
use crate::*;
use std::sync::Arc;

pub struct Group<'a, Q: Query<'a>> {
    index: usize,
    queries: Arc<Vec<(Q::State, &'a Store<Entity>)>>,
}
pub struct GroupIterator<'a, Q: Query<'a>> {
    segment: usize,
    index: usize,
    group: Group<'a, Q>,
}

impl<'a, Q: Query<'a>> Group<'a, Q> {
    #[inline]
    pub fn each<O>(&self, each: impl Fn(Q) -> O) {
        for (query, store) in self.queries.iter() {
            for i in 0..unsafe { store.count() } {
                each(Q::query(i, query));
            }
        }
    }
}

impl<'a, Q: Query<'a>> Clone for Group<'a, Q> {
    fn clone(&self) -> Self {
        Group {
            index: self.index,
            queries: self.queries.clone(),
        }
    }
}

impl<'a, Q: Query<'a>> IntoIterator for Group<'a, Q> {
    type Item = Q;
    type IntoIter = GroupIterator<'a, Q>;

    fn into_iter(self) -> Self::IntoIter {
        GroupIterator {
            segment: 0,
            index: 0,
            group: self,
        }
    }
}

impl<'a, Q: Query<'a>> IntoIterator for &Group<'a, Q> {
    type Item = Q;
    type IntoIter = GroupIterator<'a, Q>;

    fn into_iter(self) -> Self::IntoIter {
        GroupIterator {
            segment: 0,
            index: 0,
            group: self.clone(),
        }
    }
}

impl<'a, Q: Query<'a>> Iterator for GroupIterator<'a, Q> {
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

impl<'a, Q: Query<'a>> Inject<'a> for Group<'a, Q> {
    type State = (usize, Vec<(Q::State, &'a Store<Entity>)>, &'a World);

    fn initialize(world: &'a World) -> Option<Self::State> {
        Some((0, Vec::new(), world))
    }

    fn inject((_, queries, _): &Self::State) -> Self {
        todo!()
    }

    fn update((index, queries, world): &mut Self::State) {
        // TODO: Ensure that a user cannot persist a 'Group<Q>' outside of the execution of a system.
        // - Otherwise, 'Arc::get_mut' will fail...
        while let Some(segment) = world.0.segments.get(*index) {
            if let Some(mut state) = Q::initialize(&segment, world) {
                queries.push((state, segment.entities.as_ref()));
            }
            *index += 1;
        }
    }

    fn dependencies((_, queries, _): &Self::State) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for (state, _) in queries {
            dependencies.append(&mut Q::dependencies(state));
        }
        dependencies
    }
}

use crate::inject::*;
use crate::system::*;
use crate::world::*;
use crate::*;
use std::rc::Rc;

pub struct Group<'a, Q: Query<'a>> {
    queries: Rc<Vec<(Q::State, &'a Segment)>>,
}

pub struct GroupIterator<'a, Q: Query<'a>> {
    index: usize,
    segment: usize,
    group: Group<'a, Q>,
}

impl<'a, Q: Query<'a>> Group<'a, Q> {
    #[inline]
    pub fn each(&self, mut each: impl FnMut(Q)) {
        for (query, segment) in self.queries.iter() {
            for i in 0..segment.count {
                each(Q::query(i, query));
            }
        }
    }
}

impl<'a, Q: Query<'a>> Clone for Group<'a, Q> {
    fn clone(&self) -> Self {
        Group {
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
        while let Some((query, segment)) = self.group.queries.get(self.segment) {
            if self.index < segment.count {
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
    type State = (usize, Rc<Vec<(Q::State, &'a Segment)>>, &'a World);

    fn initialize(world: &'a World) -> Option<Self::State> {
        Some((0, Vec::new().into(), world))
    }

    fn inject((_, queries, _): &Self::State) -> Self {
        Group {
            queries: queries.clone(),
        }
    }

    fn update((index, queries, world): &mut Self::State) {
        if let Some(queries) = Rc::get_mut(queries) {
            while let Some(segment) = world.segments.get(*index) {
                if let Some(state) = Q::initialize(&segment, world) {
                    queries.push((state, &segment));
                }
                *index += 1;
            }
        }
    }

    fn dependencies((_, queries, _): &Self::State) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for (state, _) in queries.iter() {
            dependencies.append(&mut Q::dependencies(state));
        }
        dependencies
    }
}

use crate::{
    entities::{self, Entities},
    entity::Entity,
    inject::{Get, Inject},
    item::{At, Item},
    segment::Segment,
    system::Dependency,
    world::World,
};
use std::any::TypeId;

pub struct Query<'a, I: Item> {
    states: &'a Vec<(I::State, usize)>,
    entities: Entities<'a>,
    world: &'a World,
}

pub struct State<I: Item> {
    index: usize,
    states: Vec<(I::State, usize)>,
    entities: entities::State,
    filter: Filter,
}

pub struct Iterator<'a, 'b, I: Item> {
    index: usize,
    segment: usize,
    query: &'b Query<'a, I>,
}

pub struct Filter(fn(&Segment) -> bool);

impl Default for Filter {
    fn default() -> Self {
        Self::with::<Entity>()
    }
}

impl Filter {
    pub const TRUE: Self = Self(|_| true);
    pub const FALSE: Self = Self(|_| false);

    #[inline]
    pub fn new(filter: fn(&Segment) -> bool) -> Self {
        Self(filter)
    }

    #[inline]
    pub fn with<T: Send + 'static>() -> Self {
        Self::new(|segment| segment.static_store::<T>().is_some())
    }

    #[inline]
    pub fn filter(&self, segment: &Segment) -> bool {
        self.0(segment)
    }
}

impl<'a, I: Item> Query<'a, I> {
    pub fn each<F: FnMut(<I::State as At<'a>>::Item)>(&self, mut each: F) {
        for (item, segment) in self.states.iter() {
            let segment = &self.world.segments[*segment];
            for i in 0..segment.count {
                each(item.at(i));
            }
        }
    }

    pub fn get(&self, entity: Entity) -> Option<<I::State as At<'a>>::Item> {
        self.entities.get_datum(entity).and_then(|datum| {
            let index = datum.index as usize;
            let segment = datum.segment as usize;
            for pair in self.states {
                if pair.1 == segment {
                    return Some(pair.0.at(index));
                }
            }
            None
        })
    }
}

impl<'a, 'b, I: Item> IntoIterator for &'b Query<'a, I> {
    type Item = <I::State as At<'a>>::Item;
    type IntoIter = Iterator<'a, 'b, I>;

    fn into_iter(self) -> Self::IntoIter {
        Iterator {
            segment: 0,
            index: 0,
            query: self,
        }
    }
}

impl<'a, 'b, I: Item> std::iter::Iterator for Iterator<'a, 'b, I> {
    type Item = <I::State as At<'a>>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((item, segment)) = self.query.states.get(self.segment) {
            let segment = &self.query.world.segments[*segment];
            if self.index < segment.count {
                let item = item.at(self.index);
                self.index += 1;
                return Some(item);
            } else {
                self.segment += 1;
                self.index = 0;
            }
        }
        None
    }
}

impl<'a, I: Item + 'static> Inject for Query<'a, I> {
    type Input = Filter;
    type State = State<I>;

    fn initialize(input: Self::Input, world: &mut World) -> Option<Self::State> {
        <Entities as Inject>::initialize((), world).map(|state| State {
            index: 0,
            states: Vec::new(),
            entities: state,
            filter: input,
        })
    }

    fn update(state: &mut Self::State, world: &mut World) {
        while let Some(segment) = world.segments.get(state.index) {
            state.index += 1;

            if state.filter.filter(segment) {
                if let Some(item) = I::initialize(&segment) {
                    state.states.push((item, segment.index));
                }
            }
        }
    }

    fn depend(state: &Self::State, world: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for (item, segment) in state.states.iter() {
            dependencies.push(Dependency::Read(*segment, TypeId::of::<Entity>()));
            dependencies.append(&mut I::depend(item, world));
        }
        dependencies
    }
}

impl<'a, I: Item + 'static> Get<'a> for State<I> {
    type Item = Query<'a, I>;

    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Query {
            states: &self.states,
            entities: self.entities.get(world),
            world,
        }
    }
}

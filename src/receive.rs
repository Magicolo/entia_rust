use std::{any::TypeId, collections::VecDeque, sync::Arc};

use crate::{
    inject::{Get, Inject},
    message::{Message, Messages},
    system::Dependency,
    world::{Store, World},
};

pub struct Receive<'a, M: Message>(&'a mut Messages<M>);
pub struct State<M: Message> {
    index: usize,
    store: Arc<Store<Messages<M>>>,
    segment: usize,
}

impl<M: Message> Iterator for Receive<'_, M> {
    type Item = M;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.messages.pop_front()
    }
}

impl<M: Message> Inject for Receive<'_, M> {
    type Input = usize;
    type State = State<M>;

    fn initialize(input: Self::Input, world: &mut World) -> Option<Self::State> {
        let meta = world.get_or_add_meta::<M>();
        let segment = world.add_segment_from_metas(&[meta], 8);
        let store = segment.static_store()?;
        let index = segment.reserve(1);
        *unsafe { store.at(index) } = Messages {
            messages: VecDeque::new(),
            capacity: input,
        };
        Some(State {
            index,
            store,
            segment: segment.index,
        })
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Write(state.segment, TypeId::of::<M>())]
    }
}

impl<'a, M: Message> Get<'a> for State<M> {
    type Item = Receive<'a, M>;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        Receive(unsafe { self.store.at(self.index) })
    }
}

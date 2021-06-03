use std::{any::TypeId, collections::VecDeque, marker::PhantomData, sync::Arc};

use crate::{
    depend::{Depend, Dependency},
    inject::{Get, Inject},
    message::{Message, Messages},
    segment::Store,
    world::World,
};

pub struct Receive<'a, M: Message>(&'a mut Messages<M>);
pub struct State<M: Message> {
    index: usize,
    store: Arc<Store>,
    segment: usize,
    _marker: PhantomData<M>,
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
        let segment = world.add_segment_from_metas(vec![meta.clone()], 8);
        let store = segment.store(&meta)?;
        let index = segment.reserve(1);
        let messages = Messages::<M> {
            messages: VecDeque::new(),
            capacity: input,
        };
        unsafe { store.set(index, &[messages]) };
        Some(State {
            index,
            store,
            segment: segment.index,
            _marker: PhantomData,
        })
    }
}

impl<'a, M: Message> Get<'a> for State<M> {
    type Item = Receive<'a, M>;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        Receive(unsafe { self.store.at(self.index) })
    }
}

impl<M: Message> Depend for State<M> {
    fn depend(&self, _: &World) -> Vec<Dependency> {
        vec![Dependency::Write(self.segment, TypeId::of::<M>())]
    }
}

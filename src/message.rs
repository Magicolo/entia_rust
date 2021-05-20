use crate::inject::*;
use crate::query::*;
use crate::system::*;
use crate::world::*;
use crate::write::*;
use std::any::TypeId;
use std::collections::VecDeque;
use std::sync::Arc;

pub trait Message: Clone + Send + 'static {}
impl<T: Clone + Send + 'static> Message for T {}

pub struct Emit<'a, M: Message>(Query<'a, Write<Messages<M>>>);
pub struct EmitState<M: Message>(QueryState<Write<Messages<M>>>);
pub struct Receive<'a, M: Message>(&'a mut Messages<M>);
pub struct ReceiveState<M: Message> {
    index: usize,
    store: Arc<Store<Messages<M>>>,
    segment: usize,
}
struct Messages<M: Message> {
    messages: VecDeque<M>,
    capacity: usize,
}

/*
- Allow for entity-less segments for messages?
- If the emitter adds the message to its own segment, then receivers can all read from it without requiring 'Clone' from the message type.
- This means that receivers will read from all segments with the message type.
- This also removes the need for a queue and if adding a component to a segment can be made thread-safe, this becomes thread-safe.
- Technically, this would mean that emits and receives could all happen at the same time.
- The emitter that owns the segment will be responsible to reset the count to 0 at the beginning of its execution.
- This works because there is 1 segment per emitter, which means there may be more than 1 segment with the same 'Meta' profile.
*/

impl<M: Message> Emit<'_, M> {
    pub fn emit(&mut self, message: M) {
        self.0.each(|messages| {
            if messages.capacity > 0 {
                while messages.messages.len() >= messages.capacity {
                    messages.messages.pop_front();
                }
            }

            messages.messages.push_back(message.clone());
        });
    }
}

impl<'a, M: Message> Inject for Emit<'a, M> {
    type Input = ();
    type State = EmitState<M>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        <Query<'a, Write<Messages<M>>> as Inject>::initialize(Filter::TRUE, world)
            .map(|state| EmitState(state))
    }

    #[inline]
    fn update(EmitState(state): &mut Self::State, world: &mut World) {
        <Query<'a, Write<Messages<M>>> as Inject>::update(state, world);
    }

    #[inline]
    fn resolve(EmitState(state): &mut Self::State, world: &mut World) {
        <Query<'a, Write<Messages<M>>> as Inject>::resolve(state, world);
    }

    fn depend(EmitState(state): &Self::State, world: &World) -> Vec<Dependency> {
        <Query<'a, Write<Messages<M>>> as Inject>::depend(state, world)
    }
}

impl<'a, M: Message> Get<'a> for EmitState<M> {
    type Item = Emit<'a, M>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Emit(self.0.get(world))
    }
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
    type State = ReceiveState<M>;

    fn initialize(input: Self::Input, world: &mut World) -> Option<Self::State> {
        let meta = world.get_or_add_meta::<M>();
        let segment = world.add_segment_from_metas(&[meta], 8);
        let store = segment.static_store()?;
        let index = segment.reserve();
        *unsafe { store.at(index) } = Messages {
            messages: VecDeque::new(),
            capacity: input,
        };
        Some(ReceiveState {
            index,
            store,
            segment: segment.index,
        })
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Write(state.segment, TypeId::of::<M>())]
    }
}

impl<'a, M: Message> Get<'a> for ReceiveState<M> {
    type Item = Receive<'a, M>;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        Receive(unsafe { self.store.at(self.index) })
    }
}

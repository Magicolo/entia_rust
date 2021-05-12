use crate::inject::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub trait Message: Clone + Send + 'static {}
pub struct Emit<'a, M: Message>(&'a Vec<(Arc<Store<M>>, Arc<Segment>)>);
pub struct EmitState<M: Message>(usize, Vec<(Arc<Store<M>>, Arc<Segment>)>);
pub struct Receive<'a, M: Message>(usize, usize, &'a Store<M>, &'a Segment);
pub struct ReceiveState<M: Message>(Arc<Store<M>>, Arc<Segment>);

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
        for (store, segment) in self.0 {
            let count = segment.count.fetch_add(1, Ordering::Relaxed);
            let index = count - 1;
            segment.ensure(count);
            *unsafe { store.at(index) } = message.clone();
        }
    }
}

impl<M: Message> Inject for Emit<'_, M> {
    type State = EmitState<M>;

    fn initialize(_: &mut World) -> Option<Self::State> {
        Some(EmitState(0, Vec::new()))
    }

    fn update(state: &mut Self::State, world: &mut World) {
        while let Some(segment) = world.segments.get(state.0) {
            state.0 += 1;

            if segment.stores.len() == 1 {
                if let Some(store) = segment.store() {
                    state.1.push((store, segment.clone()));
                }
            }
        }
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for (_, segment) in state.1.iter() {
            dependencies.push(Dependency::Write(segment.index, TypeId::of::<M>()));
        }
        dependencies
    }
}

impl<'a, M: Message> Get<'a> for EmitState<M> {
    type Item = Emit<'a, M>;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        Emit(&self.1)
    }
}

impl<M: Message> Iterator for Receive<'_, M> {
    type Item = M;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.0;
        if index < self.1 {
            self.0 += 1;
            Some(unsafe { std::ptr::read(self.2.at(index) as *const M) })
        } else {
            None
        }
    }
}

impl<M: Message> Drop for Receive<'_, M> {
    fn drop(&mut self) {
        let index = self.0;
        let count = self.3.count.fetch_sub(index, Ordering::Relaxed);
        if count > 0 {
            unsafe {
                let source = self.2.at(index);
                let target = self.2.at(0);
                if index < count {
                    std::ptr::copy_nonoverlapping(source as *const M, target as *mut M, count);
                } else {
                    std::ptr::copy(source as *const M, target as *mut M, count);
                }
            }
        }
    }
}

impl<M: Message> Inject for Receive<'_, M> {
    type State = ReceiveState<M>;

    fn initialize(world: &mut World) -> Option<Self::State> {
        let meta = world.get_or_add_meta::<M>();
        let segment = world.add_segment(&[meta], 8);
        Some(ReceiveState(segment.store()?, segment))
    }

    fn depend(state: &Self::State, _: &World) -> Vec<Dependency> {
        vec![Dependency::Write(state.1.index, TypeId::of::<M>())]
    }
}

impl<'a, M: Message> Get<'a> for ReceiveState<M> {
    type Item = Receive<'a, M>;

    #[inline]
    fn get(&'a mut self, _: &World) -> Self::Item {
        Receive(0, self.1.count.load(Ordering::Relaxed), &self.0, &self.1)
    }
}

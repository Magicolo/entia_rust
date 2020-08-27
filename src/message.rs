use crate::component::Metadata;
use crate::*;
use crossbeam_queue::SegQueue;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

pub fn next_index() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

pub trait Message {
    fn index() -> usize;
}

pub struct Receiver<T: Message> {
    queue: SegQueue<T>,
}

impl<T: Message> Component for Receiver<T> {
    fn metadata() -> &'static Metadata {
        todo!()
    }
}

impl World {
    pub fn emit_message<T: Message + Copy>(&self, _: T) {
        todo!("Iterate over all entities that have the 'Receiver<T>' component and enqueue the message.")
    }

    pub fn create_receiver<T: Message>(&self) -> Entity {
        todo!("Create an entity and add the 'Receiver<T>' component to it.")
    }

    pub fn get_receiver<T: Message>(&self, _: Entity) -> &Receiver<T> {
        todo!()
    }
}

impl<T: Message> Receiver<T> {
    #[inline]
    pub fn receive(&self) -> Option<T> {
        self.queue.pop().ok()
    }
}

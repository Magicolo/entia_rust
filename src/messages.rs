use crate::*;
use crossbeam_queue::SegQueue;
use std::rc::{Rc, Weak};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

pub fn next_index() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

pub trait Message: Copy {
    fn index() -> usize;
}

pub struct Emitter<T: Message> {
    queues: Vec<Weak<SegQueue<T>>>,
}

pub struct Receiver<T: Message> {
    queue: Rc<SegQueue<T>>,
}

pub trait Messages {
    fn emit<T: Message + 'static>(&self, message: T) {
        if let Some(emitter) = self.try_emitter() {
            emitter.emit(message);
        }
    }

    fn receiver<T: Message + 'static>(&mut self) -> Receiver<T> {
        self.emitter().receiver()
    }

    fn try_emitter<T: Message + 'static>(&self) -> Option<&Emitter<T>>;
    fn emitter<T: Message + 'static>(&mut self) -> &mut Emitter<T>;
}

impl<T: Message> Emitter<T> {
    pub fn receiver(&mut self) -> Receiver<T> {
        let queue = Rc::new(SegQueue::new());
        self.queues.push(Rc::downgrade(&queue));
        Receiver { queue }
    }

    pub fn emit(&self, message: T) {
        for queue in self.queues.iter().filter_map(|queue| queue.upgrade()) {
            queue.push(message);
        }
    }
}

impl<T: Message> Receiver<T> {
    #[inline]
    pub fn receive(&self) -> Option<T> {
        self.queue.pop().ok()
    }
}

impl Messages for World {
    fn try_emitter<T: Message + 'static>(&self) -> Option<&Emitter<T>> {
        self.emitters
            .get(T::index())
            .and_then(|emitter| emitter.as_ref())
            .and_then(|emitter| emitter.downcast_ref::<Emitter<T>>())
    }

    fn emitter<T: Message + 'static>(&mut self) -> &mut Emitter<T> {
        let index = T::index();
        while self.emitters.len() <= index {
            self.emitters.push(None);
        }

        self.emitters[index]
            .get_or_insert_with(|| Box::new(Emitter::<T> { queues: Vec::new() }))
            .downcast_mut::<Emitter<T>>()
            .unwrap()
    }
}

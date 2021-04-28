use crate::component::Segment;
use crate::entity::Data;
use std::any::Any;
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct Inner {
    pub data: Vec<Data>,
    pub free_indices: Vec<u32>,
    pub frozen_indices: Vec<u32>,
    pub segments: Vec<Segment>,
}

#[derive(Clone)]
pub struct World {
    pub(crate) inner: Arc<UnsafeCell<Inner>>,
    pub(crate) states: HashMap<TypeId, Rc<dyn Any>>,
}

pub trait Module {
    fn new(world: &mut World) -> Self;
}

impl World {
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(UnsafeCell::new(Inner {
                data: Vec::new(),
                free_indices: Vec::new(),
                frozen_indices: Vec::new(),
                segments: Vec::new(),
            })),
            states: HashMap::new(),
        }
    }

    pub fn getz<T: 'static + Module>(&mut self) -> Rc<T> {
        let key = TypeId::of::<T>();
        match self.states.get(&key) {
            Some(state) => state.clone().downcast::<T>().unwrap(),
            None => {
                let value = Rc::new(T::new(self));
                self.states.insert(key, value.clone());
                value
            }
        }
    }

    #[inline]
    pub(crate) unsafe fn get(&self) -> &mut Inner {
        &mut *self.inner.get()
    }
}

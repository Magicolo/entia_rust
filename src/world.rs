use crate::components::Segment;
use crate::entities::Data;
use std::any::Any;

pub trait Downcast {
    fn cast<T: 'static>(&self) -> Option<&T>;
}

#[derive(Default)]
pub struct World {
    pub(crate) entities: Vec<Data>,
    pub(crate) free_indices: Vec<u32>,
    pub(crate) frozen_indices: Vec<u32>,
    pub(crate) segments: Vec<Segment>,
    pub(crate) emitters: Vec<Option<Box<dyn Any>>>,
}

impl World {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn resolve(&mut self) {
        while let Some(index) = self.frozen_indices.pop() {
            self.free_indices.push(index);
        }
    }
}

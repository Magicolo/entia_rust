use crate::*;

pub trait Resource {}

impl World {
    pub fn get_resource<T: Resource>(&self) -> Option<&mut T> {
        todo!()
    }

    pub fn set_resource<T: Resource>(&self, _: T) -> bool {
        todo!()
    }

    pub fn has_resource<T: Resource>(&self) -> bool {
        todo!()
    }
}

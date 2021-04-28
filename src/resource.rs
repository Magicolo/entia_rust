use crate::component::Store;
use crate::world::Inner;

pub trait Resource {}

impl Inner {
    pub fn get_resource<T: Resource>(&self) -> Option<&mut T> {
        todo!()
    }

    pub fn set_resource<T: Resource>(&self, _: T) -> bool {
        todo!()
    }

    pub fn has_resource<T: Resource>(&self) -> bool {
        todo!()
    }

    pub fn get_resource_store<R: Resource + 'static>(&self) -> Option<Store<R>> {
        todo!()
    }
}

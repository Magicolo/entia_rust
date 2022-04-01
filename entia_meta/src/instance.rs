use crate::meta::*;
use std::any::Any;

pub trait Instance: 'static {
    fn get_meta(&self) -> &'static Type;
    fn any(self: Box<Self>) -> Box<dyn Any>;
    fn any_ref(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Meta + 'static> Instance for T {
    #[inline]
    fn get_meta(&self) -> &'static Type {
        Self::meta()
    }

    #[inline]
    fn any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn any_ref(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl dyn Instance {
    #[inline]
    pub fn cast<T: 'static>(self: Box<Self>) -> Result<Box<T>, Box<dyn Any>> {
        self.any().downcast()
    }

    #[inline]
    pub fn cast_ref<T: 'static>(&self) -> Option<&T> {
        self.any_ref().downcast_ref()
    }

    #[inline]
    pub fn cast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.any_mut().downcast_mut()
    }
}

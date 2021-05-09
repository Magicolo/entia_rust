use crate::inject::*;
use crate::system::*;
use crate::world::*;
use std::any::TypeId;

pub trait Resource: Default + Send + 'static {}
impl<T: Default + Send + 'static> Resource for T {}

impl<'a, R: Resource> Inject<'a> for &'a R {
    type State = (&'a R, usize);

    fn initialize(world: &'a World) -> Option<Self::State> {
        /*
        let types = [TypeId::of::<R>()];
        match world.get_segment(types) {
            Some(segment) if segment.entities.len() > 0 {
                (segment.store()?, segment.index)
            }
            None => {
                let template = Template::new().add(R::default());
                let entity = world.create_entity(template);
                let (segment, _index) = world.find_segment(entity)?;
                (segment.store()?, segment.index)
            }
        }
        */

        todo!()
    }

    fn inject((store, _): &Self::State) -> Self {
        unsafe { &**store }
    }

    fn dependencies((_, segment): &Self::State) -> Vec<Dependency> {
        vec![Dependency::Read(*segment, TypeId::of::<R>())]
    }
}

impl<'a, R: Resource> Inject<'a> for &'a mut R {
    type State = (&'a Store<R>, usize);

    fn initialize(world: &'a World) -> Option<Self::State> {
        // let segment = world.segment(&[TypeId::of::<R>()])?
        todo!()
    }

    fn inject((store, _): &Self::State) -> Self {
        unsafe { store.at(0) }
    }

    fn dependencies((_, segment): &Self::State) -> Vec<Dependency> {
        vec![Dependency::Read(*segment, TypeId::of::<R>())]
    }
}

// pub struct Read<T>(Arc<Store<T>>, usize);
// pub struct Write<T>(Arc<Store<T>>, usize);

// impl<R: Resource> Clone for Read<R> {
//     fn clone(&self) -> Self {
//         Self(self.0.clone(), self.1)
//     }
// }

// impl<R: Resource> Deref for Read<R> {
//     type Target = R;
//     #[inline]
//     fn deref(&self) -> &R {
//         self.as_ref()
//     }
// }

// impl<R: Resource> AsRef<R> for Read<R> {
//     #[inline]
//     fn as_ref(&self) -> &R {
//         unsafe { self.0.at(self.1) }
//     }
// }

// impl<R: Resource> Clone for Write<R> {
//     fn clone(&self) -> Self {
//         Self(self.0.clone(), self.1)
//     }
// }

// impl<R: Resource> Deref for Write<R> {
//     type Target = R;
//     #[inline]
//     fn deref(&self) -> &R {
//         self.as_ref()
//     }
// }

// impl<R: Resource> DerefMut for Write<R> {
//     #[inline]
//     fn deref_mut(&mut self) -> &mut R {
//         self.as_mut()
//     }
// }

// impl<R: Resource> AsRef<R> for Write<R> {
//     #[inline]
//     fn as_ref(&self) -> &R {
//         unsafe { self.0.at(0) }
//     }
// }

// impl<R: Resource> AsMut<R> for Write<R> {
//     #[inline]
//     fn as_mut(&mut self) -> &mut R {
//         unsafe { self.0.at(0) }
//     }
// }

// impl<R: Resource> Inject for Read<R> {
//     fn initialize(world: &mut World) -> Option<Self> {
//         let segment = world.segment(&[TypeId::of::<R>()]);
//         Some(Read(segment.store()?, segment.index))
//     }

//     fn dependencies(&self, _: &World) -> Vec<Dependency> {
//         vec![Dependency::Write(self.1, TypeId::of::<R>())]
//     }
// }

// impl<R: Resource> Inject for Write<R> {
//     fn initialize(world: &mut World) -> Option<Self> {
//         let segment = world.segment(&[TypeId::of::<R>()]);
//         Some(Write(segment.store()?, segment.index))
//     }

//     fn dependencies(&self, _: &World) -> Vec<Dependency> {
//         vec![Dependency::Write(self.1, TypeId::of::<R>())]
//     }
// }

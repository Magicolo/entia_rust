use std::sync::Arc;

use crate::{
    inject::{Inject, InjectContext},
    read::Read,
    world::{store::Store, World},
    write::Write,
};

pub trait Resource: Default + Send + 'static {
    // TODO: Consider adding 'depend_ref()' and 'depend_mut()' methods with default empty implementations
    // to allow adding additionnal resource-specific dependencies.
}

unsafe impl<R: Resource> Inject for &R {
    type Input = <Read<R> as Inject>::Input;
    type State = <Read<R> as Inject>::State;

    fn initialize(input: Self::Input, context: InjectContext) -> Option<Self::State> {
        <Read<R> as Inject>::initialize(input, context)
    }
}

unsafe impl<R: Resource> Inject for &mut R {
    type Input = <Write<R> as Inject>::Input;
    type State = <Write<R> as Inject>::State;

    fn initialize(input: Self::Input, context: InjectContext) -> Option<Self::State> {
        <Write<R> as Inject>::initialize(input, context)
    }
}

pub(crate) fn initialize<T: Default + 'static>(
    default: Option<T>,
    world: &mut World,
) -> Option<(Arc<Store>, usize)> {
    let meta = world.get_or_add_meta::<T>();
    let segment = world.get_or_add_segment_by_metas(&[meta.clone()]);
    let store = segment.store(&meta)?;
    if segment.count == 0 {
        let (index, _) = segment.reserve(1);
        segment.resolve();
        unsafe { store.set(index, default.unwrap_or_default()) };
    }
    Some((store, segment.index))
}

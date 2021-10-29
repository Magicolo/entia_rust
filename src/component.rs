use crate::{
    query::{
        filter::Filter,
        item::{Context, Item},
    },
    read::Read,
    world::{segment::Segment, World},
    write::Write,
};

pub trait Component: Sync + Send + 'static {}

impl<C: Component> Filter for C {
    fn filter(segment: &Segment, world: &World) -> bool {
        if let Some(meta) = world.get_meta::<C>() {
            segment.has(&meta)
        } else {
            false
        }
    }
}

unsafe impl<T: Sync + Send + 'static> Item for &T {
    type State = <Read<T> as Item>::State;

    fn initialize(context: Context) -> Option<Self::State> {
        <Read<T> as Item>::initialize(context)
    }
}

unsafe impl<T: Sync + Send + 'static> Item for &mut T {
    type State = <Write<T> as Item>::State;

    fn initialize(context: Context) -> Option<Self::State> {
        <Write<T> as Item>::initialize(context)
    }
}

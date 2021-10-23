use crate::{
    query::{
        filter::Filter,
        item::{Item, ItemContext},
    },
    read::Read,
    world::{segment::Segment, World},
    write::Write,
};

pub trait Component: Sync + Send + 'static {
    // TODO: Consider adding 'depend_ref()' and 'depend_mut()' methods with default empty implementations
    // to allow adding additionnal component-specific dependencies.
}

impl<C: Component> Filter for C {
    fn filter(segment: &Segment, world: &World) -> bool {
        if let Some(meta) = world.get_meta::<C>() {
            segment.has(&meta)
        } else {
            false
        }
    }
}

unsafe impl<C: Component> Item for &C {
    type State = <Read<C> as Item>::State;

    fn initialize(context: ItemContext) -> Option<Self::State> {
        <Read<C> as Item>::initialize(context)
    }
}

unsafe impl<C: Component> Item for &mut C {
    type State = <Write<C> as Item>::State;

    fn initialize(context: ItemContext) -> Option<Self::State> {
        <Write<C> as Item>::initialize(context)
    }
}

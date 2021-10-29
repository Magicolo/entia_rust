use crate::{
    query::filter::Filter,
    world::{segment::Segment, World},
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

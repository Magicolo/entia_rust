pub mod bit_mask;
pub mod component;
pub mod dependency;
pub mod entity;
pub mod inject;
pub mod message;
pub mod query;
pub mod resource;
pub mod system;
pub mod system2;
pub mod system3;
pub mod system4;
pub mod system5;
pub mod world;
pub use component::Component;
pub use entity::Entity;
pub use inject::Inject;
pub use message::Message;
pub use query::Query;
pub use resource::Resource;
pub use system::System;
pub use world::World;

#[cfg(test)]
mod tests {
    use super::component::*;
    use super::*;
    use ctor::ctor;
    use std::sync::Once;

    // #[test]
    // fn create_two_entities() {
    //     let world = World::new();
    //     let entity1 = unsafe { world.create_entity() };
    //     let entity2 = unsafe { world.create_entity() };
    //     assert_ne!(entity1, entity2)
    // }

    #[test]
    fn check_metadata() {
        let metas = Metadata::get_all();
        for (index, meta) in metas.iter().enumerate() {
            assert_eq!(meta.index, index);
        }
        assert_eq!(metas.len(), 2);
    }

    struct Position(f32, f32, f32);
    impl Component for Position {
        // #[inline]
        // fn metadata() -> &'static Metadata {
        //     #[ctor]
        //     static META: Metadata = Metadata::new::<Position>();
        //     &META
        // }
    }

    struct Velocity(f32, f32, f32);
    impl Component for Velocity {
        // #[inline]
        // fn metadata() -> &'static Metadata {
        //     #[ctor]
        //     static META: Metadata = Metadata::new::<Velocity>();
        //     &META
        // }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct OnKill {}
    impl Message for OnKill {
        fn index() -> usize {
            static INITIALIZE: Once = Once::new();
            static mut INDEX: usize = 0;
            unsafe {
                INITIALIZE.call_once(|| INDEX = message::next_index());
                INDEX
            }
        }
    }

    // fn fett(world: &mut World) {
    //     world.get_all_mut::<Position>();
    //     world.get_all_mut::<Velocity>();
    // }

    // fn boba() {
    //     let mut world = World::new();
    //     let emitter1 = world.emitter::<OnKill>();
    //     let emitter2 = world.emitter::<OnKill>();
    //     let receiver = world.receiver::<OnKill>();
    //     world.emit(OnKill {});
    //     emitter1.emit(OnKill {});
    //     emitter2.emit(OnKill {});

    //     while let Some(_) = receiver.receive() {}
    // }
}

pub mod bit_mask;
mod buffer;
pub mod components;
pub mod entities;
pub mod messages;
pub mod world;
pub use components::Component;
pub use components::Components;
pub use entities::Entities;
pub use entities::Entity;
pub use messages::Message;
pub use messages::Messages;
pub use world::World;

#[cfg(test)]
mod tests {
    use super::components::*;
    use super::*;
    use ctor::ctor;
    use std::sync::Once;

    #[test]
    fn create_two_entities() {
        let mut world = World::new();
        let entity1 = world.create();
        let entity2 = world.create();
        assert_ne!(entity1, entity2)
    }

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
        #[inline]
        fn metadata() -> &'static Metadata {
            #[ctor]
            static META: Metadata = Metadata::new::<Position>();
            &META
        }
    }

    struct Velocity(f32, f32, f32);
    impl Component for Velocity {
        #[inline]
        fn metadata() -> &'static Metadata {
            #[ctor]
            static META: Metadata = Metadata::new::<Velocity>();
            &META
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct OnKill {}
    impl Message for OnKill {
        fn index() -> usize {
            static INITIALIZE: Once = Once::new();
            static mut INDEX: usize = 0;
            unsafe {
                INITIALIZE.call_once(|| INDEX = messages::next_index());
                INDEX
            }
        }
    }

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

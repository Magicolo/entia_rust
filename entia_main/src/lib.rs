pub mod create;
pub mod defer;
pub mod depend;
pub mod destroy;
pub mod duplicate;
pub mod entities;
pub mod entity;
pub mod error;
pub mod families;
pub mod family;
pub mod ignore;
pub mod inject;
pub mod local;
pub mod message;
pub mod query;
pub mod read;
pub mod system;
pub mod template;
pub mod world;
pub mod write;

pub mod core {
    pub use entia_core::*;
}

/*
{
    metas: {
        { $i: 0 }: { name: "Time", ... },
        { $i: 1 }: { name: "Position", ... },
        { $i: 2 }: { name: "Mass", ... },
    },
    resources: {
        { $r: 0 }: { delta: 0.1, frames: 100 },
    },
    segments: {
        [{ $r: 1 }, { $r: 2 }]: {
            { $i: 3 }: [{ x: 1, y: 2, z: 3 }, { mass: 100 }],
            { $i: 4 }: [{ x: 5, y: 6, z: 7 }, { mass: 80 }],
            { $i: 5 }: [{ x: 8, y: 9, z: 0 }, { mass: 0 }],
        }
    }
}
*/

pub use crate::{
    create::Create,
    defer::Defer,
    destroy::Destroy,
    entity::Entity,
    families::{adopt::Adopt, reject::Reject, Families},
    family::Family,
    ignore::Ignore,
    inject::{Inject, Injector},
    message::{emit::Emit, receive::Receive, Message},
    query::{
        filter::{Filter, Has, Not},
        Query,
    },
    system::{runner::Runner, schedule::Scheduler, IntoSystem, System},
    template::{Add, LeafTemplate, Spawn, SpawnTemplate, StaticTemplate, Template, With},
    world::{Component, Resource, World},
};
pub(crate) use entia_macro::recurse_16 as recurse;
pub use entia_main_derive::{Component, Depend, Filter, Message, Resource, Template};

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

pub use crate::{
    create::Create,
    defer::Defer,
    destroy::Destroy,
    duplicate::Duplicate,
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
pub use entia_derive::{Component, Depend, Filter, Inject, Message, Resource, Template};
pub(crate) use entia_macro::recurse_16 as recurse;

#[cfg(test)]
mod tests;

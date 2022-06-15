pub mod add;
pub mod component;
pub mod create;
pub mod defer;
pub mod depend;
pub mod destroy;
pub mod entities;
pub mod entity;
pub mod error;
pub mod families;
pub mod family;
pub mod filter;
pub mod ignore;
pub mod inject;
pub mod item;
pub mod message;
pub mod meta;
pub mod query;
pub mod resource;
pub mod segment;
pub mod store;
pub mod system;
pub mod template;
pub mod world;

pub mod core {
    pub use entia_core::*;
}

pub use crate::{
    component::Component,
    create::Create,
    defer::Defer,
    destroy::Destroy,
    entity::Entity,
    families::{adopt::Adopt, reject::Reject, Families},
    family::Family,
    filter::{Filter, Has, Not},
    ignore::Ignore,
    inject::{Inject, Injector},
    message::{emit::Emit, receive::Receive, Message},
    query::Query,
    resource::Resource,
    system::{runner::Runner, schedule::Scheduler, IntoSystem, System},
    template::{Add, LeafTemplate, Spawn, SpawnTemplate, StaticTemplate, Template, With},
    world::World,
};
pub(crate) use entia_macro::recurse_16 as recurse;
pub use entia_main_derive::{Component, Depend, Filter, Message, Resource, Template};

#[cfg(test)]
mod test;

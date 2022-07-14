#![feature(generic_associated_types)]

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
pub mod output;
pub mod query;
pub mod resource;
pub mod run;
pub mod schedule;
pub mod segment;
pub mod store;
pub mod system;
pub mod template;
pub mod world;

/*
- With the chunks iterators, it could be possible to add chunk operations such as 'Destroy/Add/Remove/Adopt/Reject'.

- When possible, decompose systems into smaller systems to allow more parallelism:
    - 'Query systems' may be divided in 'Chunk systems'.
        - Maybe 'Segment systems' would be a good enough approximation to split a system.
        - These systems must have no other purpose other than iterating a query, therefore their item dependencies do not overlap.

- A smarter scheduler that overlaps more systems and anticipates blockers.
    - Execution of systems should not be broken into 'Blocks' and should be more fluid to allow more overlap.
    - A thread pool with a system queue will most likely be more appropriate than the current 'rayon' implementation.
    - 1. Begin by running all parallel-safe systems.
    - 2. Look for the next blocking system and increase the execution priority of systems with incompatible dependencies.
    - 3. As soon as the next blocking system has become non-blocking, begin its execution.
        - Use a channel to check the block status when a relevant system finishes.
    - 4. Systems with compatible dependencies may continue to execute at the same time as the previously blocking system.
    - 5. Repeat steps [2..].
*/

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
    run::Runner,
    schedule::Scheduler,
    system::{IntoSystem, System},
    template::{Add, LeafTemplate, Spawn, SpawnTemplate, StaticTemplate, Template, With},
    world::World,
};
pub(crate) use entia_macro::recurse_16 as recurse;
pub use entia_main_derive::{Component, Depend, Filter, Message, Resource, Template};

#[cfg(test)]
mod test;

use entia_core::Change;

use crate::inject::InjectContext;
use crate::{depend::Dependency, world::World};
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashSet;
use std::mem::replace;
use std::mem::swap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct Runner {
    pub(crate) identifier: usize,
    pub(crate) blocks: Vec<Vec<System>>,
    pub(crate) segments: usize,
}

pub struct System {
    identifier: usize,
    pub(crate) run: Box<dyn FnMut(&World)>,
    pub(crate) update: Box<dyn FnMut(&mut World)>,
    pub(crate) resolve: Box<dyn FnMut(&mut World)>,
    pub(crate) depend: Box<dyn FnMut(&World) -> Vec<Dependency>>,
}

unsafe impl Send for System {}

impl Runner {
    pub fn new(systems: impl IntoIterator<Item = System>, world: &mut World) -> Option<Self> {
        Some(Self {
            identifier: world.identifier,
            blocks: Self::schedule(systems.into_iter(), world)?,
            segments: world.segments.len(),
        })
    }

    pub fn run(&mut self, world: &mut World) {
        if self.identifier != world.identifier {
            // TODO: do something more useful like return an error?
            todo!();
        }

        if self.segments.change(world.segments.len()) {
            // TODO: rather than calling 'unwrap' return an error if it failed
            // - What to do on next iteration? Add a 'state' field to the runner to indicate failure?
            self.blocks = Self::schedule(self.blocks.drain(..).flatten(), world).unwrap();
        }

        for block in self.blocks.iter_mut() {
            // If segments have been added to the world, this may mean that the dependencies used to schedule the systems
            // are not be up to date, therefore it is not safe to run the systems in parallel.
            if self.segments == world.segments.len() && block.len() > 1 {
                use rayon::prelude::*;
                block.par_iter_mut().for_each(|system| (system.run)(world));
            } else {
                for system in block.iter_mut() {
                    (system.run)(world);
                }
            }

            for system in block.iter_mut() {
                (system.resolve)(world);
            }
        }
    }

    pub(crate) fn schedule(
        systems: impl Iterator<Item = System>,
        world: &mut World,
    ) -> Option<Vec<Vec<System>>> {
        #[derive(Debug, Default)]
        struct State {
            unknown: bool,
            reads: HashSet<(usize, TypeId)>,
            writes: HashSet<(usize, TypeId)>,
            defers: HashSet<(usize, TypeId)>,
        }

        impl State {
            pub fn inner_conflicts(&mut self, dependencies: &Vec<Dependency>) -> bool {
                let Self {
                    unknown,
                    reads,
                    writes,
                    defers,
                } = self;

                for dependency in dependencies {
                    if match *dependency {
                        Dependency::Unknown => {
                            *unknown = true;
                            false
                        }
                        Dependency::Read(segment, store) => {
                            let key = (segment, store);
                            reads.insert(key);
                            writes.contains(&key)
                        }
                        Dependency::Write(segment, store) => {
                            let key = (segment, store);
                            reads.contains(&key) || !writes.insert(key)
                        }
                        Dependency::Defer(segment, store) => {
                            defers.insert((segment, store));
                            false
                        }
                    } {
                        return true;
                    }
                }
                false
            }

            pub fn outer_conflicts(&mut self, dependencies: &Vec<Dependency>) -> bool {
                let Self {
                    unknown,
                    reads,
                    writes,
                    defers,
                } = self;
                if *unknown {
                    return true;
                }

                for dependency in dependencies {
                    if match *dependency {
                        Dependency::Unknown => true,
                        Dependency::Read(segment, store) => {
                            let key = (segment, store);
                            reads.insert(key);
                            defers.contains(&key) || writes.contains(&key)
                        }
                        Dependency::Write(segment, store) => {
                            let key = (segment, store);
                            defers.contains(&key) || reads.contains(&key) || writes.insert(key)
                        }
                        Dependency::Defer(segment, store) => {
                            defers.insert((segment, store));
                            false
                        }
                    } {
                        return true;
                    }
                }
                false
            }

            pub fn clear(&mut self) {
                self.unknown = false;
                self.reads.clear();
                self.writes.clear();
                self.defers.clear();
            }
        }

        let mut sequence = Vec::new();
        let mut parallel = Vec::new();
        let mut inner = State::default();
        let mut outer = State::default();

        for mut system in systems {
            (system.update)(world);
            let dependencies = (system.depend)(world);
            if inner.inner_conflicts(&dependencies) {
                return None;
            } else if outer.outer_conflicts(&dependencies) {
                if parallel.len() > 0 {
                    sequence.push(replace(&mut parallel, Vec::new()));
                }
                swap(&mut inner, &mut outer);
            }

            parallel.push(system);
            inner.clear();
        }

        if parallel.len() > 0 {
            sequence.push(parallel);
        }
        Some(sequence)
    }
}

impl System {
    pub fn reserve() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub unsafe fn new<'a, T: 'static>(
        identifier: Option<usize>,
        state: T,
        run: fn(&'a mut T, &'a World),
        update: fn(&'a mut T, InjectContext<'a>),
        resolve: fn(&'a mut T, InjectContext<'a>),
        depend: fn(&'a mut T, &'a World) -> Vec<Dependency>,
    ) -> Self {
        // SAFETY: Since this crate controls the execution of the system's functions, it can guarantee
        // that they are not run in parallel which would allow for races.

        // SAFETY: The 'new' function is declared as unsafe because the user must guarantee that no reference
        // to the 'World' outlives the call of the function pointers. Normally this could be enforced by Rust but
        // there seem to be a limitation in the expressivity of the type system to be able to express the desired
        // intention.
        let identifier = identifier.unwrap_or_else(Self::reserve);
        let state = Arc::new(UnsafeCell::new(state));
        Self {
            identifier,
            run: {
                let state = state.clone();
                Box::new(move |world| unsafe { run(&mut *state.get(), &*(world as *const _)) })
            },
            update: {
                let state = state.clone();
                Box::new(move |world| unsafe {
                    update(
                        &mut *state.get(),
                        InjectContext::new(identifier, &mut *(world as *mut _)),
                    )
                })
            },
            resolve: {
                let state = state.clone();
                Box::new(move |world| unsafe {
                    resolve(
                        &mut *state.get(),
                        InjectContext::new(identifier, &mut *(world as *mut _)),
                    )
                })
            },
            depend: {
                let state = state.clone();
                Box::new(move |world| unsafe { depend(&mut *state.get(), &*(world as *const _)) })
            },
        }
    }

    #[inline]
    pub const fn identifier(&self) -> usize {
        self.identifier
    }
}

use crate::world::*;
use crate::{core::*, depend::Dependency};
use std::any::TypeId;
use std::cell::UnsafeCell;
use std::collections::HashSet;
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
            for system in block.iter_mut() {
                (system.update)(world);
            }

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

    fn conflicts(
        dependencies: &Vec<Dependency>,
        reads: &mut HashSet<(usize, TypeId)>,
        writes: &mut HashSet<(usize, TypeId)>,
        adds: &mut HashSet<(usize, TypeId)>,
    ) -> bool {
        for dependency in dependencies {
            match dependency {
                Dependency::Unknown => return true,
                &Dependency::Read(segment, store) => {
                    let pair = (segment, store);
                    if adds.contains(&pair) || writes.contains(&pair) {
                        return true;
                    }
                    reads.insert(pair);
                }
                &Dependency::Write(segment, store) => {
                    let pair = (segment, store);
                    if adds.contains(&pair) || reads.contains(&pair) || writes.contains(&pair) {
                        return true;
                    }
                    writes.insert(pair);
                }
                &Dependency::Defer(segment, store) => {
                    let pair = (segment, store);
                    adds.insert(pair);
                }
            }
        }
        false
    }

    pub(crate) fn schedule(
        systems: impl Iterator<Item = System>,
        world: &mut World,
    ) -> Option<Vec<Vec<System>>> {
        let mut pairs = Vec::new();
        let mut sequence = Vec::new();
        let mut parallel = Vec::new();
        let mut reads = HashSet::new();
        let mut writes = HashSet::new();
        let mut adds = HashSet::new();

        for mut system in systems {
            (system.update)(world);
            let dependencies = (system.depend)(world);
            // TODO: Detect inner conflicts.
            // - Detection should differ from 'Self::conflicts' since inner conflicts have slightly different rules:
            //      - ex: 'Unknown' is valid
            // if Self::conflicts(&dependencies, &mut reads, &mut writes, &mut adds) {
            //     return None;
            // }
            reads.clear();
            writes.clear();
            adds.clear();
            pairs.push((system, dependencies));
        }

        for (system, dependencies) in pairs {
            if Self::conflicts(&dependencies, &mut reads, &mut writes, &mut adds) {
                if parallel.len() > 0 {
                    sequence.push(std::mem::replace(&mut parallel, Vec::new()));
                }
                reads.clear();
                writes.clear();
                adds.clear();
            } else {
                parallel.push(system);
            }
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
        pre: fn(&'a mut T, &'a mut World),
        post: fn(&'a mut T, &'a mut World),
        depend: fn(&'a mut T, &'a World) -> Vec<Dependency>,
    ) -> Self {
        // SAFETY: Since this crate controls the execution of the system's functions, it can guarantee
        // that they are not run in parallel which would allow for races.

        // SAFETY: The 'new' function is declared as unsafe because the user must guarantee that no reference
        // to the 'World' outlives the call of the function pointers. Normally this could be enforced by Rust but
        // there seem to be a limitation in the expressivity of the type system to be able to express the desired
        // intention.
        let state = Arc::new(UnsafeCell::new(state));
        Self {
            identifier: identifier.unwrap_or_else(Self::reserve),
            run: {
                let state = state.clone();
                Box::new(move |world| unsafe { run(&mut *state.get(), &*(world as *const _)) })
            },
            update: {
                let state = state.clone();
                Box::new(move |world| unsafe { pre(&mut *state.get(), &mut *(world as *mut _)) })
            },
            resolve: {
                let state = state.clone();
                Box::new(move |world| unsafe { post(&mut *state.get(), &mut *(world as *mut _)) })
            },
            depend: {
                let state = state.clone();
                Box::new(move |world| unsafe { depend(&mut *state.get(), &*(world as *const _)) })
            },
        }
    }

    pub fn depend(dependencies: impl IntoIterator<Item = Dependency>) -> Self {
        unsafe {
            Self::new(
                None,
                dependencies.into_iter().collect::<Vec<_>>(),
                |_, _| {},
                |_, _| {},
                |_, _| {},
                |state, _| state.clone(),
            )
        }
    }

    #[inline]
    pub const fn identifier(&self) -> usize {
        self.identifier
    }
}

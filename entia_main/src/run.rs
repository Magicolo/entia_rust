use crate::{
    depend::{Conflict, Dependency, Scope},
    error::{Error, Result},
    system::System,
    world::World,
};
use entia_core::Change;
use rayon::prelude::*;
use std::{mem::replace, sync::Mutex};

pub struct Runner {
    identifier: usize,
    version: usize,
    systems: Vec<System>,
    blocks: Vec<Block>,
    conflict: Conflict,
    result: Mutex<Result>,
}

#[derive(Default, Clone)]
struct Block {
    runs: Vec<usize>,
    resolves: Vec<usize>,
    dependencies: Vec<Dependency>,
}

impl Runner {
    pub fn new(identifier: usize, systems: impl IntoIterator<Item = System>) -> Self {
        Self {
            identifier,
            version: 0,
            systems: systems.into_iter().collect(),
            blocks: Vec::new(),
            conflict: Conflict::default(),
            result: Mutex::new(Ok(())),
        }
    }

    #[inline]
    pub fn systems(&self) -> &[System] {
        &self.systems
    }

    pub fn update(&mut self, world: &mut World) -> Result {
        if self.identifier != world.identifier() {
            return Err(Error::WrongWorld);
        }

        let mut version = self.version;
        // Updating systems may cause more changes of the 'world.version()'. Loop until the version has stabilized.
        while version.change(world.version()) {
            for system in self.systems.iter_mut() {
                system.update(world)?;
            }
        }

        if self.version != version {
            self.schedule(world)?;
            // Only commit the new version if the scheduling succeeds.
            self.version = version;
        }

        Ok(())
    }

    pub fn run(&mut self, world: &mut World) -> Result {
        self.update(world)?;

        for block in self.blocks.iter_mut() {
            // If the world's version has changed, this may mean that the dependencies used to schedule the systems
            // are not up to date, therefore it is not safe to run the systems in parallel.
            if self.version == world.version() {
                if block.runs.len() > 1 {
                    struct Systems(*mut System);
                    unsafe impl Sync for Systems {}

                    let result = &mut self.result;
                    let systems = Systems(self.systems.as_mut_ptr());
                    block.runs.par_iter().for_each(|&index| {
                        // SAFETY: The indices stored in 'runs' are guaranteed to be unique and ordered. Therefore, there
                        // is no concurrence on the 'run: Fn'.
                        // See 'Runner::schedule'.
                        let Systems(systems) = &systems;
                        let system = unsafe { &mut *systems.add(index) };
                        if let Err(error) = system.run(world) {
                            if let Ok(mut guard) = result.lock() {
                                *guard = Err(error);
                            }
                        }
                    });
                    result
                        .get_mut()
                        .map_or(Err(Error::MutexPoison), |result| replace(result, Ok(())))?;
                } else {
                    for &index in block.runs.iter() {
                        self.systems[index].run(world)?;
                    }
                }
            } else {
                for &index in block.runs.iter() {
                    let system = &mut self.systems[index];
                    system.update(world)?;
                    system.run(world)?;
                }
            }

            for &index in block.resolves.iter() {
                self.systems[index].resolve(world)?;
            }
        }

        Ok(())
    }

    /// Batches the systems in blocks that can be executed in parallel using the dependencies produces by each system
    /// to ensure safety. The indices stored in the blocks are guaranteed to be unique and ordered within each block.
    /// Note that systems may be reordered if their dependencies allow it.
    fn schedule(&mut self, world: &mut World) -> Result {
        fn next(blocks: &mut [Block], conflict: &mut Conflict) -> Result {
            if let Some((head, rest)) = blocks.split_first_mut() {
                conflict
                    .detect(Scope::Inner, &head.dependencies)
                    .map_err(Error::Depend)?;

                for tail in rest.iter_mut() {
                    match conflict.detect(Scope::Outer, &tail.dependencies) {
                        Ok(_) => {
                            head.runs.append(&mut tail.runs);
                            head.dependencies.append(&mut tail.dependencies);
                        }
                        _ => {}
                    }
                }

                conflict.clear();
                next(rest, conflict)?;

                match rest {
                    [tail, ..] if tail.runs.len() == 0 && tail.dependencies.len() == 0 => {
                        head.resolves.append(&mut tail.resolves)
                    }
                    _ => {}
                }
            }

            Ok(())
        }

        self.blocks.clear();
        self.conflict.clear();

        for (index, system) in self.systems.iter_mut().enumerate() {
            self.blocks.push(Block {
                runs: vec![index],
                resolves: vec![index],
                dependencies: system.depend(world),
            });
        }

        next(&mut self.blocks, &mut self.conflict)?;
        self.blocks.retain(|block| {
            block.runs.len() > 0 || block.resolves.len() > 0 || block.dependencies.len() > 0
        });
        Ok(())
    }
}

use crate::{
    depend::Dependency,
    error::{Error, Result},
    run::Runner,
    system::{IntoSystem, System},
    world::World,
};
use entia_core::utility::short_type_name;
use std::any::type_name;

pub struct Scheduler<'a> {
    pub(crate) prefix: String,
    pub(crate) systems: Vec<Result<System>>,
    pub(crate) world: &'a mut World,
}

impl World {
    pub fn scheduler(&mut self) -> Scheduler {
        Scheduler {
            prefix: String::new(),
            systems: Vec::new(),
            world: self,
        }
    }

    pub fn run<I: Default, M, S: for<'a> IntoSystem<'a, M, Input = I>>(
        &mut self,
        system: S,
    ) -> Result {
        self.scheduler().add(system).schedule()?.run(self)
    }

    pub fn run_with<I, M, S: for<'a> IntoSystem<'a, M, Input = I>>(
        &mut self,
        input: I,
        system: S,
    ) -> Result {
        self.scheduler()
            .add_with(input, system)
            .schedule()?
            .run(self)
    }
}

impl<'a> Scheduler<'a> {
    pub fn pipe<F: FnOnce(Self) -> Self>(self, schedule: F) -> Self {
        self.with_prefix::<F, _>(schedule)
    }

    pub fn add<I: Default, M, S: IntoSystem<'a, M, Input = I>>(self, system: S) -> Self {
        self.add_with(I::default(), system)
    }

    pub fn add_with<I, M, S: IntoSystem<'a, M, Input = I>>(self, input: I, system: S) -> Self {
        self.with_prefix::<S, _>(|mut scheduler| {
            let system = system.system(input, scheduler.world).map(|mut system| {
                system.name.insert_str(0, &scheduler.prefix);
                system
            });
            scheduler.systems.push(system);
            scheduler
        })
    }

    pub fn barrier(self) -> Self {
        self.add(unsafe {
            System::new(
                None,
                "barrier".into(),
                (),
                |_, _| Ok(()),
                |_, _| Ok(()),
                |_, _| Ok(()),
                |_, _| vec![Dependency::Unknown],
            )
        })
    }

    pub fn schedule(self) -> Result<Runner> {
        let mut systems = Vec::new();
        let mut errors = Vec::new();

        for system in self.systems {
            match system {
                Ok(system) => systems.push(system),
                Err(error) => errors.push(error),
            }
        }

        Error::All(errors)
            .flatten(true)
            .map_or(Ok(Runner::new(self.world.identifier(), systems)), Err)
    }

    fn with_prefix<T, F: FnOnce(Self) -> Self>(mut self, with: F) -> Self {
        let count = self.prefix.len();
        let prefix = if count == 0 {
            format!("{}::", type_name::<T>())
        } else {
            format!("{}::", short_type_name::<T>())
        };
        self.prefix.push_str(&prefix);
        self = with(self);
        self.prefix.truncate(count);
        self
    }
}

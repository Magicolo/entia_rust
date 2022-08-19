use crate::{
    error::{Error, Result},
    run::Runner,
    system::{IntoSystem, System},
    world::World,
};
use entia_core::utility::short_type_name;
use std::any::type_name;

pub struct Scheduler<'a> {
    prefix: String,
    systems: Vec<Result<System>>,
    world: &'a mut World,
}

impl World {
    pub fn scheduler(&mut self) -> Scheduler {
        Scheduler {
            prefix: String::new(),
            systems: Vec::new(),
            world: self,
        }
    }

    pub fn run<I: Default, M, S: IntoSystem<M, Input = I>>(&mut self, system: S) -> Result {
        self.scheduler().add(system).schedule()?.run(self)
    }

    pub fn run_with<I, M, S: IntoSystem<M, Input = I>>(&mut self, input: I, system: S) -> Result {
        self.scheduler()
            .add_with(input, system)
            .schedule()?
            .run(self)
    }
}

impl Scheduler<'_> {
    pub fn pipe<F: FnOnce(Self) -> Self>(self, schedule: F) -> Self {
        self.with_prefix::<F, _>(schedule)
    }

    pub fn add<M, S: IntoSystem<M>>(self, system: S) -> Self
    where
        S::Input: Default,
    {
        self.add_with(S::Input::default(), system)
    }

    pub fn add_with<M, S: IntoSystem<M>>(self, input: S::Input, system: S) -> Self {
        self.with_prefix::<S, _>(|mut scheduler| {
            let system = system.system(input, scheduler.world).map(|mut system| {
                system.name.insert_str(0, &scheduler.prefix);
                system
            });
            scheduler.systems.push(system);
            scheduler
        })
    }

    pub fn schedule(self) -> Result<Runner> {
        self.schedule_with(0)
    }

    pub fn schedule_with(self, parallelism: usize) -> Result<Runner> {
        let mut schedules = Vec::new();
        let mut errors = Vec::new();

        for schedule in self.systems {
            match schedule {
                Ok(system) => schedules.push(system),
                Err(error) => errors.push(error),
            }
        }

        match Error::All(errors).flatten(true) {
            Some(error) => Err(error),
            None => Runner::new(parallelism, schedules, self.world),
        }
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

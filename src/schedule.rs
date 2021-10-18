use crate::{
    depend::Dependency,
    system::{Error, IntoSystem, Runner, System},
    world::World,
};

pub struct Scheduler<'a> {
    pub(crate) systems: Vec<Result<System, Error>>,
    pub(crate) world: &'a mut World,
}

impl World {
    pub fn scheduler(&mut self) -> Scheduler {
        Scheduler {
            systems: Vec::new(),
            world: self,
        }
    }
}

impl<'a> Scheduler<'a> {
    pub fn pipe(self, schedule: impl FnOnce(Self) -> Self) -> Self {
        schedule(self)
    }

    pub fn schedule<I: Default, M, S: IntoSystem<I, M>>(self, system: S) -> Self {
        self.schedule_with(I::default(), system)
    }

    pub fn schedule_with<I, M, S: IntoSystem<I, M>>(mut self, input: I, system: S) -> Self {
        let system = system.into_system(input, self.world);
        self.systems.push(system);
        self
    }

    pub fn synchronize(self) -> Self {
        self.schedule(vec![Dependency::Unknown])
    }

    pub fn runner(self) -> Result<Runner, Error> {
        let mut systems = Vec::new();
        let mut errors = Vec::new();

        for system in self.systems {
            match system {
                Ok(system) => systems.push(system),
                Err(error) => errors.push(error),
            }
        }

        match Error::All(errors).flatten(true) {
            Some(error) => Err(error),
            None => Runner::new(systems, self.world),
        }
    }
}

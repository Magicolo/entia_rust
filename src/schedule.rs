use entia_core::Call;

use crate::{
    depend::{Depend, Dependency},
    inject::{Get, Inject, Injector},
    system::{Runner, System},
    world::World,
};

pub struct Scheduler<'a> {
    pub(crate) systems: Vec<Option<System>>,
    pub(crate) world: &'a mut World,
}

pub trait Schedule<'a, M = ()> {
    fn schedule(self, scheduler: Scheduler<'a>) -> Scheduler<'a>;
}

impl World {
    pub fn scheduler(&mut self) -> Scheduler {
        Scheduler {
            systems: Vec::new(),
            world: self,
        }
    }
}

impl<'a> Schedule<'a> for System {
    fn schedule(self, scheduler: Scheduler<'a>) -> Scheduler<'a> {
        scheduler.schedule(Some(self))
    }
}

impl<'a> Schedule<'a> for Option<System> {
    fn schedule(self, mut scheduler: Scheduler<'a>) -> Scheduler<'a> {
        scheduler.systems.push(self);
        scheduler
    }
}

impl<'a, F: FnOnce(Scheduler) -> Scheduler> Schedule<'a> for F {
    fn schedule(self, scheduler: Scheduler<'a>) -> Scheduler<'a> {
        self(scheduler)
    }
}

impl<'a, I: Inject, C: Call<I, ()> + Call<<I::State as Get<'a>>::Item, ()> + 'static>
    Schedule<'a, [I; 0]> for C
where
    I::Input: Default,
{
    fn schedule(self, scheduler: Scheduler<'a>) -> Scheduler<'a> {
        <(I::Input, C) as Schedule<'a, [I; 1]>>::schedule((I::Input::default(), self), scheduler)
    }
}

impl<'a, I: Inject, C: Call<I, ()> + Call<<I::State as Get<'a>>::Item, ()> + 'static>
    Schedule<'a, [I; 1]> for (I::Input, C)
{
    fn schedule(self, scheduler: Scheduler<'a>) -> Scheduler<'a> {
        let (input, run) = self;
        let system = I::initialize(input, scheduler.world).map(|state| unsafe {
            System::new(
                (run, state),
                |(run, state), world| run.call(state.get(world)),
                |(_, state), world| I::update(state, world),
                |(_, state), world| I::resolve(state, world),
                |(_, state), world| state.depend(world),
            )
        });
        scheduler.schedule(system)
    }
}

impl<'a> Scheduler<'a> {
    pub fn injector(self) -> Injector<'a> {
        Injector {
            input: (),
            scheduler: self,
        }
    }

    pub fn schedule<M, S: Schedule<'a, M>>(self, schedule: S) -> Self {
        schedule.schedule(self)
    }

    pub fn synchronize(self) -> Self {
        self.schedule(System::depend(vec![Dependency::Unknown]))
    }

    pub fn runner(self) -> Option<Runner> {
        // TODO: return a 'Result<Runner<'a>, Error>'
        let mut systems = Vec::new();
        for system in self.systems {
            let mut system = system?;
            (system.update)(self.world);
            systems.push(system);
        }

        Runner::new(systems, self.world)
    }
}

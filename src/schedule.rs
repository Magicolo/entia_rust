use entia_core::Call;

use crate::{
    depend::{Depend, Dependency},
    inject::{Get, Inject, InjectContext},
    system::{Runner, System},
    world::World,
};

pub struct Scheduler<'a> {
    // TODO: 'Option' should be a 'Result' in order to preserve failures and collect them later.
    pub(crate) systems: Vec<Option<System>>,
    pub(crate) world: &'a mut World,
}

pub trait IntoSystem<M = ()> {
    type Input;
    fn into_system(self, input: Self::Input, world: &mut World) -> Option<System>;
}

impl IntoSystem for System {
    type Input = ();

    fn into_system(self, _: Self::Input, _: &mut World) -> Option<System> {
        Some(self)
    }
}

impl IntoSystem for Vec<Dependency> {
    type Input = ();

    fn into_system(self, _: Self::Input, _: &mut World) -> Option<System> {
        Some(unsafe {
            System::new(
                None,
                self,
                |_, _| {},
                |_, _| {},
                |_, _| {},
                |state, _| state.clone(),
            )
        })
    }
}

impl<'a, I: Inject, C: Call<I> + Call<<I::State as Get<'a>>::Item> + 'static> IntoSystem<[I; 0]>
    for C
{
    type Input = I::Input;

    fn into_system(self, input: Self::Input, world: &mut World) -> Option<System> {
        let identifier = System::reserve();
        let state = I::initialize(input, InjectContext::new(identifier, world))?;
        let system = unsafe {
            System::new(
                Some(identifier),
                (self, state, identifier),
                |(run, state, _), world| run.call(state.get(world)),
                |(_, state, identifier), world| {
                    I::update(state, InjectContext::new(*identifier, world))
                },
                |(_, state, identifier), world| {
                    I::resolve(state, InjectContext::new(*identifier, world))
                },
                |(_, state, _), world| state.depend(world),
            )
        };
        Some(system)
    }
}

impl World {
    pub fn scheduler(&mut self) -> Scheduler {
        Scheduler {
            systems: Vec::new(),
            world: self,
        }
    }

    pub fn run<'a, M, S: IntoSystem<M>>(&'a mut self, system: S) -> Option<()>
    where
        S::Input: Default,
    {
        self.run_with(S::Input::default(), system)
    }

    pub fn run_with<M, S: IntoSystem<M>>(&mut self, input: S::Input, system: S) -> Option<()> {
        let mut runner = self.scheduler().schedule_with(input, system).runner()?;
        runner.run(self);
        Some(())
    }
}

impl<'a> Scheduler<'a> {
    pub fn pipe(self, mut schedule: impl FnMut(Self) -> Self) -> Self {
        schedule(self)
    }

    pub fn schedule<M, S: IntoSystem<M>>(self, system: S) -> Self
    where
        S::Input: Default,
    {
        self.schedule_with(S::Input::default(), system)
    }

    pub fn schedule_with<M, S: IntoSystem<M>>(mut self, input: S::Input, system: S) -> Self {
        let system = system.into_system(input, self.world);
        self.systems.push(system);
        self
    }

    pub fn synchronize(self) -> Self {
        self.schedule(vec![Dependency::Unknown])
    }

    pub fn runner(self) -> Option<Runner> {
        // TODO: return a 'Result<Runner<'a>, Error>'
        let mut systems = Vec::new();
        for system in self.systems {
            systems.push(system?);
        }

        Runner::new(systems, self.world)
    }
}

use entia_core::Call;

use crate::{
    depend::{Depend, Dependency},
    inject::{Context, Get, Inject},
    system::{Runner, System},
    world::World,
};

pub struct Scheduler<'a> {
    pub(crate) systems: Vec<Option<System>>,
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
    pub fn pipe(self, mut schedule: impl FnMut(Self) -> Self) -> Self {
        schedule(self)
    }

    pub fn schedule<I: Inject>(
        self,
        run: impl Call<I> + Call<<I::State as Get<'a>>::Item> + 'static,
    ) -> Self
    where
        I::Input: Default,
    {
        self.add::<I, _>(I::Input::default(), run)
    }

    pub fn schedule_with<I: Inject>(
        self,
        input: I::Input,
        run: impl Call<I> + Call<<I::State as Get<'a>>::Item> + 'static,
    ) -> Self {
        self.add::<I, _>(input, run)
    }

    pub fn synchronize(mut self) -> Self {
        let system = unsafe {
            System::new(
                None,
                vec![Dependency::Unknown],
                |_, _| {},
                |_, _| {},
                |_, _| {},
                |state, _| state.clone(),
            )
        };
        self.systems.push(Some(system));
        self
    }

    pub fn runner(self) -> Option<Runner> {
        // TODO: return a 'Result<Runner<'a>, Error>'
        let mut systems = Vec::new();
        for system in self.systems {
            systems.push(system?);
        }

        Runner::new(systems, self.world)
    }

    fn add<I: Inject, R: Call<I> + Call<<I::State as Get<'a>>::Item> + 'static>(
        mut self,
        input: I::Input,
        run: R,
    ) -> Self {
        let context = Context::new(System::reserve());
        let system = I::initialize(input, &context, self.world).map(|state| unsafe {
            System::new(
                Some(context.identifier),
                (run, state),
                |(run, state), world| run.call(state.get(world)),
                |(_, state), world| I::update(state, world),
                |(_, state), world| I::resolve(state, world),
                |(_, state), world| state.depend(world),
            )
        });
        self.systems.push(system);
        self
    }
}

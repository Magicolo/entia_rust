use std::array::IntoIter;

use entia_core::Call;

use crate::{
    depend::{Depend, Dependency},
    inject::{Get, Inject, InjectContext},
    system::{Runner, System},
    world::World,
};

pub struct Scheduler<'a> {
    pub(crate) systems: Vec<Option<System>>,
    pub(crate) world: &'a mut World,
}

// trait Run<I> {
//     fn run(&mut self, input: I);
// }

// impl<'a, I: Inject, C: Call<I> + Call<<I::State as Get<'a>>::Item>> Run<I> for C {
//     fn run(&mut self, input: I) {
//         self.call(input);
//     }
// }
// type Runz<'a, I> = Call<I> + Call<<I::State as Get<'a>>::Item;

impl World {
    pub fn scheduler(&mut self) -> Scheduler {
        Scheduler {
            systems: Vec::new(),
            world: self,
        }
    }

    pub fn run<'a, I: Inject, R: Call<I> + Call<<I::State as Get<'a>>::Item> + 'static>(
        &'a mut self,
        run: R,
    ) -> Option<()>
    where
        I::Input: Default,
    {
        self.run_with::<I, _>(I::Input::default(), run)
    }

    pub fn run_with<'a, I: Inject, R: Call<I> + Call<<I::State as Get<'a>>::Item> + 'static>(
        &mut self,
        input: I::Input,
        run: R,
    ) -> Option<()> {
        let system = self.system::<I, _>(input, run)?;
        let mut runner = Runner::new(IntoIter::new([system]), self)?;
        runner.run(self);
        Some(())
    }

    fn system<'a, I: Inject, R: Call<I> + Call<<I::State as Get<'a>>::Item> + 'static>(
        &mut self,
        input: I::Input,
        run: R,
    ) -> Option<System> {
        let identifier = System::reserve();
        I::initialize(input, InjectContext::new(identifier, self)).map(|state| unsafe {
            System::new(
                Some(System::reserve()),
                (run, state),
                |(run, state), world| run.call(state.get(world)),
                |(_, state), world| I::update(state, world),
                |(_, state), world| I::resolve(state, world),
                |(_, state), world| state.depend(world),
            )
        })
    }
}

impl<'a> Scheduler<'a> {
    pub fn pipe(self, mut schedule: impl FnMut(Self) -> Self) -> Self {
        schedule(self)
    }

    pub fn schedule<I: Inject, R: Call<I> + Call<<I::State as Get<'a>>::Item> + 'static>(
        self,
        run: R,
    ) -> Self
    where
        I::Input: Default,
    {
        self.schedule_with::<I, _>(I::Input::default(), run)
    }

    pub fn schedule_with<I: Inject, R: Call<I> + Call<<I::State as Get<'a>>::Item> + 'static>(
        mut self,
        input: I::Input,
        run: R,
    ) -> Self {
        self.systems.push(self.world.system::<I, _>(input, run));
        self
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
}

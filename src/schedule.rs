use crate::call::*;
use crate::inject::*;
use crate::system::*;
use crate::world::*;

pub struct Scheduler<'a> {
    pub(crate) systems: Vec<Option<System>>,
    pub(crate) world: &'a mut World,
}

pub trait Schedule<'a, M = ()> {
    type Input;
    fn schedule(self, input: Self::Input, scheduler: Scheduler<'a>) -> Scheduler<'a>;
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
    type Input = ();

    fn schedule(self, _: Self::Input, scheduler: Scheduler<'a>) -> Scheduler<'a> {
        scheduler.add(Some(self))
    }
}

impl<'a, F: FnMut(Injector<'a>) -> Injector<'a>> Schedule<'a> for F {
    type Input = ();

    fn schedule(mut self, _: Self::Input, scheduler: Scheduler<'a>) -> Scheduler<'a> {
        self(scheduler.injector()).scheduler
    }
}

impl<'a, I: Inject, C: Call<I, ()> + Call<<I::State as Get<'a>>::Item, ()> + 'static>
    Schedule<'a, [I; 0]> for C
where
    I::Input: Default,
{
    type Input = ();

    fn schedule(self, _: Self::Input, scheduler: Scheduler<'a>) -> Scheduler<'a> {
        <C as Schedule<'a, [I; 1]>>::schedule(self, I::Input::default(), scheduler)
    }
}

impl<'a, I: Inject, C: Call<I, ()> + Call<<I::State as Get<'a>>::Item, ()> + 'static>
    Schedule<'a, [I; 1]> for C
{
    type Input = I::Input;

    fn schedule(self, input: Self::Input, scheduler: Scheduler<'a>) -> Scheduler<'a> {
        let system = I::initialize(input, scheduler.world).map(|state| unsafe {
            System::new(
                (self, state),
                |(run, state), world| run.call(state.get(world)),
                |(_, state), world| I::update(state, world),
                |(_, state), world| I::resolve(state, world),
                |(_, state), world| I::depend(state, world),
            )
        });
        scheduler.add(system)
    }
}

impl<'a> Scheduler<'a> {
    pub fn add(mut self, system: Option<System>) -> Self {
        self.systems.push(system);
        self
    }

    pub fn schedule<M, S: Schedule<'a, M>>(self, schedule: S) -> Self
    where
        S::Input: Default,
    {
        self.schedule_with(S::Input::default(), schedule)
    }

    pub fn schedule_with<M, S: Schedule<'a, M>>(self, input: S::Input, schedule: S) -> Self {
        schedule.schedule(input, self)
    }

    pub fn injector(self) -> Injector<'a> {
        Injector {
            input: (),
            scheduler: self,
        }
    }

    pub fn pipe<F: FnOnce(Self) -> Self>(self, pipe: F) -> Self {
        pipe(self)
    }

    // pub fn system<M>(mut self, system: impl IntoSystem<'a, M>) -> Self {
    //     self.systems.push(system.system(self.world));
    //     self
    // }

    // pub fn system_with<I, M1, M2, S: IntoSystem<'a, M1>>(mut self, input: I, system: S) -> Self
    // where
    //     (I, S): IntoSystem<'a, M2>,
    // {
    //     self.add((input, system).system(self.world))
    // }

    pub fn synchronize(self) -> Self {
        self.schedule(System::depend(vec![Dependency::Unknown]))
    }

    pub fn runner(self) -> Option<Runner> {
        let mut systems = Vec::new();
        for system in self.systems {
            let mut system = system?;
            (system.update)(self.world);
            systems.push(system);
        }

        // TODO: return a 'Result<Runner<'a>, Error>'
        Runner::new(systems, self.world)
    }
}

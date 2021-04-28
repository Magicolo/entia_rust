use crate::component::Store;
use crate::Resource;
use crate::World;

fn test(world: &World) {
    struct Time;
    impl Resource for Time {}

    let mut runner = Runner::new(|_: &Time| {});

    loop {
        runner.run(world);
    }
}

trait Inject {
    type State;

    fn initialize(world: World) -> Option<Self::State>;
    fn inject(state: &mut Self::State) -> &mut Self;
}

struct Runner<P, S: System<P>> {
    state: Option<S::State>,
    system: S,
}

trait System<P = ()> {
    type State;

    fn initialize(world: World) -> Option<Self::State>;
    fn run(&self, state: &mut Self::State);
}

impl<P, S: System<P>> Runner<P, S> {
    pub fn new(system: S) -> Self {
        Self {
            state: None,
            system,
        }
    }

    pub fn run(&mut self, world: &World) {
        if self.state.is_none() {
            self.state = S::initialize(world.clone());
        }
        if let Some(state) = &mut self.state {
            self.system.run(state);
        }
    }
}

impl<R: Resource + 'static> Inject for R {
    type State = Store<R>;

    fn initialize(world: World) -> Option<Self::State> {
        unsafe { world.get().get_resource_store() }
    }

    fn inject(state: &mut Self::State) -> &mut Self {
        unsafe { &mut state.get()[0] }
    }
}

impl<I: Inject, F: Fn(&I)> System<[(I, ()); 1]> for F {
    type State = I::State;

    fn initialize(world: World) -> Option<Self::State> {
        I::initialize(world)
    }

    fn run(&self, state: &mut Self::State) {
        self(I::inject(state));
    }
}

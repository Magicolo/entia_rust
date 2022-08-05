use crate::{
    depend::{Conflict, Depend, Dependency, Scope},
    error::{Error, Result},
    identify, recurse,
    world::World,
};
use entia_core::{utility::short_type_name, Change};
use std::marker::PhantomData;

pub struct Injector<I: Inject> {
    identifier: usize,
    name: String,
    world: usize,
    version: usize,
    state: I::State,
    dependencies: Vec<Dependency>,
    _marker: PhantomData<I>,
}

pub trait Inject {
    type Input;
    type State: for<'a> Get<'a> + Depend;

    #[inline]
    fn name() -> String {
        short_type_name::<Self>()
    }

    fn initialize(input: Self::Input, identifier: usize, world: &mut World) -> Result<Self::State>;

    #[inline]
    fn update(_: &mut Self::State, _: &mut World) -> Result {
        Ok(())
    }

    #[inline]
    fn resolve(_: &mut Self::State) -> Result {
        Ok(())
    }
}

pub trait Get<'a> {
    type Item;
    unsafe fn get(&'a mut self) -> Self::Item;
}

impl World {
    pub fn injector<I: Inject>(&mut self) -> Result<Injector<I>>
    where
        I::Input: Default,
    {
        self.injector_with(I::Input::default())
    }

    pub fn injector_with<I: Inject>(&mut self, input: I::Input) -> Result<Injector<I>> {
        let identifier = identify();
        let state = I::initialize(input, identifier, self)?;
        self.modify();
        Ok(Injector {
            identifier,
            name: I::name(),
            world: self.identifier(),
            version: 0,
            state,
            dependencies: Vec::new(),
            _marker: PhantomData,
        })
    }
}

impl<I: Inject> Injector<I> {
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn identifier(&self) -> usize {
        self.identifier
    }

    #[inline]
    pub fn version(&self) -> usize {
        self.version
    }

    pub fn update(&mut self, world: &mut World) -> Result<bool> {
        if self.world != world.identifier() {
            return Err(Error::WrongWorld);
        } else if self.version == world.version() {
            return Ok(false);
        }

        let mut version = self.version;
        // 'I::update' may cause more changes of the 'world.version()'. Loop until the version has stabilized.
        for _ in 0..1_000 {
            if version.change(world.version()) {
                I::update(&mut self.state, world)?;
            } else {
                break;
            }
        }

        if version.change(world.version()) {
            return Err(Error::UnstableWorldVersion);
        }

        self.dependencies = self.state.depend();
        Conflict::default()
            .detect(Scope::Inner, &self.dependencies)
            .map_err(Error::Depend)?;

        // Only commit the new version if all updates and dependency analysis succeed.
        self.version = version;
        Ok(true)
    }

    pub fn run<T, R: FnOnce(<I::State as Get<'_>>::Item) -> T>(
        &mut self,
        world: &mut World,
        run: R,
    ) -> Result<T> {
        self.update(world)?;
        let value = run(unsafe { self.state.get() });
        I::resolve(&mut self.state)?;
        Ok(value)
    }
}

impl<I: Inject, const N: usize> Inject for [I; N] {
    type Input = [I::Input; N];
    type State = [I::State; N];

    fn initialize(input: Self::Input, identifier: usize, world: &mut World) -> Result<Self::State> {
        let mut items = [(); N].map(|_| None);
        for (i, input) in input.into_iter().enumerate() {
            items[i] = Some(I::initialize(input, identifier, world)?);
        }
        Ok(items.map(Option::unwrap))
    }

    #[inline]
    fn update(state: &mut Self::State, world: &mut World) -> Result {
        for state in state {
            I::update(state, world)?;
        }
        Ok(())
    }

    #[inline]
    fn resolve(state: &mut Self::State) -> Result {
        for state in state {
            I::resolve(state)?;
        }
        Ok(())
    }
}

impl<'a, T: Get<'a>, const N: usize> Get<'a> for [T; N] {
    type Item = [T::Item; N];

    #[inline]
    unsafe fn get(&'a mut self) -> Self::Item {
        let mut iterator = self.iter_mut();
        [(); N].map(|_| iterator.next().unwrap().get())
    }
}

unsafe impl<T: Depend, const N: usize> Depend for [T; N] {
    fn depend(&self) -> Vec<Dependency> {
        let mut dependencies = Vec::new();
        for item in self {
            dependencies.append(&mut item.depend());
        }
        dependencies
    }
}

impl<T> Inject for PhantomData<T> {
    type Input = <() as Inject>::Input;
    type State = <() as Inject>::State;
    fn initialize(input: Self::Input, identifier: usize, world: &mut World) -> Result<Self::State> {
        <()>::initialize(input, identifier, world)
    }
    fn update(state: &mut Self::State, world: &mut World) -> Result {
        <()>::update(state, world)
    }
    fn resolve(state: &mut Self::State) -> Result {
        <()>::resolve(state)
    }
}

macro_rules! inject {
    ($($p:ident, $t:ident),*) => {
        impl<$($t: Inject,)*> Inject for ($($t,)*) {
            type Input = ($($t::Input,)*);
            type State = ($($t::State,)*);

            fn initialize(($($p,)*): Self::Input, _identifier: usize, _world: &mut World) -> Result<Self::State> {
                Ok(($($t::initialize($p, _identifier, _world)?,)*))
            }

            fn update(($($p,)*): &mut Self::State, _world: &mut World) -> Result {
                $($t::update($p, _world)?;)*
                Ok(())
            }

            #[inline]
            fn resolve(($($p,)*): &mut Self::State) -> Result {
                $($t::resolve($p)?;)*
                Ok(())
            }
        }

        impl<'a, $($t: Get<'a>,)*> Get<'a> for ($($t,)*) {
            type Item = ($($t::Item,)*);

            #[inline]
            unsafe fn get(&'a mut self) -> Self::Item {
                let ($($p,)*) = self;
                ($($p.get(),)*)
            }
        }
    };
}

recurse!(inject);

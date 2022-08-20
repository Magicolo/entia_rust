use crate::{
    depend::Dependency,
    error::{Error, Result},
    identify,
    inject::{Adapt, Cast, Context, Get, Inject},
    output::IntoOutput,
    run::{as_mut, Run},
    world::World,
};
use entia_core::{utility::short_type_name, Call};
use std::{
    any::Any,
    fmt::{self},
    result,
    sync::Arc,
};

pub struct System {
    identifier: usize,
    pub(crate) name: String,
    pub(crate) state: Arc<dyn Any + Send + Sync>,
    schedule: Box<dyn FnMut(&mut dyn Any, &mut World) -> Vec<Run>>,
}

pub trait IntoSystem<M = ()> {
    type Input;
    fn system(self, input: Self::Input, world: &mut World) -> Result<System>;
}

impl System {
    #[inline]
    pub const fn identifier(&self) -> usize {
        self.identifier
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn schedule(&mut self, world: &mut World) -> Vec<Run> {
        let state = as_mut(&mut self.state);
        (self.schedule)(state, world)
    }
}

impl fmt::Debug for System {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(&short_type_name::<Self>())
            .field(&self.name())
            .finish()
    }
}

impl<I, S> IntoSystem for (I, S)
where
    (I, S): Into<System>,
{
    type Input = ();

    fn system(self, _: Self::Input, _: &mut World) -> Result<System> {
        Ok(self.into())
    }
}

impl<M, S: IntoSystem<M>> IntoSystem<(M, S)> for Option<S> {
    type Input = S::Input;

    fn system(self, input: Self::Input, world: &mut World) -> Result<System> {
        match self {
            Some(system) => system.system(input, world),
            None => Err(Error::MissingSystem),
        }
    }
}

impl<M, S: IntoSystem<M>, E: Into<Error>> IntoSystem<(M, S)> for result::Result<S, E> {
    type Input = S::Input;

    fn system(self, input: Self::Input, world: &mut World) -> Result<System> {
        match self {
            Ok(system) => system.system(input, world),
            Err(error) => Err(error.into()),
        }
    }
}

impl<'a, I: Inject, O: IntoOutput, C: Call<I, O> + Send + Sync + 'static> IntoSystem<(I, O, C)>
    for C
where
    I::State: Get<'a, Item = I>,
{
    type Input = I::Input;

    fn system(self, input: I::Input, world: &mut World) -> Result<System> {
        let cast = Cast::<(I::State, C)>::new();
        let map = cast.clone().map(|(state, _)| state);
        let mut schedules = Vec::new();
        let context = Context::new(map.clone(), &mut schedules, world);
        let identifier = context.identifier();
        let state = I::initialize(input, context)?;
        world.modify();

        Ok(System {
            identifier,
            name: short_type_name::<I>(),
            state: Arc::new((state, self)),
            schedule: Box::new(move |state, world| match map.adapt(state) {
                Some(state) => {
                    let mut pre = Vec::new();
                    let mut post = Vec::new();

                    for schedule in schedules.iter_mut() {
                        let runs = schedule(state, world);
                        pre.extend(runs.0);
                        post.extend(runs.1);
                    }

                    let cast = cast.clone();
                    pre.push(Run::new(
                        move |state| match cast.adapt(state) {
                            Some((state, run)) => {
                                let state = unsafe { &mut *(state as *mut I::State) };
                                run.call(unsafe { state.get() }).output()
                            }
                            None => Ok(()),
                        },
                        I::depend(state),
                    ));
                    pre.extend(post);
                    pre
                }
                None => vec![],
            }),
        })
    }
}

pub struct Barrier;

impl IntoSystem for Barrier {
    type Input = ();

    fn system(self, _: Self::Input, _: &mut World) -> Result<System> {
        Ok(System {
            identifier: identify(),
            name: "barrier".into(),
            state: Arc::new(()),
            schedule: Box::new(|_, _| vec![Run::new(|_| Ok(()), [Dependency::Unknown])]),
        })
    }
}

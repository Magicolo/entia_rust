use std::marker::PhantomData;

use crate::{
    defer::{self, Resolve},
    depend::Depend,
    destroy::{Early, Late},
    error::Result,
    item::{At, Context, Item},
    template::LeafTemplate,
};

/*
    |temperature: &Temperature, time: &Time, query: Query<(&mut Cold, Add<Frozen, Early|Late>)>| {
        for (cold, mut frozen) in query.iter() {
            cold.0 = cold.0.lerp(temperature.celcius, time.delta).min(0);
            if cold.0 < -15 {
                frozen.add(Frozen);
            }
        }
    }
    // For each segment of the query, try to create a segment with the added component.
*/

pub struct Add<'a, T: LeafTemplate + 'a, R = Early>(defer::Defer<'a, Inner<T>>, PhantomData<R>);
pub struct State<T, R>(defer::State<Inner<T>>, PhantomData<R>);

struct Inner<T>(PhantomData<T>);
struct Defer<T>(PhantomData<T>);

impl<T: LeafTemplate + 'static, R> Item for Add<'_, T, R>
where
    State<T, R>: Depend,
{
    type State = State<T, R>;

    fn initialize(context: Context) -> Result<Self::State> {
        // T::declare(input, context)
        todo!()
    }
}

impl<'a, T: LeafTemplate + 'a, R> At<'a> for State<T, R> {
    type State = ();
    type Ref = Add<'a, T, R>;
    type Mut = Add<'a, T, R>;

    fn get(&'a self, world: &'a crate::World) -> Self::State {
        todo!()
    }

    fn at(state: &Self::State, index: usize) -> Self::Ref {
        todo!()
    }

    fn at_mut(state: &mut Self::State, index: usize) -> Self::Mut {
        Self::at(state, index)
    }
}

impl<T> Resolve for Inner<T> {
    type Item = Defer<T>;

    fn resolve(
        &mut self,
        items: impl ExactSizeIterator<Item = Self::Item>,
        world: &mut crate::World,
    ) -> crate::error::Result {
        todo!()
    }
}

unsafe impl<T> Depend for State<T, Early> {
    fn depend(&self, world: &crate::World) -> Vec<crate::depend::Dependency> {
        let mut dependencies = self.0.depend(world);
        // TODO: Depend on segments
        dependencies
    }
}

unsafe impl<T> Depend for State<T, Late> {
    fn depend(&self, world: &crate::World) -> Vec<crate::depend::Dependency> {
        self.0.depend(world)
    }
}

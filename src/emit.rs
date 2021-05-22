use crate::{
    inject::{Get, Inject},
    message::{Message, Messages},
    query::{self, Filter, Query},
    system::Dependency,
    world::World,
    write::Write,
};

pub struct Emit<'a, M: Message>(Query<'a, Write<Messages<M>>>);
pub struct State<M: Message>(query::State<Write<Messages<M>>>);

impl<M: Message> Emit<'_, M> {
    pub fn emit(&mut self, message: M) {
        self.0.each(|messages| {
            if messages.capacity > 0 {
                while messages.messages.len() >= messages.capacity {
                    messages.messages.pop_front();
                }
            }

            messages.messages.push_back(message.clone());
        });
    }
}

impl<'a, M: Message> Inject for Emit<'a, M> {
    type Input = ();
    type State = State<M>;

    fn initialize(_: Self::Input, world: &mut World) -> Option<Self::State> {
        <Query<'a, Write<Messages<M>>> as Inject>::initialize(Filter::TRUE, world)
            .map(|state| State(state))
    }

    #[inline]
    fn update(State(state): &mut Self::State, world: &mut World) {
        <Query<'a, Write<Messages<M>>> as Inject>::update(state, world);
    }

    #[inline]
    fn resolve(State(state): &mut Self::State, world: &mut World) {
        <Query<'a, Write<Messages<M>>> as Inject>::resolve(state, world);
    }

    fn depend(State(state): &Self::State, world: &World) -> Vec<Dependency> {
        <Query<'a, Write<Messages<M>>> as Inject>::depend(state, world)
    }
}

impl<'a, M: Message> Get<'a> for State<M> {
    type Item = Emit<'a, M>;

    #[inline]
    fn get(&'a mut self, world: &'a World) -> Self::Item {
        Emit(self.0.get(world))
    }
}

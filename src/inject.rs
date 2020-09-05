use crate::*;

pub trait Inject<'a> {
    type State: 'a;

    fn inject(world: &mut World) -> Option<Self::State>;
    fn get(state: &'a mut Self::State, world: &'a World) -> Self;
}

pub struct Group<'a, Q: Query<'a>> {
    pub(crate) segments: Vec<(usize, Q::State)>,
}

pub struct Components<C: Component> {
    _stores: Vec<*mut C>,
}
impl<C: Component> Components<C> {
    pub fn get(&self, _: Entity) -> Option<&C> {
        todo!()
    }

    pub fn get_mut(&mut self, _: Entity) -> Option<&mut C> {
        todo!()
    }

    pub fn set(&mut self, _: Entity, _: C) -> bool {
        todo!()
    }

    pub fn remove(&mut self, _: Entity) -> bool {
        todo!()
    }
}

pub struct Entities;
impl Entities {
    pub fn has(&self, _: Entity) {}
    pub fn create(&mut self) -> Entity {
        todo!()
    }
    pub fn destroy(&mut self, _: Entity) {}
}

pub struct Defer;
impl Defer {
    pub fn create(&self) -> Entity {
        todo!()
    }
    pub fn destroy(&self) -> Entity {
        todo!()
    }
}

impl<'a> Inject<'a> for &'a Entities {
    type State = ();

    fn inject(_: &mut World) -> Option<Self::State> {
        todo!()
    }
    fn get(_: &mut Self::State, _: &World) -> Self {
        todo!()
    }
}

impl<'a> Inject<'a> for &'a mut Entities {
    type State = Entities;

    fn inject(_: &mut World) -> Option<Self::State> {
        todo!()
    }
    fn get(_: &mut Self::State, _: &World) -> Self {
        todo!()
    }
}

impl<'a> Inject<'a> for &'a Defer {
    type State = ();

    fn inject(_: &mut World) -> Option<Self::State> {
        todo!()
    }
    fn get(_: &mut Self::State, _: &World) -> Self {
        todo!()
    }
}

impl<'a> Inject<'a> for &'a mut Defer {
    type State = ();
    fn inject(_: &mut World) -> Option<Self::State> {
        todo!()
    }
    fn get(_: &mut Self::State, _: &World) -> Self {
        todo!()
    }
}

impl<'a, C: Component> Inject<'a> for &'a Components<C> {
    type State = ();

    fn inject(_: &mut World) -> Option<Self::State> {
        todo!()
    }
    fn get(_: &mut Self::State, _: &World) -> Self {
        todo!()
    }
}

impl<'a, R: Resource> Inject<'a> for &'a R {
    type State = Entity;

    fn inject(_: &mut World) -> Option<Self::State> {
        todo!()
        // returns the resource entity
        // world.resources();
    }

    fn get(_: &mut Self::State, _: &World) -> Self {
        todo!()
        // unsafe { world.get::<R>(*state).unwrap() }
    }
}

impl<'a, R: Resource> Inject<'a> for &'a mut R {
    type State = Entity;

    fn inject(_: &mut World) -> Option<Self::State> {
        todo!()
        // returns the resource entity
        // world.resources();
    }

    fn get(_: &mut Self::State, _: &World) -> Self {
        todo!()
        // unsafe { world.get::<R>(*state).unwrap() }
    }
}

impl<'a, Q: Query<'a>> Inject<'a> for &'a Group<'a, Q> {
    type State = (usize, Group<'a, Q>);

    fn inject(_: &mut World) -> Option<Self::State> {
        let group = Group {
            segments: Vec::new(),
        };
        Some((0, group))
    }

    fn get(state: &'a mut Self::State, world: &World) -> Self {
        let inner = unsafe { world.get() };
        let count = inner.segments.len();
        for i in state.0..count {
            if let Some(query) = Q::query(&inner.segments[i]) {
                state.1.segments.push((i, query));
            }
        }
        state.0 = count;
        &state.1
    }
}

macro_rules! tuples {
    ($m:ident, $p:ident, $s:ident) => {};
    ($m:ident, $p:ident, $s:ident, $($ps:ident, $ss:ident),+) => {
        $m!($p, $s, $($ps, $ss),+);
        tuples!($m, $($ps, $ss),+);
    };
}

macro_rules! inject {
    ($i:ident, $s:ident) => {};
    ($($i:ident, $s:ident),+) => {
        impl<'a, $($i: Inject<'a>),+> Inject<'a> for ($($i),+) {
            type State = ($($i::State),+);

            fn inject(world: &mut World) -> Option<Self::State> {
                match ($($i::inject(world)),+) {
                    ($(Some($s)),+) => Some(($($s),+)),
                    _ => None,
                }
            }

            #[inline(always)]
            fn get(($($s),+): &'a mut Self::State, world: &'a World) -> Self {
                ($($i::get($s, world)),+)
            }
        }
    };
}

tuples!(
    inject, I0, state0, I1, state1, I2, state2, I3, state3, I4, state4, I5, state5, I6, state6, I7,
    state7, I8, state8, I9, state9
);

use crate::*;

pub trait Inject<'a> {
    type State: 'a;

    fn state(world: &mut World) -> Option<Self::State>;
    fn inject(state: &'a mut Self::State, world: &'a World) -> Self;
    fn copy(&self) -> Self;
}

pub struct Group<'a, Q: Query<'a>> {
    pub(crate) segments: Vec<(usize, Q::State)>,
}

pub struct Entities;
// impl Entities {
//     fn has(&self, entity: Entity) {}
//     fn create(&mut self) -> Entity {
//         todo!()
//     }
//     fn destroy(&mut self, entity: Entity) {}
// }
pub struct Defer;
// impl Defer {
//     fn create(&self) -> Entity {
//         todo!()
//     }
//     fn destroy(&self) -> Entity {
//         todo!()
//     }
// }

impl<'a> Inject<'a> for &'a Entities {
    type State = ();

    fn state(_: &mut World) -> Option<Self::State> {
        todo!()
    }
    fn inject(_: &mut Self::State, _: &World) -> Self {
        todo!()
    }
    #[inline(always)]
    fn copy(&self) -> Self {
        todo!()
    }
}

impl<'a> Inject<'a> for &'a mut Entities {
    type State = ();

    fn state(_: &mut World) -> Option<Self::State> {
        todo!()
    }
    fn inject(_: &mut Self::State, _: &World) -> Self {
        todo!()
    }
    #[inline(always)]
    fn copy(&self) -> Self {
        todo!()
    }
}

impl<'a> Inject<'a> for &'a Defer {
    type State = ();

    fn state(_: &mut World) -> Option<Self::State> {
        todo!()
    }
    fn inject(_: &mut Self::State, _: &World) -> Self {
        todo!()
    }
    #[inline(always)]
    fn copy(&self) -> Self {
        todo!()
    }
}

impl<'a> Inject<'a> for &'a mut Defer {
    type State = ();
    fn state(_: &mut World) -> Option<Self::State> {
        todo!()
    }
    fn inject(_: &mut Self::State, _: &World) -> Self {
        todo!()
    }
    #[inline(always)]
    fn copy(&self) -> Self {
        todo!()
    }
}

impl<'a, R: Resource> Inject<'a> for &'a R {
    type State = Entity;

    fn state(_: &mut World) -> Option<Self::State> {
        todo!()
        // returns the resource entity
        // world.resources();
    }

    fn inject(_: &mut Self::State, _: &World) -> Self {
        todo!()
        // unsafe { world.get::<R>(*state).unwrap() }
    }

    #[inline(always)]
    fn copy(&self) -> Self {
        self
    }
}

impl<'a, R: Resource> Inject<'a> for &'a mut R {
    type State = Entity;

    fn state(_: &mut World) -> Option<Self::State> {
        todo!()
        // returns the resource entity
        // world.resources();
    }

    fn inject(_: &mut Self::State, _: &World) -> Self {
        todo!()
        // unsafe { world.get::<R>(*state).unwrap() }
    }

    #[inline(always)]
    fn copy(&self) -> Self {
        unsafe { &mut *(*self as *const R as *mut R) }
    }
}

impl<'a, Q: Query<'a>> Inject<'a> for &'a Group<'a, Q> {
    type State = (usize, Group<'a, Q>);

    fn state(_: &mut World) -> Option<Self::State> {
        Some((
            0,
            Group {
                segments: Vec::new(),
            },
        ))
    }

    fn inject(state: &'a mut Self::State, world: &World) -> Self {
        let count = world.segments.len();
        for i in state.0..count {
            if let Some(query) = Q::state(&world.segments[i]) {
                state.1.segments.push((i, query));
            }
        }
        state.0 = count;
        &state.1
    }

    #[inline(always)]
    fn copy(&self) -> Self {
        self
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
    ($($i:ident, $s:ident),+) => {
        impl<'a, $($i: Inject<'a>),+> Inject<'a> for ($($i),+) {
            type State = ($($i::State),+);

            fn state(world: &mut World) -> Option<Self::State> {
                match ($($i::state(world)),+) {
                    ($(Some($s)),+) => Some(($($s),+)),
                    _ => None,
                }
            }

            #[inline(always)]
            fn inject(($($s),+): &'a mut Self::State, world: &'a World) -> Self {
                ($($i::inject($s, world)),+)
            }

            #[inline(always)]
            fn copy(&self) -> Self {
                let ($($s),+) = self;
                (($($s.copy()),+))
            }
        }
    };
}

tuples!(
    inject, I0, state0, I1, state1, I2, state2, I3, state3, I4, state4, I5, state5, I6, state6, I7,
    state7, I8, state8, I9, state9
);

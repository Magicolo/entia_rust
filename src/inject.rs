use crate::dependency::Dependency;
use crate::*;

pub trait Inject<'a> {
    type State: 'a;

    fn dependencies() -> Vec<Dependency>;
    fn inject(world: World) -> Option<Self::State>;
    fn get(state: &'a mut Self::State) -> Self;
}

pub struct Group<'a, Q: Query<'a>> {
    segments: Vec<(usize, Q::State)>,
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

pub struct Entities(World);
impl Entities {
    pub fn has(&self, entity: Entity) -> bool {
        unsafe { self.0.get() }.get_entity_data(entity).is_some()
    }

    pub fn create(&mut self) -> Entity {
        todo!()
    }
    pub fn destroy(&mut self, _: Entity) {}
}

// pub struct Defer<'a, I: Inject<'a>>;
// impl Defer {
//     pub fn create(&self) -> Entity {
//         todo!()
//     }
//     pub fn destroy(&self, _: Entity) {
//         todo!()
//     }
// }

impl<'a> Inject<'a> for &'a Entities {
    type State = ();

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::read::<Entity>()]
    }

    fn inject(_: World) -> Option<Self::State> {
        todo!()
    }

    #[inline(always)]
    fn get(_: &mut Self::State) -> Self {
        todo!()
    }
}

impl<'a> Inject<'a> for &'a mut Entities {
    type State = Entities;

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::write::<Entity>()]
    }

    fn inject(_: World) -> Option<Self::State> {
        todo!()
    }

    #[inline(always)]
    fn get(_: &mut Self::State) -> Self {
        todo!()
    }
}

impl<'a, C: Component + 'static> Inject<'a> for &'a Components<C> {
    type State = ();

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::read::<Entity>(), Dependency::read::<C>()]
    }

    fn inject(_: World) -> Option<Self::State> {
        todo!()
    }

    #[inline(always)]
    fn get(_: &mut Self::State) -> Self {
        todo!()
    }
}

impl<'a, C: Component + 'static> Inject<'a> for &'a mut Components<C> {
    type State = ();

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::read::<Entity>(), Dependency::write::<C>()]
    }

    fn inject(_: World) -> Option<Self::State> {
        todo!()
    }

    #[inline(always)]
    fn get(_: &mut Self::State) -> Self {
        todo!()
    }
}

impl<'a, R: Resource + 'static> Inject<'a> for &'a R {
    type State = Entity;

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::read::<R>()]
    }

    fn inject(_: World) -> Option<Self::State> {
        todo!()
        // returns the resource entity
        // world.resources();
    }

    #[inline(always)]
    fn get(_: &mut Self::State) -> Self {
        todo!()
        // unsafe { world.get::<R>(*state).unwrap() }
    }
}

impl<'a, R: Resource + 'static> Inject<'a> for &'a mut R {
    type State = Entity;

    fn dependencies() -> Vec<Dependency> {
        vec![Dependency::write::<R>()]
    }

    fn inject(_: World) -> Option<Self::State> {
        todo!()
        // returns the resource entity
        // world.resources();
    }

    #[inline(always)]
    fn get(_: &mut Self::State) -> Self {
        todo!()
        // unsafe { world.get::<R>(*state).unwrap() }
    }
}

impl<'a, Q: Query<'a>> Inject<'a> for &'a Group<'a, Q> {
    type State = (usize, World, Group<'a, Q>);

    fn dependencies() -> Vec<Dependency> {
        Q::dependencies()
    }

    fn inject(world: World) -> Option<Self::State> {
        let group = Group {
            segments: Vec::new(),
        };
        Some((0, world, group))
    }

    #[inline(always)]
    fn get(state: &'a mut Self::State) -> Self {
        let inner = unsafe { state.1.get() };
        let count = inner.segments.len();
        for i in state.0..count {
            if let Some(query) = Q::query(&inner.segments[i]) {
                state.2.segments.push((i, query));
            }
        }
        state.0 = count;
        &state.2
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

            fn dependencies() -> Vec<Dependency> {
                let mut dependencies = Vec::new();
                $(dependencies.append(&mut $i::dependencies());)+
                dependencies
            }

            fn inject(world: World) -> Option<Self::State> {
                match ($($i::inject(world.clone())),+) {
                    ($(Some($s)),+) => Some(($($s),+)),
                    _ => None,
                }
            }

            #[inline(always)]
            fn get(($($s),+): &'a mut Self::State) -> Self {
                ($($i::get($s)),+)
            }
        }
    };
}

tuples!(
    inject, I0, state0, I1, state1, I2, state2, I3, state3, I4, state4, I5, state5, I6, state6, I7,
    state7, I8, state8, I9, state9
);

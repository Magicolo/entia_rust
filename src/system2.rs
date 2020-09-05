use crate::World;
use crossbeam::scope;
use std::any::TypeId;
use std::collections::HashMap;

trait System: Depend {
    type State: Send;

    fn schedule(world: &mut World) -> Option<Self::State>;
    fn run(&self, state: &mut Self::State, world: &World);
}

// struct Runner<S: System>(S::Run, S::State, Vec<Dependency>);

// enum Runner<S: System> {
//     Sequence(Runner<S>, Runner<S>),
//     Parallel(Runner<S>, Runner<S>),
//     System(S, S::State, Dependency),
// }

fn boba() {
    scope(|scope| {
        scope.spawn(|_| {});
    })
    .unwrap();
}

pub enum Dependency {
    Read(TypeId),
    Write(TypeId),
}

impl Dependency {
    pub fn synchronous(dependencies: &[Dependency]) -> bool {
        use Dependency::*;

        let mut map = HashMap::<TypeId, bool>::new();
        for dependency in dependencies {
            match *dependency {
                Read(read) => {
                    if map.insert(read, false).unwrap_or(false) {
                        return true;
                    }
                }
                Write(write) => {
                    if map.insert(write, true).is_some() {
                        return true;
                    }
                }
            }
        }
        return false;
    }
}

pub trait Depend {
    fn dependencies() -> Vec<Dependency>;
}

macro_rules! depend {
    ($($ts: ident),+) => {
        impl<$($ts: Depend),+> Depend for ($($ts),+) {
            #[inline]
            fn dependencies() -> Vec<Dependency> {
                let mut dependencies = Vec::new();
                $(dependencies.append(&mut $ts::dependencies());)+
                dependencies
            }
        }
    };
}

macro_rules! depends {
    ($t: ident) => {};
    ($t: ident, $($ts: ident),+) => {
        depend!($t, $($ts),+);
        depends!($($ts),+);
    };
}

depends!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);

// macro_rules! system {
//     ($($s: ident, $sy: ident, $st: ident),+) => {
//         impl<$($s: System + Sync),+> System for ($($s),+) {
//             type State = ($($s::State),+, bool);

//             fn schedule(world: &mut World) -> Option<Self::State> {
//                 match ($($s::schedule(world)),+) {
//                     ($(Some($st)),+) => Some((
//                         $($st),+,
//                         Dependency::synchronous(&Self::dependencies()),
//                     )),
//                     _ => None,
//                 }
//             }

//             fn run(&self, state: &mut Self::State, world: &World) {
//                 let ($($sy),+) = self;
//                 let ($($st),+, synchronous) = state;
//                 if *synchronous {
//                     $($sy.run($st, world);)+
//                 } else {
//                     scope(|scope| {
//                         $(scope.spawn(|_| $sy.run($st, world));)+
//                     })
//                     .unwrap();
//                 }
//             }
//         }
//     };
// }

// macro_rules! systems {
//     ($s:ident, $sy:ident, $st:ident) => {};
//     ($s:ident, $sy:ident, $st:ident, $($ss:ident, $sys:ident, $sts:ident),+) => {
//         system!($s, $sy, $st, $($ss, $sys, $sts),+);
//         systems!($($ss, $sys, $sts),+);
//     };
// }

// systems!(
//     S0, system0, state0, S1, system1, state1, S2, system2, state2, S3, system3, state3, S4,
//     system4, state4, S5, system5, state5, S6, system6, state6, S7, system7, state7, S8, system8,
//     state8, S9, system9, state9
// );

// impl<S1: System + Sync, S2: System> System for (S1, S2) {
//     type State = (S1::State, S2::State, bool);

//     fn schedule(world: &mut World) -> Option<Self::State> {
//         match (S1::schedule(world), S2::schedule(world)) {
//             (Some(state1), Some(state2)) => Some((
//                 state1,
//                 state2,
//                 Dependency::synchronous(&Self::dependencies()),
//             )),
//             _ => None,
//         }
//     }

//     fn run(&self, state: &mut Self::State, world: &World) {
//         let (system1, system2) = self;
//         let (state1, state2, synchronous) = state;
//         if *synchronous {
//             system1.run(state1, world);
//             system2.run(state2, world);
//         } else {
//             scope(|scope| {
//                 scope.spawn(|_| system1.run(state1, world));
//                 system2.run(state2, world);
//             })
//             .unwrap();
//         }
//     }
// }

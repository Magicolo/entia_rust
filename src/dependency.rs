use std::any::TypeId;
use std::collections::HashMap;

#[derive(Copy, Clone)]
pub enum Dependency {
    Read(TypeId),
    Write(TypeId),
}

pub trait Depend {
    fn dependencies() -> Vec<Dependency>;
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

impl Depend for () {
    #[inline]
    fn dependencies() -> Vec<Dependency> {
        Vec::new()
    }
}

impl<T: Depend> Depend for Option<T> {
    #[inline]
    fn dependencies() -> Vec<Dependency> {
        T::dependencies()
    }
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

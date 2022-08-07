use crate::depend;
use std::{any::TypeId, result};

#[derive(Debug, Clone)]
pub enum Error {
    WrongWorld {
        expected: usize,
        actual: usize,
    },
    WrongSegment,
    WrongState,
    WrongInput,
    MutexPoison,
    MissingSystem,
    MissingStore {
        identifier: TypeId,
        segment: usize,
    },
    MissingMeta {
        identifier: TypeId,
    },
    MissingResource {
        name: &'static str,
        identifier: TypeId,
    },
    MissingClone {
        name: &'static str,
    },
    SegmentIndexOutOfRange {
        index: usize,
        segment: usize,
    },
    SegmentMustBeClonable {
        segment: usize,
    },
    StaticCountMustBeTrue,
    FailedToInitialize {
        entity: u32,
        store: u32,
        segment: u32,
    },
    FailedToUpdate {
        entity: u32,
        store: u32,
        segment: u32,
    },
    FailedToSchedule,
    FailedToRun,
    Depend(depend::Error),
    All(Vec<Error>),
    UnstableWorldVersion,
}

pub type Result<T = ()> = result::Result<T, Error>;

impl Error {
    pub fn all<E: Into<Error>>(errors: impl IntoIterator<Item = E>) -> Self {
        Self::All(errors.into_iter().map(Into::into).collect())
    }

    pub fn merge(self, error: Self) -> Self {
        match (self, error) {
            (Error::All(mut left), Error::All(mut right)) => {
                left.append(&mut right);
                Error::All(left)
            }
            (Error::All(mut left), right) => {
                left.push(right);
                Error::All(left)
            }
            (left, Error::All(mut right)) => {
                right.insert(0, left);
                Error::All(right)
            }
            (left, right) => Error::All(vec![left, right]),
        }
    }

    pub fn flatten(self, recursive: bool) -> Option<Self> {
        fn descend(error: Error, errors: &mut Vec<Error>, recursive: bool) {
            match error {
                Error::All(mut inner) => {
                    if recursive {
                        for error in inner {
                            descend(error, errors, recursive);
                        }
                    } else {
                        errors.append(&mut inner);
                    }
                }
                error => errors.push(error),
            }
        }

        let mut errors = Vec::new();
        descend(self, &mut errors, recursive);

        if errors.len() == 0 {
            None
        } else if errors.len() == 1 {
            Some(errors.into_iter().next().unwrap())
        } else {
            Some(Error::All(errors))
        }
    }
}

error!(Error);

macro_rules! error {
    ($t:ty) => {
        impl std::error::Error for $t {}

        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                <Self as std::fmt::Debug>::fmt(self, f)
            }
        }
    };
    ($t:ty, $e:expr) => {
        $crate::error::error!($t);

        impl Into<$crate::error::Error> for $t {
            #[inline]
            fn into(self) -> $crate::error::Error {
                $e(self)
            }
        }
    };
}

pub(crate) use error;

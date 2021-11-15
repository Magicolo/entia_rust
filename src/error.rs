use crate::entity::Entity;
use std::{
    error,
    fmt::{self, Display},
    result,
};

#[derive(Debug)]
pub enum Error {
    WrongWorld,
    MissingSystem,
    MissingStore(&'static str, usize),
    MissingMeta(&'static str),
    MissingClone(&'static str),
    MissingFamily,
    SegmentIndexOutOfRange(usize, usize),
    InvalidEntity(Entity),
    InvalidResolveState,
    FailedToInject,
    InnerConflict(String, Box<Error>),
    OuterConflict(String, Box<Error>),
    UnknownConflict,
    ReadWriteConflict(String, Option<usize>),
    WriteWriteConflict(String, Option<usize>),
    ReadDeferConflict(String, Option<usize>),
    WriteDeferConflict(String, Option<usize>),
    StaticCountMustBeTrue,
    All(Vec<Error>),
}

pub type Result<T = ()> = result::Result<T, Error>;

impl Error {
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

impl error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

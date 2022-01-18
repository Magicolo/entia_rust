pub mod append;
pub mod bits;
pub mod call;
pub mod change;
pub mod flags;
pub mod marker;
pub mod maybe;
pub mod slice;
pub mod utility;

pub use crate::{
    append::Append,
    bits::Bits,
    call::Call,
    change::Change,
    flags::{Flags, IntoFlags},
    marker::{Indirect, Marker},
    maybe::{Maybe, Wrap},
    slice::Slice,
};
pub(crate) use entia_macro::recurse_16 as recurse;

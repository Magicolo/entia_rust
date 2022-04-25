pub mod enumeration;
pub mod function;
pub mod generic;
pub mod meta;
pub mod module;
pub mod primitive;
pub mod structure;
pub mod r#trait;
pub mod value;

pub use self::{
    enumeration::{Enumeration, Variant},
    function::{Argument, Function, Parameter, Signature},
    generic::Generic,
    meta::{Access, Attribute, Constant, Data, Index, Meta},
    module::Module,
    primitive::Primitive,
    r#trait::Trait,
    structure::{Field, Structure},
    value::Value,
};
pub use entia_meta_macro::{meta, Meta};

// mod examples {
//     use ::std::mem::swap;

//     use super::*;
//     use meta::*;
//     use value::*;

//     #[derive(Clone)]
//     pub struct Boba {
//         a: usize,
//         b: Vec<bool>,
//         c: Fett,
//     }

//     #[derive(Clone)]
//     pub struct Jango(usize, Vec<bool>);
//     #[derive(Clone)]
//     pub struct Jangoz;

//     #[derive(Clone)]
//     pub enum Fett {
//         A(usize),
//         B { b: Vec<bool> },
//         C,
//     }

//     impl Meta for Boba {
//         #[inline]
//         fn meta() -> Type {
//             structure!(pub Boba { priv a[0]: usize, priv b[1]: Vec<bool>, priv c[2]: Fett })
//         }
//     }

//     impl Meta for Jango {
//         #[inline]
//         fn meta() -> Type {
//             structure!(pub Jango(priv 0: usize, priv 1: Vec<bool>))
//         }
//     }

//     impl Meta for Jangoz {
//         #[inline]
//         fn meta() -> Type {
//             structure!(pub Jangoz)
//         }
//     }

//     impl Meta for Fett {
//         fn meta() -> Type {
//             static META: Enumeration = Enumeration {
//                 access: Visibility::Public,
//                 name: "Fett",
//                 size: std::mem::size_of::<Fett>(),
//                 identifier: std::any::TypeId::of::<Fett>,
//                 variant_index: |name| match name {
//                     "A" => Some(0),
//                     "B" => Some(1),
//                     "C" => Some(2),
//                     _ => None,
//                 },
//                 index: |instance| match instance.downcast_ref()? {
//                     Fett::A(..) => Some(0),
//                     Fett::B { .. } => Some(1),
//                     Fett::C => Some(2),
//                 },
//                 generics: &[],
//                 attributes: &[attribute!(derive(Clone))],
//                 variants: &[
//                     Variant {
//                         name: "A",
//                         kind: Structures::Tuple,
//                         values: |instance| match *instance.downcast::<Fett>()? {
//                             Fett::A(a) => Ok([Value::from(a)].into()),
//                             fett => Err(Box::new(fett)),
//                         },
//                         values: |instance| match *instance.downcast::<Fett>() {
//                             Ok(Fett::A(a)) => Ok([Value::from(a)].into()),
//                             Ok(fett) => Err(Box::new(fett)),
//                             Err(instance) => Err(Box::new(instance)),
//                         },
//                         attributes: &[],
//                         fields: &[Field {
//                             access: Visibility::Public,
//                             name: "0",
//                             attributes: &[],
//                             meta: usize::meta,
//                             get: |instance| match instance.downcast_ref()? {
//                                 Fett::A(a) => Some(a),
//                                 _ => None,
//                             },
//                             get_mut: |instance| match instance.downcast_mut()? {
//                                 Fett::A(a) => Some(a),
//                                 _ => None,
//                             },
//                             set: |instance, value| match instance.downcast_mut()? {
//                                 Fett::A(a) => Some(swap(a, value.downcast_mut()?)),
//                                 _ => None,
//                             },
//                         }],
//                         field_index: |name| match name {
//                             "0" => Some(0),
//                             _ => None,
//                         },
//                         new: |values| Some(Box::new(Fett::A(values.next()?.downcast().ok()?))),
//                     },
//                     Variant {
//                         name: "B",
//                         kind: Structures::Map,
//                         attributes: &[],
//                         fields: &[],
//                         values: |instance| match *instance.downcast::<Fett>()? {
//                             Fett::B { b } => Ok([Value::from(b)].into()),
//                             fett => Err(Box::new(fett)),
//                         },
//                         new: |values| {
//                             Some(Box::new(Fett::B {
//                                 b: values.next()?.downcast().ok()?,
//                             }))
//                         },
//                         field_index: |name| match name {
//                             "b" | "0" => Some(0),
//                             _ => None,
//                         },
//                     },
//                     Variant {
//                         name: "C",
//                         kind: Structures::Unit,
//                         attributes: &[],
//                         fields: &[],
//                         values: |instance| match *instance.downcast::<Fett>()? {
//                             Fett::C => Ok([].into()),
//                             fett => Err(Box::new(fett)),
//                         },
//                         new: |_| Some(Box::new(Fett::C)),
//                         field_index: |_| None,
//                     },
//                 ],
//                 functions: &[],
//             };
//             Type::Enumeration(&META)
//         }
//     }
// }

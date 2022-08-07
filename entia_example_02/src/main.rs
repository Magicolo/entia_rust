// #![feature(custom_inner_attributes)]
// #![feature(proc_macro_hygiene)]

// use std::marker::PhantomData;

// use entia::meta::{module::Member, Meta, Module, Structure};

// #[entia::meta::meta]
// pub mod b {
//     pub struct A;
//     pub enum G {
//         A,
//         B(usize, char, ()),
//         C { a: isize, b: bool, c: A },
//     }

//     pub const D: usize = 123;
//     // use super::*;

//     // pub struct C;

//     fn e() {}
//     pub(crate) fn h(a: u8, b: &A, c: &mut G) {}
//     pub(crate) fn i<'a: 'b, 'b>(a: &'a A, b: &'b A) -> &'b A {
//         a
//     }

//     // pub(crate) trait F {}
// }

// fn main() {
//     let a = <b::A as Meta>::meta();
//     let a = &b::META.members[0];
//     let a = match &b::META {
//         Module { members, .. } => match &members[..] {
//             [Member::Structure(structure), b, c] => match structure() {
//                 Structure { name: "A", .. } => true,
//                 _ => false,
//             },
//             _ => false,
//         },
//         _ => false,
//     };
// }

fn main() {}

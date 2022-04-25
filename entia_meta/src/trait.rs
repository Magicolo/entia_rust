use crate::{
    function::Signature,
    generic::Generic,
    meta::{Access, Attribute, Index},
};

pub struct Trait {
    pub access: Access,
    pub name: &'static str,
    pub generics: &'static [Generic],
    pub attributes: &'static [Attribute],
    pub functions: Index<Signature>,
    pub associates: Index<Associate>,
}

#[derive(Debug)]
pub struct Associate {
    pub name: &'static str,
}

use crate::{
    enumeration::Enumeration,
    function::Function,
    meta::{Access, Attribute, Constant, Index, Static},
    r#trait::Trait,
    structure::Structure,
};

pub struct Module {
    pub access: Access,
    pub name: &'static str,
    pub attributes: &'static [Attribute],
    pub members: Index<Member>,
}

pub enum Member {
    Module(&'static Module),
    Constant(Constant),
    Static(Static),
    Function(Function),
    Structure(fn() -> &'static Structure),
    Enumeration(fn() -> &'static Enumeration),
    // Union(Union),
    Trait(Trait),
    // Implementation(Implementation),
}

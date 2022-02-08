pub enum Node {
    Unit,
    Bool(bool),
    Char(char),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    F32(f32),
    F64(f64),
    // Shared(),
    // Exclusive(),
    // Constant(*const ()),
    // Mutable(*mut ()),
    List(List),
    Map(Map),
    Tuple(List),
    Slice(List),
    Array(List),
    Bytes(Vec<u8>),
    String(String),
    Structure(Structure),
    Enumeration(Enumeration),
}

pub struct List(Vec<Node>);
pub struct Map(Vec<(Node, Node)>);

pub enum Structure {
    Unit,
    Tuple(List),
    Map(Map),
}

pub enum Enumeration {
    Never,
    Variant(Structure),
}

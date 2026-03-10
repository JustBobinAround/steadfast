use std::collections::HashMap;
#[derive(Debug)]
pub enum PrimType {
    Bool(bool),
    Char(char),
    F32(f32),
    F64(f64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    String(String),
    None,
}

impl PartialEq for PrimType {
    fn eq(&self, other: &Self) -> bool {
        use PrimType::*;

        match (self, other) {
            (Bool(a), Bool(b)) => a == b,
            (Char(a), Char(b)) => a == b,

            // float comparison via string
            (F32(a), F32(b)) => a.to_string() == b.to_string(),
            (F64(a), F64(b)) => a.to_string() == b.to_string(),

            (I8(a), I8(b)) => a == b,
            (I16(a), I16(b)) => a == b,
            (I32(a), I32(b)) => a == b,
            (I64(a), I64(b)) => a == b,
            (I128(a), I128(b)) => a == b,
            (Isize(a), Isize(b)) => a == b,

            (U8(a), U8(b)) => a == b,
            (U16(a), U16(b)) => a == b,
            (U32(a), U32(b)) => a == b,
            (U64(a), U64(b)) => a == b,
            (U128(a), U128(b)) => a == b,
            (Usize(a), Usize(b)) => a == b,

            (String(a), String(b)) => a == b,
            (None, None) => true,

            _ => false,
        }
    }
}

impl Eq for PrimType {}

#[derive(Debug, PartialEq, Eq)]
pub enum DataHolder {
    Primitive(PrimType),
    Struct(HashMap<String, DataHolder>),
    Array(Vec<DataHolder>),
}

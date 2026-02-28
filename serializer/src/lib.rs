use std::{collections::HashMap, str::FromStr};

#[derive(Debug, PartialEq, Eq)]
pub enum PrimType {
    Bool,
    Char,
    F32,
    F64,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    String,
    None,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DataHolder {
    Primitive { ty: PrimType, val: String },
    Struct(HashMap<String, DataHolder>),
    Array(Vec<DataHolder>),
}

// impl DataHolder {
//     pub fn from_map
// }

pub trait Serialize {
    fn serialize(self) -> DataHolder;
}

macro_rules! impl_primitive_serialize {
    ($name: expr, $t:ty) => {
        impl Serialize for $t {
            fn serialize(self) -> DataHolder {
                DataHolder::Primitive {
                    ty: $name,
                    val: self.to_string(),
                }
            }
        }
        impl Serialize for &$t {
            fn serialize(self) -> DataHolder {
                DataHolder::Primitive {
                    ty: $name,
                    val: self.to_string(),
                }
            }
        }
    };
}

impl_primitive_serialize!(PrimType::Bool, bool);
impl_primitive_serialize!(PrimType::Char, char);
impl_primitive_serialize!(PrimType::F32, f32);
impl_primitive_serialize!(PrimType::F64, f64);
impl_primitive_serialize!(PrimType::I8, i8);
impl_primitive_serialize!(PrimType::I16, i16);
impl_primitive_serialize!(PrimType::I32, i32);
impl_primitive_serialize!(PrimType::I64, i64);
impl_primitive_serialize!(PrimType::I128, i128);
impl_primitive_serialize!(PrimType::Isize, isize);
impl_primitive_serialize!(PrimType::U8, u8);
impl_primitive_serialize!(PrimType::U16, u16);
impl_primitive_serialize!(PrimType::U32, u32);
impl_primitive_serialize!(PrimType::U64, u64);
impl_primitive_serialize!(PrimType::U128, u128);
impl_primitive_serialize!(PrimType::Usize, usize);
impl_primitive_serialize!(PrimType::String, String);

pub trait Deserialize: Sized {
    fn deserialize(dh: DataHolder) -> Result<Self, ()>;
}

macro_rules! impl_primitive_deserialize {
    ($t:ty) => {
        impl Deserialize for $t {
            fn deserialize(dh: DataHolder) -> Result<Self, ()> {
                match dh {
                    DataHolder::Primitive { ty: _, val } => match Self::from_str(&val) {
                        Ok(s) => Ok(s),
                        Err(_) => Err(()),
                    },
                    _ => Err(()),
                }
            }
        }
        impl Deserialize for HashMap<String, $t> {
            fn deserialize(dh: DataHolder) -> Result<Self, ()> {
                match dh {
                    DataHolder::Struct(map) => map
                        .into_iter()
                        .map(|(k, v)| Ok((k, <$t>::deserialize(v)?)))
                        .collect(),
                    _ => Err(()),
                }
            }
        }
    };
}

impl_primitive_deserialize!(bool);
impl_primitive_deserialize!(char);
impl_primitive_deserialize!(f32);
impl_primitive_deserialize!(f64);
impl_primitive_deserialize!(i8);
impl_primitive_deserialize!(i16);
impl_primitive_deserialize!(i32);
impl_primitive_deserialize!(i64);
impl_primitive_deserialize!(i128);
impl_primitive_deserialize!(isize);
impl_primitive_deserialize!(u8);
impl_primitive_deserialize!(u16);
impl_primitive_deserialize!(u32);
impl_primitive_deserialize!(u64);
impl_primitive_deserialize!(u128);
impl_primitive_deserialize!(usize);

impl Deserialize for String {
    fn deserialize(dh: DataHolder) -> Result<Self, ()> {
        match dh {
            DataHolder::Primitive { ty: _, val } => Ok(val),
            _ => Err(()),
        }
    }
}
impl Deserialize for HashMap<String, String> {
    fn deserialize(dh: DataHolder) -> Result<Self, ()> {
        match dh {
            DataHolder::Struct(map) => map
                .into_iter()
                .map(|(k, v)| Ok((k, String::deserialize(v)?)))
                .collect(),
            _ => Err(()),
        }
    }
}

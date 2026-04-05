mod data_holder;

pub use crate::data_holder::{DataHolder, PrimType};
use std::{collections::HashMap, str::FromStr};

// impl DataHolder {
//     pub fn from_map
// }

pub trait Serialize: PartialEq {
    fn serialize(self) -> DataHolder;
}

macro_rules! impl_primitive_serialize {
    ($name: expr, $t:ty) => {
        impl Serialize for $t {
            fn serialize(self) -> DataHolder {
                DataHolder::Primitive($name(self))
            }
        }
        impl Serialize for &$t {
            fn serialize(self) -> DataHolder {
                DataHolder::Primitive($name(self.clone()))
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

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize(self) -> DataHolder {
        DataHolder::Array(self.into_iter().map(|item| item.serialize()).collect())
    }
}

pub trait Deserialize: Sized + PartialEq {
    fn deserialize(dh: DataHolder) -> Result<Self, ()>;
}

macro_rules! impl_primitive_deserialize {
    ($prim_ty: ident, $t:ty) => {
        impl Deserialize for $t {
            fn deserialize(dh: DataHolder) -> Result<Self, ()> {
                match dh {
                    DataHolder::Primitive(ty) => match ty {
                        PrimType::$prim_ty(val) => Ok(val),
                        _ => Err(()),
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

impl_primitive_deserialize!(Bool, bool);
impl_primitive_deserialize!(Char, char);
impl_primitive_deserialize!(F32, f32);
impl_primitive_deserialize!(F64, f64);
impl_primitive_deserialize!(I8, i8);
impl_primitive_deserialize!(I16, i16);
impl_primitive_deserialize!(I32, i32);
impl_primitive_deserialize!(I64, i64);
impl_primitive_deserialize!(I128, i128);
impl_primitive_deserialize!(Isize, isize);
impl_primitive_deserialize!(U8, u8);
impl_primitive_deserialize!(U16, u16);
impl_primitive_deserialize!(U32, u32);
impl_primitive_deserialize!(U64, u64);
impl_primitive_deserialize!(U128, u128);
impl_primitive_deserialize!(Usize, usize);

impl Deserialize for String {
    fn deserialize(dh: DataHolder) -> Result<Self, ()> {
        match dh {
            DataHolder::Primitive(PrimType::String(val)) => Ok(val),
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
impl<T: Deserialize> Deserialize for Vec<T> {
    fn deserialize(dh: DataHolder) -> Result<Self, ()> {
        match dh {
            DataHolder::Array(data) => data.into_iter().map(|item| T::deserialize(item)).collect(),
            _ => Err(()),
        }
    }
}

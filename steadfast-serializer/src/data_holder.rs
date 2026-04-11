use std::collections::BTreeMap;
use steadfast_bytes::{ByteSize, FromBytes, ToBytes};
use steadfast_time::UTC;
use steadfast_uuid::UUID;
fn push_le_bytes<const N: usize, T: ToBytes<[u8; N]>>(val: T, mut bytes: Vec<u8>) -> Vec<u8> {
    for b in val.to_bytes_le() {
        bytes.push(b);
    }
    bytes
}

macro_rules! impl_into_primtype {
    ($ty: ty, $name: ident) => {
        impl From<$ty> for PrimType {
            fn from(other: $ty) -> PrimType {
                PrimType::$name(other)
            }
        }
    };
}

impl_into_primtype!(bool, Bool);
impl_into_primtype!(char, Char);
impl_into_primtype!(f32, F32);
impl_into_primtype!(f64, F64);
impl_into_primtype!(i8, I8);
impl_into_primtype!(i16, I16);
impl_into_primtype!(i32, I32);
impl_into_primtype!(i64, I64);
impl_into_primtype!(i128, I128);
impl_into_primtype!(u8, U8);
impl_into_primtype!(u16, U16);
impl_into_primtype!(u32, U32);
impl_into_primtype!(u64, U64);
impl_into_primtype!(UTC, UTC);
impl_into_primtype!(UUID, UUID);
impl_into_primtype!(u128, U128);
// String(String),

fn from_first_chunk<const N: usize, T: FromBytes<[u8; N]> + ByteSize + Into<PrimType>>(
    bytes: &[u8],
) -> Result<(usize, PrimType), DataHolderErr> {
    match bytes.first_chunk::<N>() {
        Some(chunk) => Ok((T::BYTE_SIZE, <T>::from_bytes_le(*chunk).into())),
        None => Err(DataHolderErr::NotEnoughBytes {
            need: T::BYTE_SIZE,
            found: bytes.len(),
        }),
    }
}
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
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    UTC(UTC),
    UUID(UUID),
    U128(u128),
    String(String),
    None,
}

impl PrimType {
    const fn num_code(&self) -> u8 {
        match self {
            Self::Bool(_) => NumCode::BOOL as u8,
            Self::Char(_) => NumCode::CHAR as u8,
            Self::F32(_) => NumCode::F32 as u8,
            Self::F64(_) => NumCode::F64 as u8,
            Self::I8(_) => NumCode::I8 as u8,
            Self::I16(_) => NumCode::I16 as u8,
            Self::I32(_) => NumCode::I32 as u8,
            Self::I64(_) => NumCode::I64 as u8,
            Self::I128(_) => NumCode::I128 as u8,
            Self::U8(_) => NumCode::U8 as u8,
            Self::U16(_) => NumCode::U16 as u8,
            Self::U32(_) => NumCode::U32 as u8,
            Self::U64(_) => NumCode::U64 as u8,
            Self::U128(_) => NumCode::U128 as u8,
            Self::String(_) => NumCode::STRING as u8,
            Self::UTC(_) => NumCode::UTC as u8,
            Self::UUID(_) => NumCode::UUID as u8,
            Self::None => NumCode::NONE as u8,
        }
    }

    fn to_bytes(self, mut bytes: Vec<u8>) -> Vec<u8> {
        match self {
            Self::Bool(val) => push_le_bytes(val, bytes),
            Self::Char(val) => push_le_bytes(val, bytes),
            Self::F32(val) => push_le_bytes(val, bytes),
            Self::F64(val) => push_le_bytes(val, bytes),
            Self::I8(val) => push_le_bytes(val, bytes),
            Self::I16(val) => push_le_bytes(val, bytes),
            Self::I32(val) => push_le_bytes(val, bytes),
            Self::I64(val) => push_le_bytes(val, bytes),
            Self::I128(val) => push_le_bytes(val, bytes),
            Self::U8(val) => push_le_bytes(val, bytes),
            Self::U16(val) => push_le_bytes(val, bytes),
            Self::U32(val) => push_le_bytes(val, bytes),
            Self::U64(val) => push_le_bytes(val, bytes),
            Self::UTC(val) => push_le_bytes(val, bytes),
            Self::UUID(val) => push_le_bytes(val, bytes),
            Self::U128(val) => push_le_bytes(val, bytes),
            Self::String(val) => {
                let s_len = val.len();
                for b in s_len.to_le_bytes() {
                    bytes.push(b);
                }
                for b in val.into_bytes() {
                    bytes.push(b);
                }
                bytes
            }
            Self::None => bytes,
        }
    }

    fn from_bytes(num_code: u8, bytes: &[u8]) -> Result<(usize, Self), DataHolderErr> {
        if bytes.len() == 0 {
            return Err(DataHolderErr::FoundEmptyBytes);
        }
        match NumCode::from_u8(num_code)? {
            NumCode::NONE => todo!(),
            NumCode::BOOL => from_first_chunk::<1, bool>(bytes),
            NumCode::CHAR => from_first_chunk::<1, char>(bytes),
            NumCode::U8 => from_first_chunk::<1, u8>(bytes),
            NumCode::U16 => from_first_chunk::<2, u16>(bytes),
            NumCode::U32 => from_first_chunk::<4, u32>(bytes),
            NumCode::U64 => from_first_chunk::<8, u64>(bytes),
            NumCode::U128 => from_first_chunk::<16, u128>(bytes),
            NumCode::I8 => from_first_chunk::<1, i8>(bytes),
            NumCode::I16 => from_first_chunk::<2, i16>(bytes),
            NumCode::I32 => from_first_chunk::<4, i32>(bytes),
            NumCode::I64 => from_first_chunk::<8, i64>(bytes),
            NumCode::I128 => from_first_chunk::<16, i128>(bytes),
            NumCode::F32 => from_first_chunk::<4, f32>(bytes),
            NumCode::F64 => from_first_chunk::<8, f64>(bytes),
            NumCode::UTC => from_first_chunk::<8, UTC>(bytes),
            NumCode::UUID => from_first_chunk::<16, UUID>(bytes),
            NumCode::ARRAY => todo!(),
            NumCode::STRING => todo!(),
            NumCode::STRUCT => todo!(),
        }
    }
}

impl PartialEq for PrimType {
    fn eq(&self, other: &Self) -> bool {
        use PrimType::*;

        match (self, other) {
            (Bool(a), Bool(b)) => a == b,
            (Char(a), Char(b)) => a == b,

            (F32(a), F32(b)) => a.to_bytes_le() == b.to_bytes_le(),
            (F64(a), F64(b)) => a.to_bytes_le() == b.to_bytes_le(),

            (I8(a), I8(b)) => a == b,
            (I16(a), I16(b)) => a == b,
            (I32(a), I32(b)) => a == b,
            (I64(a), I64(b)) => a == b,
            (I128(a), I128(b)) => a == b,

            (U8(a), U8(b)) => a == b,
            (U16(a), U16(b)) => a == b,
            (U32(a), U32(b)) => a == b,
            (U64(a), U64(b)) => a == b,
            (U128(a), U128(b)) => a == b,
            (UTC(a), UTC(b)) => a == b,
            (UUID(a), UUID(b)) => a == b,

            (String(a), String(b)) => a == b,
            (None, None) => true,

            _ => false,
        }
    }
}

impl Eq for PrimType {}

pub enum DataHolderErr {
    InvalidNumCode(u8),
    FoundEmptyBytes,
    FieldNotFound,
    NotEnoughBytes { need: usize, found: usize },
    InvalidBoolVal { found: u8 },
    InvalidUTF8,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DataHolder {
    Primitive(PrimType),
    Struct(BTreeMap<String, DataHolder>),
    Array(Vec<DataHolder>),
}

#[repr(u8)]
pub enum NumCode {
    NONE = 0,
    BOOL = 1,
    CHAR = 2,
    U8 = 3,
    U16 = 4,
    U32 = 5,
    U64 = 6,
    U128 = 7,
    I8 = 8,
    I16 = 9,
    I32 = 10,
    I64 = 11,
    I128 = 12,
    F32 = 13,
    F64 = 14,
    UTC = 15,
    UUID = 16,
    ARRAY = 17,
    STRING = 18,
    STRUCT = 19,
}

impl NumCode {
    fn from_u8(code: u8) -> Result<Self, DataHolderErr> {
        match code {
            0 => Ok(NumCode::NONE),
            1 => Ok(NumCode::BOOL),
            2 => Ok(NumCode::CHAR),
            3 => Ok(NumCode::U8),
            4 => Ok(NumCode::U16),
            5 => Ok(NumCode::U32),
            6 => Ok(NumCode::U64),
            7 => Ok(NumCode::U128),
            8 => Ok(NumCode::I8),
            9 => Ok(NumCode::I16),
            10 => Ok(NumCode::I32),
            11 => Ok(NumCode::I64),
            12 => Ok(NumCode::I128),
            13 => Ok(NumCode::F32),
            14 => Ok(NumCode::F64),
            15 => Ok(NumCode::UTC),
            16 => Ok(NumCode::UUID),
            17 => Ok(NumCode::ARRAY),
            18 => Ok(NumCode::STRING),
            19 => Ok(NumCode::STRUCT),
            _ => Err(DataHolderErr::InvalidNumCode(code)),
        }
    }
}

impl DataHolder {
    const fn num_code(&self) -> u8 {
        match self {
            Self::Primitive(ty) => ty.num_code(),
            Self::Struct(_) => NumCode::STRUCT as u8,
            Self::Array(_) => NumCode::ARRAY as u8,
        }
    }

    pub fn to_bytes(self, mut bytes: Vec<u8>) -> Vec<u8> {
        let code = self.num_code();
        match self {
            Self::Primitive(ty) => {
                bytes.push(code);
                ty.to_bytes(bytes)
            }
            Self::Struct(map) => {
                bytes.push(code);
                bytes = push_le_bytes(map.len(), bytes);

                for (key, val) in map {
                    bytes = push_le_bytes(key.len(), bytes);
                    for b in key.into_bytes() {
                        bytes.push(b);
                    }
                    bytes = val.to_bytes(bytes);
                }

                bytes
            }
            Self::Array(a) => {
                bytes.push(code);
                bytes = push_le_bytes(a.len(), bytes);
                for item in a {
                    bytes = item.to_bytes(bytes);
                }
                bytes
            }
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<(usize, DataHolder), DataHolderErr> {
        if bytes.len() == 0 {
            return Err(DataHolderErr::FoundEmptyBytes);
        }
        let num_code = bytes[0];
        if num_code == 0 {
            return Ok((1, DataHolder::Primitive(PrimType::None)));
        }

        if bytes.len() > 1 {
            if num_code < NumCode::ARRAY as u8 {
                let (offset, prim_ty) = PrimType::from_bytes(num_code, &bytes[1..])?;
                Ok((offset + 1, DataHolder::Primitive(prim_ty)))
            } else if num_code == NumCode::STRUCT as u8 {
                let (map_len, _prim_type) = from_first_chunk::<8, u64>(&bytes[1..])?;

                let mut offset = 1;
                let mut map = BTreeMap::new();
                for _ in 0..map_len {
                    let (key_len, _prim_type) = from_first_chunk::<8, u64>(&bytes[offset..])?;

                    offset += 8;
                    if bytes.len() < offset + key_len + 1 {
                        return Err(DataHolderErr::NotEnoughBytes {
                            need: offset + key_len,
                            found: bytes.len(),
                        });
                    }

                    let key = match String::from_utf8(bytes[offset..offset + key_len].to_vec()) {
                        Ok(s) => s,
                        Err(_) => return Err(DataHolderErr::InvalidUTF8),
                    };

                    offset += key_len;
                    let (bytes_read, val) = DataHolder::from_bytes(&bytes[offset..])?;

                    map.insert(key, val);
                    offset += bytes_read;
                }

                Ok((offset, DataHolder::Struct(map)))
            } else if num_code == NumCode::ARRAY as u8 {
                todo!();
            } else {
                Err(DataHolderErr::InvalidNumCode(num_code))
            }
        } else {
            Err(DataHolderErr::FoundEmptyBytes)
        }
    }
}

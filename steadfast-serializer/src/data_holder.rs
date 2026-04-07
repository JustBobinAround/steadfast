use std::collections::BTreeMap;
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

macro_rules! impl_prim_from_bytes {
    ($bytes: ident, $num_code: ident, $cmp_code: expr, $bytes_to_read: literal, $prim_ty: expr, $inner_ty: ty) => {
        if $num_code == DataHolder::NUM_CODE_F32 {
            return match $bytes.first_chunk::<4>() {
                Some(chunk) => Ok((4, PrimType::F32(<f32>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 4,
                    found: $bytes.len(),
                }),
            };
        }
    };
}

impl PrimType {
    const fn num_code(&self) -> u8 {
        match self {
            Self::Bool(_) => DataHolder::NUM_CODE_BOOL,
            Self::Char(_) => DataHolder::NUM_CODE_CHAR,
            Self::F32(_) => DataHolder::NUM_CODE_F32,
            Self::F64(_) => DataHolder::NUM_CODE_F64,
            Self::I8(_) => DataHolder::NUM_CODE_I8,
            Self::I16(_) => DataHolder::NUM_CODE_I16,
            Self::I32(_) => DataHolder::NUM_CODE_I32,
            Self::I64(_) => DataHolder::NUM_CODE_I64,
            Self::I128(_) => DataHolder::NUM_CODE_I128,
            Self::Isize(_) => DataHolder::NUM_CODE_ISIZE,
            Self::U8(_) => DataHolder::NUM_CODE_U8,
            Self::U16(_) => DataHolder::NUM_CODE_U16,
            Self::U32(_) => DataHolder::NUM_CODE_U32,
            Self::U64(_) => DataHolder::NUM_CODE_U64,
            Self::U128(_) => DataHolder::NUM_CODE_U128,
            Self::Usize(_) => DataHolder::NUM_CODE_USIZE,
            Self::String(_) => DataHolder::NUM_CODE_STRING,
            Self::None => DataHolder::NUM_CODE_NONE,
        }
    }

    fn to_bytes(self, bytes: &mut Vec<u8>) {
        match self {
            Self::Bool(val) => {
                if val {
                    bytes.push(1);
                } else {
                    bytes.push(0);
                }
            }
            Self::Char(val) => {
                bytes.push(val as u8);
            }
            Self::F32(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::F64(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::I8(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::I16(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::I32(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::I64(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::I128(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::Isize(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::U8(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::U16(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::U32(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::U64(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::U128(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::Usize(val) => {
                for b in val.to_le_bytes() {
                    bytes.push(b);
                }
            }
            Self::String(val) => {
                let s_len = val.len();
                for b in s_len.to_le_bytes() {
                    bytes.push(b);
                }
                for b in val.into_bytes() {
                    bytes.push(b);
                }
            }
            Self::None => {}
        }
    }

    fn from_bytes(num_code: u8, bytes: &[u8]) -> Result<(usize, Self), DataHolderErr> {
        if bytes.len() == 0 {
            return Err(DataHolderErr::FoundEmptyBytes);
        }
        if num_code == DataHolder::NUM_CODE_BOOL {
            if bytes[0] == 1 {
                Ok((1, PrimType::Bool(true)))
            } else if bytes[0] == 0 {
                Ok((1, PrimType::Bool(false)))
            } else {
                Err(DataHolderErr::InvalidBoolVal { found: bytes[0] })
            }
        } else if num_code == DataHolder::NUM_CODE_CHAR {
            Ok((1, PrimType::Char(bytes[0] as char)))
        } else if num_code == DataHolder::NUM_CODE_F32 {
            match bytes.first_chunk::<4>() {
                Some(chunk) => Ok((4, PrimType::F32(<f32>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 4,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_F64 {
            match bytes.first_chunk::<8>() {
                Some(chunk) => Ok((8, PrimType::F64(<f64>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 4,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_I8 {
            Ok((1, PrimType::I8(bytes[0] as i8)))
        } else if num_code == DataHolder::NUM_CODE_I16 {
            match bytes.first_chunk::<2>() {
                Some(chunk) => Ok((2, PrimType::I16(<i16>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 2,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_I32 {
            match bytes.first_chunk::<4>() {
                Some(chunk) => Ok((4, PrimType::I32(<i32>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 4,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_I64 {
            match bytes.first_chunk::<8>() {
                Some(chunk) => Ok((8, PrimType::I64(<i64>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 8,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_I128 {
            match bytes.first_chunk::<16>() {
                Some(chunk) => Ok((16, PrimType::I128(<i128>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 16,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_ISIZE {
            match bytes.first_chunk::<8>() {
                Some(chunk) => Ok((8, PrimType::Isize(<isize>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 8,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_U8 {
            Ok((1, PrimType::U8(bytes[0])))
        } else if num_code == DataHolder::NUM_CODE_U16 {
            match bytes.first_chunk::<2>() {
                Some(chunk) => Ok((2, PrimType::U16(<u16>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 2,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_U32 {
            match bytes.first_chunk::<4>() {
                Some(chunk) => Ok((4, PrimType::U32(<u32>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 4,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_U64 {
            match bytes.first_chunk::<8>() {
                Some(chunk) => Ok((8, PrimType::U64(<u64>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 8,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_U128 {
            match bytes.first_chunk::<16>() {
                Some(chunk) => Ok((16, PrimType::U128(<u128>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 16,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_USIZE {
            match bytes.first_chunk::<8>() {
                Some(chunk) => Ok((8, PrimType::Usize(<usize>::from_le_bytes(*chunk)))),
                None => Err(DataHolderErr::NotEnoughBytes {
                    need: 8,
                    found: bytes.len(),
                }),
            }
        } else if num_code == DataHolder::NUM_CODE_STRING {
            let s_len = match bytes.first_chunk::<8>() {
                Some(chunk) => <usize>::from_le_bytes(*chunk),
                None => {
                    return Err(DataHolderErr::NotEnoughBytes {
                        need: 8,
                        found: bytes.len(),
                    });
                }
            };

            let total_offset = 8 + s_len;

            if bytes.len() < total_offset {
                return Err(DataHolderErr::NotEnoughBytes {
                    need: total_offset,
                    found: bytes.len(),
                });
            } else {
                match String::from_utf8(bytes[8..total_offset].to_vec()) {
                    Ok(s) => Ok((total_offset, PrimType::String(s))),
                    Err(_) => Err(DataHolderErr::InvalidUTF8),
                }
            }
        } else {
            unreachable!("Invalid prim type codes should be handled in actual data holder impl");
        }
    }
}

impl PartialEq for PrimType {
    fn eq(&self, other: &Self) -> bool {
        use PrimType::*;

        match (self, other) {
            (Bool(a), Bool(b)) => a == b,
            (Char(a), Char(b)) => a == b,

            (F32(a), F32(b)) => a.to_le_bytes() == b.to_le_bytes(),
            (F64(a), F64(b)) => a.to_le_bytes() == b.to_le_bytes(),

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

impl DataHolder {
    pub const NUM_CODE_BOOL: u8 = 1;
    pub const NUM_CODE_CHAR: u8 = 4;
    pub const NUM_CODE_F32: u8 = 0;
    pub const NUM_CODE_F64: u8 = 0;
    pub const NUM_CODE_I8: u8 = 3;
    pub const NUM_CODE_I16: u8 = 6;
    pub const NUM_CODE_I32: u8 = 8;
    pub const NUM_CODE_I64: u8 = 10;
    pub const NUM_CODE_I128: u8 = 12;
    pub const NUM_CODE_ISIZE: u8 = 14;
    pub const NUM_CODE_U8: u8 = 2;
    pub const NUM_CODE_U16: u8 = 5;
    pub const NUM_CODE_U32: u8 = 7;
    pub const NUM_CODE_U64: u8 = 9;
    pub const NUM_CODE_U128: u8 = 11;
    pub const NUM_CODE_USIZE: u8 = 13;
    pub const NUM_CODE_STRING: u8 = 15;
    pub const NUM_CODE_NONE: u8 = 0;
    pub const NUM_CODE_STRUCT: u8 = 17;
    pub const NUM_CODE_ARRAY: u8 = 16;

    const fn num_code(&self) -> u8 {
        match self {
            Self::Primitive(ty) => ty.num_code(),
            Self::Struct(_) => Self::NUM_CODE_STRUCT,
            Self::Array(_) => Self::NUM_CODE_ARRAY,
        }
    }

    pub fn to_bytes(self, bytes: &mut Vec<u8>) {
        let code = self.num_code();
        match self {
            Self::Primitive(ty) => {
                bytes.push(code);
                ty.to_bytes(bytes);
            }
            Self::Struct(map) => {
                bytes.push(code);
                let map_len = map.len().to_le_bytes();
                for b in map_len {
                    bytes.push(b);
                }

                for (key, val) in map {
                    let key_len = key.len();
                    for b in key_len.to_le_bytes() {
                        bytes.push(b);
                    }
                    for b in key.into_bytes() {
                        bytes.push(b);
                    }
                    val.to_bytes(bytes);
                }
            }
            Self::Array(a) => {
                bytes.push(code);
                for b in a.len().to_le_bytes() {
                    bytes.push(b);
                }
                for item in a {
                    item.to_bytes(bytes)
                }
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
            if num_code < Self::NUM_CODE_ARRAY {
                let (offset, prim_ty) = PrimType::from_bytes(num_code, &bytes[1..])?;
                Ok((offset + 1, DataHolder::Primitive(prim_ty)))
            } else if num_code == Self::NUM_CODE_STRUCT {
                let map_len = match bytes[1..].first_chunk::<8>() {
                    Some(chunk) => <usize>::from_le_bytes(*chunk),
                    None => {
                        return Err(DataHolderErr::NotEnoughBytes {
                            need: 8,
                            found: bytes.len(),
                        });
                    }
                };

                let mut offset = 1;
                let mut map = BTreeMap::new();
                for _ in 0..map_len {
                    let key_len = match bytes[offset..].first_chunk::<8>() {
                        Some(chunk) => <usize>::from_le_bytes(*chunk),
                        None => {
                            return Err(DataHolderErr::NotEnoughBytes {
                                need: 8,
                                found: bytes.len(),
                            });
                        }
                    };

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
            } else if num_code == Self::NUM_CODE_ARRAY {
                todo!();
            } else {
                Err(DataHolderErr::InvalidNumCode(num_code))
            }
        } else {
            Err(DataHolderErr::FoundEmptyBytes)
        }
    }
}

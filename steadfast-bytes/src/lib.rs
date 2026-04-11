use std::{
    cmp::Ord,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    hash::Hash,
};
#[derive(Debug)]
pub enum BytesErr {
    NotEnoughBytes { need: usize, found: usize },
    IoError(std::io::Error),
    FromUtf8Error(std::string::FromUtf8Error),
    UnexpectedTypeCode { expected: TypeCode, found: TypeCode },
}

impl From<std::io::Error> for BytesErr {
    fn from(other: std::io::Error) -> Self {
        BytesErr::IoError(other)
    }
}
impl From<std::string::FromUtf8Error> for BytesErr {
    fn from(other: std::string::FromUtf8Error) -> Self {
        BytesErr::FromUtf8Error(other)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TypeCode {
    None,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    CHAR,
    USIZE,
    ISIZE,
    BOOL,

    Extension(u8),

    ResultOk,
    ResultErr,
    Some,
    DynSize,
}

impl TypeCode {
    pub const fn as_u8(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::U8 => 1,
            Self::U16 => 2,
            Self::U32 => 3,
            Self::U64 => 4,
            Self::U128 => 5,
            Self::I8 => 6,
            Self::I16 => 7,
            Self::I32 => 8,
            Self::I64 => 9,
            Self::I128 => 10,
            Self::F32 => 11,
            Self::F64 => 12,
            Self::CHAR => 13,
            Self::USIZE => 14,
            Self::ISIZE => 15,
            Self::BOOL => 16,

            Self::Extension(u) => *u,

            Self::ResultOk => u8::MAX - 3,
            Self::ResultErr => u8::MAX - 2,
            Self::Some => u8::MAX - 1,
            Self::DynSize => u8::MAX,
        }
    }
    pub const fn from_u8(num: u8) -> Self {
        match num {
            0 => Self::None,
            1 => Self::U8,
            2 => Self::U16,
            3 => Self::U32,
            4 => Self::U64,
            5 => Self::U128,
            6 => Self::I8,
            7 => Self::I16,
            8 => Self::I32,
            9 => Self::I64,
            10 => Self::I128,
            11 => Self::F32,
            12 => Self::F64,
            13 => Self::CHAR,
            14 => Self::USIZE,
            15 => Self::ISIZE,
            16 => Self::BOOL,
            252 => Self::ResultOk,
            253 => Self::ResultErr,
            254 => Self::Some,
            255 => Self::DynSize,
            num => Self::Extension(num),
        }
    }

    pub const fn type_size(&self) -> Option<usize> {
        match self {
            Self::None => Some(0),
            Self::U8 => Some(1),
            Self::U16 => Some(2),
            Self::U32 => Some(4),
            Self::U64 => Some(8),
            Self::U128 => Some(16),
            Self::I8 => Some(1),
            Self::I16 => Some(2),
            Self::I32 => Some(4),
            Self::I64 => Some(8),
            Self::I128 => Some(16),
            Self::F32 => Some(4),
            Self::F64 => Some(8),
            Self::CHAR => Some(1),
            Self::USIZE => Some(8),
            Self::ISIZE => Some(8),
            Self::BOOL => Some(1),
            _ => None,
        }
    }
}

pub trait FromBytes<T>: Sized {
    fn from_bytes_le(bytes: T) -> Self;
    fn from_bytes_be(bytes: T) -> Self;
    fn from_bytes_ne(bytes: T) -> Self;
}

pub trait TryReadBytes: Sized {
    fn try_read_bytes_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr>;
    fn try_read_bytes_be<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr>;
    fn try_read_bytes_ne<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr>;
}

pub trait ToBytes<T> {
    fn to_bytes_le(&self) -> T;
    fn to_bytes_be(&self) -> T;
    fn to_bytes_ne(&self) -> T;
}

pub trait TryWriteBytes {
    fn try_write_bytes_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr>;
    fn try_write_bytes_be<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr>;
    fn try_write_bytes_ne<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr>;
}
pub trait ByteSize: Sized {
    const BYTE_SIZE: usize;
    const TYPE_CODE: TypeCode;
}

pub trait AsArrayRef {
    fn as_array_ref<'a>(&'a self) -> &'a [u8];
    fn as_array_len(&self) -> usize;
}

impl<const N: usize> AsArrayRef for [u8; N] {
    fn as_array_ref<'a>(&'a self) -> &'a [u8] {
        self
    }
    fn as_array_len(&self) -> usize {
        self.len()
    }
}

impl AsArrayRef for String {
    fn as_array_ref<'a>(&'a self) -> &'a [u8] {
        self.as_bytes()
    }
    fn as_array_len(&self) -> usize {
        self.len()
    }
}

impl AsArrayRef for Vec<u8> {
    fn as_array_ref<'a>(&'a self) -> &'a [u8] {
        &self
    }
    fn as_array_len(&self) -> usize {
        self.len()
    }
}

pub trait AsArraySelf<const N: usize>: AsArrayRef {
    fn as_array_self(self) -> [u8; N];
}

impl<const N: usize> AsArraySelf<N> for [u8; N] {
    fn as_array_self(self) -> [u8; N] {
        self
    }
}

macro_rules! impl_byte_size {
    ($ty:ty, $size: literal, $ty_code: ident) => {
        impl ByteSize for $ty {
            const BYTE_SIZE: usize = $size;
            const TYPE_CODE: TypeCode = TypeCode::$ty_code;
        }

        impl<T> FromBytes<T> for $ty
        where
            T: AsArraySelf<$size>,
        {
            fn from_bytes_le(bytes: T) -> Self {
                <$ty>::from_le_bytes(bytes.as_array_self())
            }
            fn from_bytes_be(bytes: T) -> Self {
                <$ty>::from_be_bytes(bytes.as_array_self())
            }
            fn from_bytes_ne(bytes: T) -> Self {
                <$ty>::from_ne_bytes(bytes.as_array_self())
            }
        }

        impl TryReadBytes for $ty {
            fn try_read_bytes_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                Ok(<$ty>::from_le_bytes(buf))
            }
            fn try_read_bytes_be<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                Ok(<$ty>::from_be_bytes(buf))
            }
            fn try_read_bytes_ne<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                Ok(<$ty>::from_ne_bytes(buf))
            }
        }

        impl ToBytes<[u8; $size]> for $ty {
            fn to_bytes_le(&self) -> [u8; $size] {
                self.to_le_bytes()
            }
            fn to_bytes_be(&self) -> [u8; $size] {
                self.to_be_bytes()
            }
            fn to_bytes_ne(&self) -> [u8; $size] {
                self.to_ne_bytes()
            }
        }
        impl TryWriteBytes for $ty {
            fn try_write_bytes_le<W: std::io::Write>(
                &self,
                stream: &mut W,
            ) -> Result<(), BytesErr> {
                stream.write(&self.to_le_bytes())?;
                Ok(())
            }
            fn try_write_bytes_be<W: std::io::Write>(
                &self,
                stream: &mut W,
            ) -> Result<(), BytesErr> {
                stream.write(&self.to_be_bytes())?;
                Ok(())
            }
            fn try_write_bytes_ne<W: std::io::Write>(
                &self,
                stream: &mut W,
            ) -> Result<(), BytesErr> {
                stream.write(&self.to_ne_bytes())?;
                Ok(())
            }
        }
    };
    ($ty: ty, $override: ty, $size: literal, $ty_code: ident) => {
        impl ByteSize for $ty {
            const BYTE_SIZE: usize = $size;
            const TYPE_CODE: TypeCode = TypeCode::$ty_code;
        }

        impl<T> FromBytes<T> for $ty
        where
            T: AsArraySelf<$size>,
        {
            fn from_bytes_le(bytes: T) -> Self {
                <$override>::from_le_bytes(bytes.as_array_self()) as Self
            }
            fn from_bytes_be(bytes: T) -> Self {
                <$override>::from_be_bytes(bytes.as_array_self()) as Self
            }
            fn from_bytes_ne(bytes: T) -> Self {
                <$override>::from_ne_bytes(bytes.as_array_self()) as Self
            }
        }

        impl ToBytes<[u8; $size]> for $ty {
            fn to_bytes_le(&self) -> [u8; $size] {
                (*self as $override).to_le_bytes()
            }
            fn to_bytes_be(&self) -> [u8; $size] {
                (*self as $override).to_be_bytes()
            }
            fn to_bytes_ne(&self) -> [u8; $size] {
                (*self as $override).to_ne_bytes()
            }
        }
        impl ToBytes<Vec<u8>> for $ty {
            fn to_bytes_le(&self) -> Vec<u8> {
                (*self as $override).to_le_bytes().to_vec()
            }
            fn to_bytes_be(&self) -> Vec<u8> {
                (*self as $override).to_be_bytes().to_vec()
            }
            fn to_bytes_ne(&self) -> Vec<u8> {
                (*self as $override).to_ne_bytes().to_vec()
            }
        }
        impl TryWriteBytes for $ty {
            fn try_write_bytes_le<W: std::io::Write>(
                &self,
                stream: &mut W,
            ) -> Result<(), BytesErr> {
                stream.write(&(*self as $override).to_le_bytes())?;
                Ok(())
            }
            fn try_write_bytes_be<W: std::io::Write>(
                &self,
                stream: &mut W,
            ) -> Result<(), BytesErr> {
                stream.write(&(*self as $override).to_be_bytes())?;
                Ok(())
            }
            fn try_write_bytes_ne<W: std::io::Write>(
                &self,
                stream: &mut W,
            ) -> Result<(), BytesErr> {
                stream.write(&(*self as $override).to_ne_bytes())?;
                Ok(())
            }
        }
        impl TryReadBytes for $ty {
            fn try_read_bytes_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                Ok(<$override>::from_le_bytes(buf) as $ty)
            }
            fn try_read_bytes_be<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                Ok(<$override>::from_le_bytes(buf) as $ty)
            }
            fn try_read_bytes_ne<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                Ok(<$override>::from_le_bytes(buf) as $ty)
            }
        }
    };
}

impl_byte_size!(u8, 1, U8);
impl_byte_size!(u16, 2, U16);
impl_byte_size!(u32, 4, U32);
impl_byte_size!(u64, 8, U64);
impl_byte_size!(u128, 16, U128);
impl_byte_size!(i8, 1, I8);
impl_byte_size!(i16, 2, I16);
impl_byte_size!(i32, 4, I32);
impl_byte_size!(i64, 8, I64);
impl_byte_size!(i128, 16, I128);
impl_byte_size!(f32, 4, F32);
impl_byte_size!(f64, 8, F64);
impl_byte_size!(char, u8, 1, CHAR);
impl_byte_size!(usize, u64, 8, USIZE);
impl_byte_size!(isize, i64, 8, ISIZE);

impl ByteSize for bool {
    const BYTE_SIZE: usize = 1;
    const TYPE_CODE: TypeCode = TypeCode::BOOL;
}

impl<T> FromBytes<T> for bool
where
    T: AsArraySelf<1>,
{
    fn from_bytes_le(bytes: T) -> Self {
        <u8>::from_le_bytes(bytes.as_array_self()) == 0
    }
    fn from_bytes_be(bytes: T) -> Self {
        <u8>::from_be_bytes(bytes.as_array_self()) == 0
    }
    fn from_bytes_ne(bytes: T) -> Self {
        <u8>::from_ne_bytes(bytes.as_array_self()) == 0
    }
}

impl TryReadBytes for bool {
    fn try_read_bytes_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let mut buf = [0u8; 1];
        stream.read_exact(&mut buf)?;
        Ok(buf[0] == 1)
    }
    fn try_read_bytes_be<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let mut buf = [0u8; 1];
        stream.read_exact(&mut buf)?;
        Ok(buf[0] == 1)
    }
    fn try_read_bytes_ne<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let mut buf = [0u8; 1];
        stream.read_exact(&mut buf)?;
        Ok(buf[0] == 1)
    }
}

impl ToBytes<[u8; 1]> for bool {
    fn to_bytes_le(&self) -> [u8; 1] {
        if *self { 1u8 } else { 0u8 }.to_le_bytes()
    }
    fn to_bytes_be(&self) -> [u8; 1] {
        if *self { 1u8 } else { 0u8 }.to_be_bytes()
    }
    fn to_bytes_ne(&self) -> [u8; 1] {
        if *self { 1u8 } else { 0u8 }.to_ne_bytes()
    }
}

pub trait WriteByteStreamLE: Sized {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr>;
}
pub trait ReadByteStreamLE: Sized {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr>;
}

impl<TT: TryWriteBytes + ByteSize> WriteByteStreamLE for TT {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        stream.write_all(&[TT::TYPE_CODE.as_u8()])?;
        self.try_write_bytes_le(stream)?;
        Ok(())
    }
}

impl<TT: TryReadBytes + ByteSize> ReadByteStreamLE for TT {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let expected = Self::TYPE_CODE;
        let mut type_buf = [0u8; 1];
        stream.read_exact(&mut type_buf)?;
        if type_buf[0] != expected.as_u8() {
            return Err(BytesErr::UnexpectedTypeCode {
                expected,
                found: TypeCode::from_u8(type_buf[0]),
            });
        }
        Ok(<TT>::try_read_bytes_le(stream)?)
    }
}
impl<TT: TryWriteBytes + ByteSize> WriteByteStreamLE for std::sync::Arc<TT> {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        stream.write_all(&[TT::TYPE_CODE.as_u8()])?;
        self.try_write_bytes_le(stream)?;
        Ok(())
    }
}

impl<TT: TryReadBytes + ByteSize> ReadByteStreamLE for std::sync::Arc<TT> {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let expected = TT::TYPE_CODE;
        let mut type_buf = [0u8; 1];
        stream.read_exact(&mut type_buf)?;
        if type_buf[0] != expected.as_u8() {
            return Err(BytesErr::UnexpectedTypeCode {
                expected,
                found: TypeCode::from_u8(type_buf[0]),
            });
        }
        Ok(std::sync::Arc::new(<TT>::try_read_bytes_le(stream)?))
    }
}

impl<TT: TryWriteBytes + ByteSize> WriteByteStreamLE for std::rc::Rc<TT> {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        TT::TYPE_CODE.as_u8().try_write_bytes_le(stream)?;
        self.try_write_bytes_le(stream)?;
        Ok(())
    }
}

impl<TT: TryReadBytes + ByteSize> ReadByteStreamLE for std::rc::Rc<TT> {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let expected = TT::TYPE_CODE;
        let found = TypeCode::from_u8(<u8>::try_read_bytes_le(stream)?);
        if found != expected {
            return Err(BytesErr::UnexpectedTypeCode { expected, found });
        }
        Ok(std::rc::Rc::new(<TT>::try_read_bytes_le(stream)?))
    }
}

impl<TT: WriteByteStreamLE> WriteByteStreamLE for Option<TT> {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        match self {
            Some(t) => {
                TypeCode::Some.as_u8().try_write_bytes_le(stream)?;
                t.write_byte_stream_le(stream)?;
            }
            None => {
                TypeCode::None.as_u8().try_write_bytes_le(stream)?;
            }
        }
        Ok(())
    }
}
impl<TT: ReadByteStreamLE> ReadByteStreamLE for Option<TT> {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let found = TypeCode::from_u8(<u8>::try_read_bytes_le(stream)?);
        match found {
            TypeCode::None => Ok(None),
            TypeCode::Some => Ok(Some(TT::read_byte_stream_le(stream)?)),
            _ => Err(BytesErr::UnexpectedTypeCode {
                expected: TypeCode::Some,
                found,
            }),
        }
    }
}
impl<TT: WriteByteStreamLE, TTT: WriteByteStreamLE> WriteByteStreamLE for Result<TT, TTT> {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        match self {
            Ok(a) => {
                TypeCode::ResultOk.as_u8().try_write_bytes_le(stream)?;
                a.write_byte_stream_le(stream)
            }
            Err(b) => {
                TypeCode::ResultErr.as_u8().try_write_bytes_le(stream)?;
                b.write_byte_stream_le(stream)
            }
        }
    }
}

impl<TT: ReadByteStreamLE, TTT: ReadByteStreamLE> ReadByteStreamLE for Result<TT, TTT> {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let found = TypeCode::from_u8(<u8>::try_read_bytes_le(stream)?);
        match found {
            TypeCode::ResultOk => Ok(Ok(TT::read_byte_stream_le(stream)?)),
            TypeCode::ResultErr => Ok(Err(TTT::read_byte_stream_le(stream)?)),
            _ => Err(BytesErr::UnexpectedTypeCode {
                expected: TypeCode::ResultErr,
                found,
            }),
        }
    }
}

impl<'a, TT: WriteByteStreamLE> WriteByteStreamLE for &[TT] {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        TypeCode::DynSize.as_u8().try_write_bytes_le(stream)?;
        self.len().try_write_bytes_le(stream)?;
        for chunk in *self {
            chunk.write_byte_stream_le(stream)?;
        }
        Ok(())
    }
}

impl<'a, TT: WriteByteStreamLE> WriteByteStreamLE for Vec<TT> {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        TypeCode::DynSize.as_u8().try_write_bytes_le(stream)?;
        self.len().try_write_bytes_le(stream)?;
        for chunk in self {
            chunk.write_byte_stream_le(stream)?;
        }
        Ok(())
    }
}

impl<'a, TT: ReadByteStreamLE> ReadByteStreamLE for Vec<TT> {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let found = TypeCode::from_u8(<u8>::try_read_bytes_le(stream)?);
        match found {
            TypeCode::DynSize => {
                let len = <usize>::try_read_bytes_le(stream)?;
                let mut entries = Vec::with_capacity(len);
                for _ in 0..len {
                    entries.push(<TT>::read_byte_stream_le(stream)?);
                }

                Ok(entries)
            }
            _ => Err(BytesErr::UnexpectedTypeCode {
                expected: TypeCode::DynSize,
                found,
            }),
        }
    }
}

impl<'a> WriteByteStreamLE for String {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        stream.write(&[TypeCode::DynSize.as_u8(), TypeCode::CHAR.as_u8()])?;
        self.len().try_write_bytes_le(stream)?;
        stream.write(self.as_bytes())?;
        Ok(())
    }
}

impl<'a> ReadByteStreamLE for String {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let mut type_buf = [0u8; 2];
        stream.read_exact(&mut type_buf)?;
        let init_ty = TypeCode::from_u8(type_buf[0]);
        match init_ty {
            TypeCode::DynSize => {
                if TypeCode::from_u8(type_buf[1]) == TypeCode::CHAR {
                    let mut capacity_buf = [0u8; 8];
                    stream.read_exact(&mut capacity_buf)?;
                    let capacity = <u64>::from_le_bytes(capacity_buf);
                    let mut entries = vec![0u8; capacity as usize];
                    stream.read_exact(entries.as_mut_slice())?;
                    Ok(String::from_utf8(entries)?)
                } else {
                    Err(BytesErr::UnexpectedTypeCode {
                        expected: TypeCode::CHAR,
                        found: init_ty,
                    })
                }
            }
            _ => Err(BytesErr::UnexpectedTypeCode {
                expected: TypeCode::DynSize,
                found: init_ty,
            }),
        }
    }
}

impl<'a, A: WriteByteStreamLE, B: WriteByteStreamLE> WriteByteStreamLE for HashMap<A, B> {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        TypeCode::DynSize.as_u8().try_write_bytes_le(stream)?;
        self.len().try_write_bytes_le(stream)?;
        for (a, b) in self {
            a.write_byte_stream_le(stream)?;
            b.write_byte_stream_le(stream)?;
        }
        Ok(())
    }
}

impl<'a, A: ReadByteStreamLE + Eq + Hash, B: ReadByteStreamLE + Eq + Hash> ReadByteStreamLE
    for HashMap<A, B>
{
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let found = TypeCode::from_u8(<u8>::try_read_bytes_le(stream)?);
        match found {
            TypeCode::DynSize => {
                let len = <usize>::try_read_bytes_le(stream)?;
                let mut entries = HashMap::new();
                for _ in 0..len {
                    let a = <A>::read_byte_stream_le(stream)?;
                    let b = <B>::read_byte_stream_le(stream)?;
                    entries.insert(a, b);
                }

                Ok(entries)
            }
            _ => Err(BytesErr::UnexpectedTypeCode {
                expected: TypeCode::DynSize,
                found,
            }),
        }
    }
}

impl<'a, A: WriteByteStreamLE> WriteByteStreamLE for HashSet<A> {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        TypeCode::DynSize.as_u8().try_write_bytes_le(stream)?;
        self.len().try_write_bytes_le(stream)?;
        for a in self {
            a.write_byte_stream_le(stream)?;
        }
        Ok(())
    }
}

impl<'a, A: ReadByteStreamLE + Eq + Hash> ReadByteStreamLE for HashSet<A> {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let found = TypeCode::from_u8(<u8>::try_read_bytes_le(stream)?);
        match found {
            TypeCode::DynSize => {
                let len = <usize>::try_read_bytes_le(stream)?;
                let mut entries = HashSet::new();
                for _ in 0..len {
                    let a = <A>::read_byte_stream_le(stream)?;
                    entries.insert(a);
                }

                Ok(entries)
            }
            _ => Err(BytesErr::UnexpectedTypeCode {
                expected: TypeCode::DynSize,
                found,
            }),
        }
    }
}

impl<'a, A: WriteByteStreamLE, B: WriteByteStreamLE> WriteByteStreamLE for BTreeMap<A, B> {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        TypeCode::DynSize.as_u8().try_write_bytes_le(stream)?;
        self.len().try_write_bytes_le(stream)?;
        for (a, b) in self {
            a.write_byte_stream_le(stream)?;
            b.write_byte_stream_le(stream)?;
        }
        Ok(())
    }
}

impl<'a, A: ReadByteStreamLE + Ord, B: ReadByteStreamLE + Ord> ReadByteStreamLE for BTreeMap<A, B> {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let found = TypeCode::from_u8(<u8>::try_read_bytes_le(stream)?);
        match found {
            TypeCode::DynSize => {
                let len = <usize>::try_read_bytes_le(stream)?;
                let mut entries = BTreeMap::new();
                for _ in 0..len {
                    let a = <A>::read_byte_stream_le(stream)?;
                    let b = <B>::read_byte_stream_le(stream)?;
                    entries.insert(a, b);
                }

                Ok(entries)
            }
            _ => Err(BytesErr::UnexpectedTypeCode {
                expected: TypeCode::DynSize,
                found,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_str() {
        let s = String::from("just a test");
        let mut stream = Cursor::new(Vec::<u8>::new());
        s.write_byte_stream_le(&mut stream).unwrap();
        stream.set_position(0);
        assert_eq!(s, <String>::read_byte_stream_le(&mut stream).unwrap());
    }

    #[test]
    fn test_u8() {
        let mut stream = Cursor::new(Vec::<u8>::new());
        0u8.write_byte_stream_le(&mut stream).unwrap();
        stream.set_position(0);
        assert_eq!(0u8, <u8>::read_byte_stream_le(&mut stream).unwrap());
    }

    #[test]
    fn test_vec() {
        let v = vec![0, 1, 1];
        let mut stream = Cursor::new(Vec::<u8>::new());
        v.write_byte_stream_le(&mut stream).unwrap();
        stream.set_position(0);
        assert_eq!(v, <Vec<i32>>::read_byte_stream_le(&mut stream).unwrap());

        let v = vec!["asdf".to_string(), "a".to_string(), "bb".to_string()];
        let mut stream = Cursor::new(Vec::<u8>::new());
        v.write_byte_stream_le(&mut stream).unwrap();
        stream.set_position(0);
        assert_eq!(v, <Vec<String>>::read_byte_stream_le(&mut stream).unwrap());
    }
}

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
}

pub trait FromBytes<T>: Sized {
    fn from_bytes_le(bytes: T) -> Self;
    fn from_bytes_be(bytes: T) -> Self;
    fn from_bytes_ne(bytes: T) -> Self;
}

pub trait TryFromBytes<T: AsArrayRef>: Sized {
    fn try_from_bytes_le(bytes: T) -> Result<Self, BytesErr>;
    fn try_from_bytes_be(bytes: T) -> Result<Self, BytesErr>;
    fn try_from_bytes_ne(bytes: T) -> Result<Self, BytesErr>;
}

pub trait ToBytes<T> {
    fn to_bytes_le(&self) -> T;
    fn to_bytes_be(&self) -> T;
    fn to_bytes_ne(&self) -> T;
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

        impl<T> TryFromBytes<T> for $ty
        where
            T: AsArrayRef,
        {
            fn try_from_bytes_le(bytes: T) -> Result<Self, BytesErr> {
                match bytes.as_array_ref().first_chunk::<$size>() {
                    Some(bytes) => Ok(<$ty>::from_le_bytes(*bytes)),
                    None => Err(BytesErr::NotEnoughBytes {
                        need: $size,
                        found: bytes.as_array_len(),
                    }),
                }
            }
            fn try_from_bytes_be(bytes: T) -> Result<Self, BytesErr> {
                match bytes.as_array_ref().first_chunk::<$size>() {
                    Some(bytes) => Ok(<$ty>::from_be_bytes(*bytes)),
                    None => Err(BytesErr::NotEnoughBytes {
                        need: $size,
                        found: bytes.as_array_len(),
                    }),
                }
            }
            fn try_from_bytes_ne(bytes: T) -> Result<Self, BytesErr> {
                match bytes.as_array_ref().first_chunk::<$size>() {
                    Some(bytes) => Ok(<$ty>::from_ne_bytes(*bytes)),
                    None => Err(BytesErr::NotEnoughBytes {
                        need: $size,
                        found: bytes.as_array_len(),
                    }),
                }
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

impl<T> TryFromBytes<T> for bool
where
    T: AsArrayRef,
{
    fn try_from_bytes_le(bytes: T) -> Result<Self, BytesErr> {
        match bytes.as_array_ref().first_chunk::<1>() {
            Some(bytes) => Ok(<u8>::from_le_bytes(*bytes) != 0),
            None => Err(BytesErr::NotEnoughBytes {
                need: 1,
                found: bytes.as_array_len(),
            }),
        }
    }
    fn try_from_bytes_be(bytes: T) -> Result<Self, BytesErr> {
        match bytes.as_array_ref().first_chunk::<1>() {
            Some(bytes) => Ok(<u8>::from_be_bytes(*bytes) != 0),
            None => Err(BytesErr::NotEnoughBytes {
                need: 1,
                found: bytes.as_array_len(),
            }),
        }
    }
    fn try_from_bytes_ne(bytes: T) -> Result<Self, BytesErr> {
        match bytes.as_array_ref().first_chunk::<1>() {
            Some(bytes) => Ok(<u8>::from_ne_bytes(*bytes) != 0),
            None => Err(BytesErr::NotEnoughBytes {
                need: 1,
                found: bytes.as_array_len(),
            }),
        }
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

pub trait WriteByteStreamLE<T>: Sized {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr>;
}
pub trait ReadByteStreamLE<T>: Sized {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr>;
}

impl<T: AsArrayRef, TT: TryFromBytes<Vec<u8>> + ToBytes<T> + ByteSize> WriteByteStreamLE<T> for TT {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        stream.write_all(&[TT::TYPE_CODE.as_u8()])?;
        stream.write_all(self.to_bytes_le().as_array_ref())?;
        Ok(())
    }
}
impl<T: AsArrayRef, TT: TryFromBytes<Vec<u8>> + ToBytes<T> + ByteSize> ReadByteStreamLE<T> for TT {
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
        let mut num_buf = Vec::with_capacity(Self::BYTE_SIZE);
        stream.read_exact(&mut num_buf)?;
        Ok(TT::try_from_bytes_le(num_buf)?)
    }
}
impl<T: AsArrayRef, TT: TryFromBytes<Vec<u8>> + ToBytes<T> + ByteSize> WriteByteStreamLE<T>
    for std::sync::Arc<TT>
{
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        stream.write_all(&[TT::TYPE_CODE.as_u8()])?;
        stream.write_all(self.to_bytes_le().as_array_ref())?;
        Ok(())
    }
}

impl<T: AsArrayRef, TT: TryFromBytes<Vec<u8>> + ToBytes<T> + ByteSize> ReadByteStreamLE<T>
    for std::sync::Arc<TT>
{
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
        let mut num_buf = Vec::with_capacity(TT::BYTE_SIZE);
        stream.read_exact(&mut num_buf)?;
        Ok(std::sync::Arc::new(TT::try_from_bytes_le(num_buf)?))
    }
}

impl<T: AsArrayRef, TT: TryFromBytes<Vec<u8>> + ToBytes<T> + ByteSize> WriteByteStreamLE<T>
    for std::rc::Rc<TT>
{
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        stream.write_all(&[TT::TYPE_CODE.as_u8()])?;
        stream.write_all(self.to_bytes_le().as_array_ref())?;
        Ok(())
    }
}
impl<T: AsArrayRef, TT: TryFromBytes<Vec<u8>> + ToBytes<T> + ByteSize> ReadByteStreamLE<T>
    for std::rc::Rc<TT>
{
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
        let mut num_buf = Vec::with_capacity(TT::BYTE_SIZE);
        stream.read_exact(&mut num_buf)?;
        Ok(std::rc::Rc::new(TT::try_from_bytes_le(num_buf)?))
    }
}

impl<T: AsArrayRef, TT: WriteByteStreamLE<T>> WriteByteStreamLE<T> for Option<TT> {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        match self {
            Some(t) => {
                stream.write_all(&[TypeCode::Some.as_u8()])?;
                t.write_byte_stream_le(stream)?;
            }
            None => {
                stream.write_all(&[TypeCode::None.as_u8()])?;
            }
        }
        Ok(())
    }
}
impl<T: AsArrayRef, TT: ReadByteStreamLE<T>> ReadByteStreamLE<T> for Option<TT> {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let mut type_buf = [0u8; 1];
        stream.read_exact(&mut type_buf)?;
        match TypeCode::from_u8(type_buf[0]) {
            TypeCode::None => Ok(None),
            TypeCode::Some => Ok(Some(TT::read_byte_stream_le(stream)?)),
            _ => Err(BytesErr::UnexpectedTypeCode {
                expected: TypeCode::Some,
                found: TypeCode::from_u8(type_buf[0]),
            }),
        }
    }
}
impl<T: AsArrayRef, TT: WriteByteStreamLE<T>, TTT: WriteByteStreamLE<T>> WriteByteStreamLE<T>
    for Result<TT, TTT>
{
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        match self {
            Ok(a) => {
                stream.write(&[TypeCode::ResultOk.as_u8()])?;
                a.write_byte_stream_le(stream)
            }
            Err(b) => {
                stream.write(&[TypeCode::ResultErr.as_u8()])?;
                b.write_byte_stream_le(stream)
            }
        }
    }
}

impl<T: AsArrayRef, TT: ReadByteStreamLE<T>, TTT: ReadByteStreamLE<T>> ReadByteStreamLE<T>
    for Result<TT, TTT>
{
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let mut type_buf = [0u8; 1];
        stream.read_exact(&mut type_buf)?;
        match TypeCode::from_u8(type_buf[0]) {
            TypeCode::ResultOk => Ok(Ok(TT::read_byte_stream_le(stream)?)),
            TypeCode::ResultErr => Ok(Err(TTT::read_byte_stream_le(stream)?)),
            _ => Err(BytesErr::UnexpectedTypeCode {
                expected: TypeCode::ResultErr,
                found: TypeCode::from_u8(type_buf[0]),
            }),
        }
    }
}

impl<'a, T: AsArrayRef, TT: ToBytes<T> + ByteSize> WriteByteStreamLE<T> for &[TT] {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        stream.write(&[TypeCode::DynSize.as_u8(), TT::TYPE_CODE.as_u8()])?;
        stream.write(&self.len().to_bytes_le())?;
        for chunk in *self {
            stream.write(chunk.to_bytes_le().as_array_ref())?;
        }
        Ok(())
    }
}

impl<'a, T: AsArrayRef, TT: ToBytes<T> + ByteSize> WriteByteStreamLE<T> for Vec<TT> {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
        stream.write(&[TypeCode::DynSize.as_u8(), TT::TYPE_CODE.as_u8()])?;
        stream.write(&self.len().to_bytes_le())?;
        for chunk in self {
            stream.write(chunk.to_bytes_le().as_array_ref())?;
        }
        Ok(())
    }
}

impl<'a, T: AsArrayRef, TT: TryFromBytes<Vec<u8>> + ByteSize> ReadByteStreamLE<T> for Vec<TT> {
    fn read_byte_stream_le<R: std::io::Read>(stream: &mut R) -> Result<Self, BytesErr> {
        let mut type_buf = [0u8; 2];
        stream.read_exact(&mut type_buf)?;
        let init_ty = TypeCode::from_u8(type_buf[0]);
        match init_ty {
            TypeCode::DynSize => {
                if TypeCode::from_u8(type_buf[1]) == TT::TYPE_CODE {
                    let mut capacity_buf = [0u8; 8];
                    stream.read_exact(&mut capacity_buf)?;
                    let capacity = <u64>::from_le_bytes(capacity_buf);
                    let mut entries = Vec::with_capacity(capacity as usize);
                    for _ in 0..capacity {
                        let mut buf = Vec::with_capacity(TT::BYTE_SIZE);
                        stream.read(&mut buf)?; //<< TODO: this is not correct
                        entries.push(<TT>::try_from_bytes_le(buf)?);
                    }
                    Ok(entries)
                } else {
                    Err(BytesErr::UnexpectedTypeCode {
                        expected: TT::TYPE_CODE,
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
impl<'a, T: AsArrayRef> ReadByteStreamLE<T> for String {
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
                    let mut entries = Vec::with_capacity(capacity as usize);
                    stream.read(&mut entries)?; //TODO: this is not correct
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

// impl<'a, T: AsArrayRef> ByteStreamLE<T> for String {
//     fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr> {
//         stream.write(&[DYN_SIZE_CODE, 1])?;
//         stream.write(&self.len().to_bytes_le())?;
//         stream.write(self.as_bytes())?;
//         Ok(())
//     }
//     fn read_byte_stream_le<R: std::io::Read>(&self, stream: &mut R) -> Result<Self, BytesErr> {
//         todo!()
//     }
// }

// pub trait FromByteBuf<const N: usize>: FromBytes<[u8; N]> {
//     fn from_first_sf_le_chunk(bytes: &[u8]) -> Result<Self, BytesErr> {
//         match bytes.first_chunk::<N>() {
//             Some(chunk) => Ok(<Self>::from_sf_le_bytes(*chunk)),
//             None => Err(BytesErr::NotEnoughBytes {
//                 need: N,
//                 found: bytes.len(),
//             }),
//         }
//     }
//     fn from_first_sf_be_chunk(bytes: &[u8]) -> Result<Self, BytesErr> {
//         match bytes.first_chunk::<N>() {
//             Some(chunk) => Ok(<Self>::from_sf_be_bytes(*chunk)),
//             None => Err(BytesErr::NotEnoughBytes {
//                 need: N,
//                 found: bytes.len(),
//             }),
//         }
//     }
//     fn from_first_sf_ne_chunk(bytes: &[u8]) -> Result<Self, BytesErr> {
//         match bytes.first_chunk::<N>() {
//             Some(chunk) => Ok(<Self>::from_sf_ne_bytes(*chunk)),
//             None => Err(BytesErr::NotEnoughBytes {
//                 need: N,
//                 found: bytes.len(),
//             }),
//         }
//     }
// }

// pub trait FromByteChunkBuf {
//     fn next_le_chunk
// }

// #[repr(transparent)]
// pub struct ByteChunk<T: ByteKind> {
//     chunk: Box<dyn AsArrayRef>,
//     byte_kind: PhantomData<T>,
// }

// impl<TT: ByteKind> ByteChunk<TT> {
//     pub fn new<T: AsArrayRef + 'static>(chunk: T) -> Self {
//         Self {
//             chunk: Box::new(chunk),
//             byte_kind: PhantomData,
//         }
//     }
//     pub fn iter(&self) -> std::slice::Iter<'_, u8> {
//         self.chunk.as_array_ref().iter()
//     }

//     pub fn len(&self) -> usize {
//         self.chunk.as_array_len()
//     }

//     pub fn write_to_stream<W: std::io::write_byte_stream>(&self, stream: &mut W) -> Result<(), std::io::Error> {
//         stream.write_byte_stream(&TT::to_bytes(&self.chunk.as_array_len()))?;
//         stream.write_byte_stream(self.into())?;
//         Ok(())
//     }

//     pub fn read_from_stream<R: std::io::Read>(
//         &self,
//         stream: &mut R,
//     ) -> Result<Self, std::io::Error> {
//         let mut capacity_buf = [0u8; 8];
//         stream.read(&mut capacity_buf)?;
//         let capacity: usize = TT::from_bytes(capacity_buf);
//         let mut item_buf = Vec::with_capacity(capacity);
//         stream.read(&mut item_buf)?;
//         // stream.write_byte_stream(&TT::to_bytes(&self.chunk.as_array_len()))?;
//         // stream.write_byte_stream(self.into())?;
//         Ok(Self::new(item_buf))
//     }
// }

// impl<'a, T: ByteKind> IntoIterator for &'a ByteChunk<T> {
//     type Item = &'a u8;
//     type IntoIter = std::slice::Iter<'a, u8>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.iter()
//     }
// }

// impl<'a, T: ByteKind> From<&'a ByteChunk<T>> for &'a [u8] {
//     fn from(from: &'a ByteChunk<T>) -> &'a [u8] {
//         from.chunk.as_array_ref()
//     }
// }

// impl<TT: ByteKind> ByteChunk<TT> {
//     // pub fn take_chunk<const N: usize, T: FromByteBuf<N>>(&self) -> Result<T, BytesErr> {
//     //     <T>::from_first_sf_le_chunk(self.chunk.as_array_ref())
//     // }
// }

// pub struct ByteChunkBuf<T: ByteKind> {
//     inner: Vec<ByteChunk<T>>,
// }

// impl<TT: ByteKind> ByteChunkBuf<TT> {
//     pub fn new() -> Self {
//         Self { inner: Vec::new() }
//     }

//     pub fn push<T: AsArrayRef + 'static>(&mut self, to_push: T) {
//         self.inner.push(ByteChunk::<TT>::new(to_push));
//     }

//     pub fn as_bytes<'a>(&'a self) -> impl Iterator<Item = &'a u8> + 'a {
//         self.inner.iter().flat_map(|chunk| chunk.iter())
//     }

//     pub fn write_all_to_stream<W: std::io::write_byte_stream>(
//         &self,
//         stream: &mut W,
//     ) -> Result<(), std::io::Error> {
//         for chunk in self.inner.iter() {
//             stream.write_byte_stream(&TT::to_bytes(&chunk.len()))?;
//             stream.write_byte_stream(chunk.into())?;
//         }

//         Ok(())
//     }
// }

// impl<T: ByteKind> IntoIterator for ByteChunkBuf<T> {
//     type Item = ByteChunk<T>;
//     type IntoIter = std::vec::IntoIter<Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.inner.into_iter()
//     }
// }

// impl<'a, T: ByteKind> IntoIterator for &'a ByteChunkBuf<T> {
//     type Item = &'a ByteChunk<T>;
//     type IntoIter = std::slice::Iter<'a, ByteChunk<T>>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.inner.iter()
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test() {
//         let t = 4u8;
//         let bytes = t.to_sf_le_bytes();
//         assert!(<u8>::from_sf_le_bytes(bytes) == t);
//     }
// }

#[derive(Debug)]
pub enum BytesErr {
    NotEnoughBytes {
        need: usize,
        found: usize,
    },
    IoError(std::io::Error),
    FromUtf8Error(std::string::FromUtf8Error),
    UnexpectedTypeCode {
        expected: TypeCode,
        found: TypeCode,
    },
    ChecksumFailed {
        expected: usize,
        found: usize,
    },
    Extension {
        crate_name: &'static str, //TODO: make error extension crate
    },
}

impl BytesErr {
    pub fn compare_checksums(expected: usize, found: usize) -> Result<(), BytesErr> {
        if expected == found {
            Ok(())
        } else {
            Err(BytesErr::ChecksumFailed { expected, found })
        }
    }
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
    pub fn expect_from_stream_le<R: std::io::Read>(
        self,
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<(), BytesErr> {
        self.expect_from_u8(<u8>::try_read_bytes_le(stream, checksum)?)
    }
    pub fn expect_from_stream_be<R: std::io::Read>(
        self,
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<(), BytesErr> {
        self.expect_from_u8(<u8>::try_read_bytes_be(stream, checksum)?)
    }
    pub fn expect_from_stream_ne<R: std::io::Read>(
        self,
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<(), BytesErr> {
        self.expect_from_u8(<u8>::try_read_bytes_ne(stream, checksum)?)
    }

    pub fn expect_from_u8(self, num: u8) -> Result<(), BytesErr> {
        let found = TypeCode::from_u8(num);
        if found == self {
            Ok(())
        } else {
            Err(BytesErr::UnexpectedTypeCode {
                expected: self,
                found,
            })
        }
    }
}

pub trait FromBytes<T>: Sized {
    fn from_bytes_le(bytes: T) -> Self;
    fn from_bytes_be(bytes: T) -> Self;
    fn from_bytes_ne(bytes: T) -> Self;
}

pub trait TryReadBytes: Sized {
    fn try_read_bytes_le<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr>;
    fn try_read_bytes_be<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr>;
    fn try_read_bytes_ne<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr>;
}

pub trait ToBytes<T> {
    fn to_bytes_le(&self) -> T;
    fn to_bytes_be(&self) -> T;
    fn to_bytes_ne(&self) -> T;
}

pub trait TryWriteBytes {
    fn try_write_bytes_le<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr>;
    fn try_write_bytes_be<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr>;
    fn try_write_bytes_ne<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr>;
}

impl<T: TryWriteBytes> TypeCoded for Vec<T> {
    const TYPE_CODE: TypeCode = TypeCode::DynSize;
}

pub trait TypeCoded: Sized {
    const TYPE_CODE: TypeCode;
}

pub trait ByteSize: Sized {
    const BYTE_SIZE: usize;
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
        impl TypeCoded for $ty {
            const TYPE_CODE: TypeCode = TypeCode::$ty_code;
        }
        impl ByteSize for $ty {
            const BYTE_SIZE: usize = $size;
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
            fn try_read_bytes_le<R: std::io::Read>(
                stream: &mut R,
                checksum: &mut usize,
            ) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                *checksum += $size;
                Ok(<$ty>::from_le_bytes(buf))
            }
            fn try_read_bytes_be<R: std::io::Read>(
                stream: &mut R,
                checksum: &mut usize,
            ) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                *checksum += $size;
                Ok(<$ty>::from_be_bytes(buf))
            }
            fn try_read_bytes_ne<R: std::io::Read>(
                stream: &mut R,
                checksum: &mut usize,
            ) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                *checksum += $size;
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
            ) -> Result<usize, BytesErr> {
                Ok(stream.write(&self.to_le_bytes())?)
            }
            fn try_write_bytes_be<W: std::io::Write>(
                &self,
                stream: &mut W,
            ) -> Result<usize, BytesErr> {
                Ok(stream.write(&self.to_be_bytes())?)
            }
            fn try_write_bytes_ne<W: std::io::Write>(
                &self,
                stream: &mut W,
            ) -> Result<usize, BytesErr> {
                Ok(stream.write(&self.to_ne_bytes())?)
            }
        }
    };
    ($ty: ty, $override: ty, $size: literal, $ty_code: ident) => {
        impl TypeCoded for $ty {
            const TYPE_CODE: TypeCode = TypeCode::$ty_code;
        }
        impl ByteSize for $ty {
            const BYTE_SIZE: usize = $size;
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
            ) -> Result<usize, BytesErr> {
                Ok(stream.write(&(*self as $override).to_le_bytes())?)
            }
            fn try_write_bytes_be<W: std::io::Write>(
                &self,
                stream: &mut W,
            ) -> Result<usize, BytesErr> {
                Ok(stream.write(&(*self as $override).to_be_bytes())?)
            }
            fn try_write_bytes_ne<W: std::io::Write>(
                &self,
                stream: &mut W,
            ) -> Result<usize, BytesErr> {
                Ok(stream.write(&(*self as $override).to_ne_bytes())?)
            }
        }
        impl TryReadBytes for $ty {
            fn try_read_bytes_le<R: std::io::Read>(
                stream: &mut R,
                checksum: &mut usize,
            ) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                *checksum += $size;
                Ok(<$override>::from_le_bytes(buf) as $ty)
            }
            fn try_read_bytes_be<R: std::io::Read>(
                stream: &mut R,
                checksum: &mut usize,
            ) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                *checksum += $size;
                Ok(<$override>::from_be_bytes(buf) as $ty)
            }
            fn try_read_bytes_ne<R: std::io::Read>(
                stream: &mut R,
                checksum: &mut usize,
            ) -> Result<Self, BytesErr> {
                let mut buf = [0u8; $size];
                stream.read_exact(&mut buf)?;
                *checksum += $size;
                Ok(<$override>::from_ne_bytes(buf) as $ty)
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
// TODO: this needs try_fit checks as a u64 may not always fit into a usize
impl_byte_size!(usize, u64, 8, USIZE);
impl_byte_size!(isize, i64, 8, ISIZE);

impl TypeCoded for bool {
    const TYPE_CODE: TypeCode = TypeCode::BOOL;
}
impl ByteSize for bool {
    const BYTE_SIZE: usize = 1;
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

macro_rules! impl_trb_bool {
    ($fn_name: ident) => {
        fn $fn_name<R: std::io::Read>(
            stream: &mut R,
            checksum: &mut usize,
        ) -> Result<Self, BytesErr> {
            let mut buf = [0u8; 1];
            stream.read_exact(&mut buf)?;
            *checksum += 1;
            Ok(buf[0] == 1)
        }
    };
}

impl TryReadBytes for bool {
    impl_trb_bool!(try_read_bytes_le);
    impl_trb_bool!(try_read_bytes_be);
    impl_trb_bool!(try_read_bytes_ne);
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

pub trait TryWriteDynBytes: Sized {
    fn try_write_bytes_le<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr>;
    fn try_write_bytes_be<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr>;
    fn try_write_bytes_ne<W: std::io::Write>(&self, stream: &mut W) -> Result<(), BytesErr>;
}

pub struct SizedBytes;
pub struct DynBytes;
pub trait ByteType {}
impl ByteType for SizedBytes {}
impl ByteType for DynBytes {}

pub trait WriteByteStream<T: ByteType>: Sized {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr>;
    fn write_byte_stream_be<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr>;
    fn write_byte_stream_ne<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr>;
}

pub trait ReadByteStream<T: ByteType>: Sized {
    fn read_byte_stream_le<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr>;
    fn read_byte_stream_be<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr>;
    fn read_byte_stream_ne<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr>;
}

macro_rules! impl_wbs_tt {
    ($fn_name: ident, $try_name: ident) => {
        fn $fn_name<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
            stream.write_all(&[TT::TYPE_CODE.as_u8()])?;
            Ok(self.$try_name(stream)? + 1)
        }
    };
}

impl<TT> WriteByteStream<SizedBytes> for TT
where
    TT: TryWriteBytes + TypeCoded,
{
    impl_wbs_tt!(write_byte_stream_le, try_write_bytes_le);
    impl_wbs_tt!(write_byte_stream_be, try_write_bytes_be);
    impl_wbs_tt!(write_byte_stream_ne, try_write_bytes_ne);
}

macro_rules! impl_rbs_tt {
    ($fn_name: ident, $expect_name: ident, $try_name: ident) => {
        fn $fn_name<R: std::io::Read>(
            stream: &mut R,
            checksum: &mut usize,
        ) -> Result<Self, BytesErr> {
            TT::TYPE_CODE.$expect_name(stream, checksum)?;
            Ok(<TT>::$try_name(stream, checksum)?)
        }
    };
}

impl<TT> ReadByteStream<SizedBytes> for TT
where
    TT: TryReadBytes + TypeCoded,
{
    impl_rbs_tt!(
        read_byte_stream_le,
        expect_from_stream_le,
        try_read_bytes_le
    );
    impl_rbs_tt!(
        read_byte_stream_be,
        expect_from_stream_be,
        try_read_bytes_be
    );
    impl_rbs_tt!(
        read_byte_stream_ne,
        expect_from_stream_ne,
        try_read_bytes_ne
    );
}

macro_rules! impl_wbss_vec_tt {
    ($fn_name: ident, $trb_name: ident) => {
        fn $fn_name<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
            TypeCode::DynSize.as_u8().$trb_name(stream)?;
            TT::TYPE_CODE.as_u8().$trb_name(stream)?;
            self.len().$trb_name(stream)?;
            let mut bytes_written = 10;
            for item in self {
                bytes_written += item.$trb_name(stream)?;
            }
            Ok(bytes_written)
        }
    };
}

impl<TT> WriteByteStream<SizedBytes> for Vec<TT>
where
    TT: TryWriteBytes + TypeCoded,
{
    impl_wbss_vec_tt!(write_byte_stream_le, try_write_bytes_le);
    impl_wbss_vec_tt!(write_byte_stream_be, try_write_bytes_be);
    impl_wbss_vec_tt!(write_byte_stream_ne, try_write_bytes_ne);
}

macro_rules! impl_rbss_vec_tt {
    ($fn_name: ident, $expect: ident, $trb_name: ident) => {
        fn $fn_name<R: std::io::Read>(
            stream: &mut R,
            checksum: &mut usize,
        ) -> Result<Self, BytesErr> {
            TypeCode::DynSize.$expect(stream, checksum)?;
            TT::TYPE_CODE.$expect(stream, checksum)?;
            let mut entries = Vec::with_capacity(<usize>::$trb_name(stream, checksum)?);
            for _ in 0..entries.capacity() {
                entries.push(<TT>::$trb_name(stream, checksum)?);
            }
            Ok(entries)
        }
    };
}

impl<TT> ReadByteStream<SizedBytes> for Vec<TT>
where
    TT: TryReadBytes + TypeCoded,
{
    impl_rbss_vec_tt!(
        read_byte_stream_le,
        expect_from_stream_le,
        try_read_bytes_le
    );
    impl_rbss_vec_tt!(
        read_byte_stream_be,
        expect_from_stream_be,
        try_read_bytes_be
    );
    impl_rbss_vec_tt!(
        read_byte_stream_ne,
        expect_from_stream_ne,
        try_read_bytes_ne
    );
}

macro_rules! impl_wbsd_vec_tt {
    ($fn_name: ident, $trb: ident, $wbs: ident) => {
        fn $fn_name<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
            TypeCode::DynSize.as_u8().$trb(stream)?;
            self.len().$trb(stream)?;
            let mut bytes_written = 9;
            for item in self {
                bytes_written += item.$wbs(stream)?;
            }
            Ok(bytes_written)
        }
    };
}

impl<TT> WriteByteStream<DynBytes> for Vec<TT>
where
    TT: WriteByteStream<DynBytes>,
{
    impl_wbsd_vec_tt!(
        write_byte_stream_le,
        try_write_bytes_le,
        write_byte_stream_le
    );
    impl_wbsd_vec_tt!(
        write_byte_stream_be,
        try_write_bytes_be,
        write_byte_stream_be
    );
    impl_wbsd_vec_tt!(
        write_byte_stream_ne,
        try_write_bytes_ne,
        write_byte_stream_ne
    );
}

impl<TT> ReadByteStream<DynBytes> for Vec<TT>
where
    TT: ReadByteStream<DynBytes>,
{
    fn read_byte_stream_le<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr> {
        TypeCode::DynSize.expect_from_stream_le(stream, checksum)?;
        let mut entries = Vec::with_capacity(<usize>::try_read_bytes_le(stream, checksum)?);
        for _ in 0..entries.capacity() {
            entries.push(<TT>::read_byte_stream_le(stream, checksum)?);
        }
        Ok(entries)
    }
    fn read_byte_stream_be<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr> {
        TypeCode::DynSize.expect_from_stream_be(stream, checksum)?;
        let mut entries = Vec::with_capacity(<usize>::try_read_bytes_be(stream, checksum)?);
        for _ in 0..entries.capacity() {
            entries.push(<TT>::read_byte_stream_be(stream, checksum)?);
        }
        Ok(entries)
    }
    fn read_byte_stream_ne<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr> {
        TypeCode::DynSize.expect_from_stream_ne(stream, checksum)?;
        let mut entries = Vec::with_capacity(<usize>::try_read_bytes_ne(stream, checksum)?);
        for _ in 0..entries.capacity() {
            entries.push(<TT>::read_byte_stream_ne(stream, checksum)?);
        }
        Ok(entries)
    }
}
impl WriteByteStream<DynBytes> for String {
    fn write_byte_stream_le<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
        TypeCode::DynSize.as_u8().try_write_bytes_le(stream)?;
        TypeCode::CHAR.as_u8().try_write_bytes_le(stream)?;
        let bytes = self.as_bytes();
        bytes.len().try_write_bytes_le(stream)?;
        stream.write_all(bytes)?;
        Ok(10 + bytes.len())
    }
    fn write_byte_stream_be<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
        TypeCode::DynSize.as_u8().try_write_bytes_be(stream)?;
        TypeCode::CHAR.as_u8().try_write_bytes_be(stream)?;
        let bytes = self.as_bytes();
        bytes.len().try_write_bytes_be(stream)?;
        stream.write_all(bytes)?;
        Ok(10 + bytes.len())
    }
    fn write_byte_stream_ne<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
        TypeCode::DynSize.as_u8().try_write_bytes_ne(stream)?;
        TypeCode::CHAR.as_u8().try_write_bytes_ne(stream)?;
        let bytes = self.as_bytes();
        bytes.len().try_write_bytes_ne(stream)?;
        stream.write_all(bytes)?;
        Ok(10 + bytes.len())
    }
}
impl ReadByteStream<DynBytes> for String {
    fn read_byte_stream_le<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr> {
        TypeCode::DynSize.expect_from_stream_le(stream, checksum)?;
        TypeCode::CHAR.expect_from_stream_le(stream, checksum)?;
        let mut entries = vec![0u8; <usize>::try_read_bytes_le(stream, checksum)?];
        stream.read_exact(entries.as_mut_slice())?;
        *checksum += entries.len();
        Ok(String::from_utf8(entries)?)
    }
    fn read_byte_stream_be<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr> {
        TypeCode::DynSize.expect_from_stream_be(stream, checksum)?;
        TypeCode::CHAR.expect_from_stream_be(stream, checksum)?;
        let mut entries = vec![0u8; <usize>::try_read_bytes_be(stream, checksum)?];
        stream.read_exact(entries.as_mut_slice())?;
        *checksum += entries.len();
        Ok(String::from_utf8(entries)?)
    }
    fn read_byte_stream_ne<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr> {
        TypeCode::DynSize.expect_from_stream_ne(stream, checksum)?;
        TypeCode::CHAR.expect_from_stream_ne(stream, checksum)?;
        let mut entries = vec![0u8; <usize>::try_read_bytes_ne(stream, checksum)?];
        stream.read_exact(entries.as_mut_slice())?;
        *checksum += entries.len();
        Ok(String::from_utf8(entries)?)
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
        let checksum_a = s.write_byte_stream_le(&mut stream).unwrap();
        stream.set_position(0);
        let mut checksum_b = 0;
        assert_eq!(
            s,
            <String>::read_byte_stream_le(&mut stream, &mut checksum_b).unwrap()
        );
        assert_eq!(checksum_a, checksum_b);
        let s = String::from("just a test");

        let mut stream = Cursor::new(Vec::<u8>::new());
        let checksum_a = s.write_byte_stream_be(&mut stream).unwrap();
        stream.set_position(0);
        let mut checksum_b = 0;
        assert_eq!(
            s,
            <String>::read_byte_stream_be(&mut stream, &mut checksum_b).unwrap()
        );
        assert_eq!(checksum_a, checksum_b);

        let s = String::from("just a test");
        let mut stream = Cursor::new(Vec::<u8>::new());
        let checksum_b = s.write_byte_stream_ne(&mut stream).unwrap();
        stream.set_position(0);
        let mut checksum_b = 0;
        assert_eq!(
            s,
            <String>::read_byte_stream_ne(&mut stream, &mut checksum_b).unwrap()
        );
        assert_eq!(checksum_a, checksum_b);
    }

    #[test]
    fn test_u8() {
        let mut stream = Cursor::new(Vec::<u8>::new());
        let checksum_a = 0u8.write_byte_stream_le(&mut stream).unwrap();
        stream.set_position(0);
        let mut checksum_b = 0;
        assert_eq!(
            0u8,
            <u8>::read_byte_stream_le(&mut stream, &mut checksum_b).unwrap()
        );
        assert_eq!(checksum_a, checksum_a);

        let mut stream = Cursor::new(Vec::<u8>::new());
        let checksum_a = 255u8.write_byte_stream_le(&mut stream).unwrap();
        stream.set_position(0);
        let mut checksum_b = 0;
        assert_eq!(
            255u8,
            <u8>::read_byte_stream_le(&mut stream, &mut checksum_b).unwrap()
        );
        assert_eq!(checksum_a, checksum_a);
    }

    #[test]
    fn test_vec() {
        let v = vec![0, 1, 1];
        let mut stream = Cursor::new(Vec::<u8>::new());
        let checksum_a = v.write_byte_stream_le(&mut stream).unwrap();
        stream.set_position(0);
        let mut checksum_b = 0;
        assert_eq!(
            v,
            <Vec<i32>>::read_byte_stream_le(&mut stream, &mut checksum_b).unwrap()
        );

        let v = vec!["asdf".to_string(), "a".to_string(), "bb".to_string()];
        let mut stream = Cursor::new(Vec::<u8>::new());
        let checksum_a = v.write_byte_stream_le(&mut stream).unwrap();
        stream.set_position(0);
        let mut checksum_b = 0;
        assert_eq!(
            v,
            <Vec<String>>::read_byte_stream_le(&mut stream, &mut checksum_b).unwrap()
        );
    }
}

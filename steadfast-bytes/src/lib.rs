use steadfast_crypt::SHA256;
use steadfast_time::UTC;
use steadfast_uuid::UUID;
pub trait FromBytes<T>: ByteSize {
    fn from_sf_le_bytes(bytes: T) -> Self;
    fn from_sf_be_bytes(bytes: T) -> Self;
    fn from_sf_ne_bytes(bytes: T) -> Self;
}

pub trait ToBytes<T>: ByteSize {
    fn to_sf_le_bytes(&self) -> T;
    fn to_sf_be_bytes(&self) -> T;
    fn to_sf_ne_bytes(&self) -> T;
}

pub trait ByteSize: Sized {
    const BYTE_SIZE: usize;
}

trait AsArraySelf<const N: usize> {
    fn as_self(self) -> [u8; N];
}

impl<const N: usize> AsArraySelf<N> for [u8; N] {
    fn as_self(self) -> [u8; N] {
        self
    }
}

macro_rules! impl_byte_size {
    ($ty:ty, $size: literal) => {
        impl ByteSize for $ty {
            const BYTE_SIZE: usize = $size;
        }

        impl<T> FromBytes<T> for $ty
        where
            T: AsArraySelf<$size>,
        {
            fn from_sf_le_bytes(bytes: T) -> Self {
                <$ty>::from_le_bytes(bytes.as_self())
            }
            fn from_sf_be_bytes(bytes: T) -> Self {
                <$ty>::from_be_bytes(bytes.as_self())
            }
            fn from_sf_ne_bytes(bytes: T) -> Self {
                <$ty>::from_ne_bytes(bytes.as_self())
            }
        }

        impl ToBytes<[u8; $size]> for $ty {
            fn to_sf_le_bytes(&self) -> [u8; $size] {
                self.to_le_bytes()
            }
            fn to_sf_be_bytes(&self) -> [u8; $size] {
                self.to_be_bytes()
            }
            fn to_sf_ne_bytes(&self) -> [u8; $size] {
                self.to_ne_bytes()
            }
        }
    };
}

impl_byte_size!(u8, 1);
impl_byte_size!(u16, 2);
impl_byte_size!(u32, 4);
impl_byte_size!(u64, 8);
impl_byte_size!(u128, 16);
impl_byte_size!(i8, 1);
impl_byte_size!(i16, 2);
impl_byte_size!(i32, 4);
impl_byte_size!(i64, 8);
impl_byte_size!(i128, 16);
impl ByteSize for UUID {
    const BYTE_SIZE: usize = 16;
}

impl<T> FromBytes<T> for UUID
where
    T: AsArraySelf<16>,
{
    fn from_sf_le_bytes(bytes: T) -> Self {
        UUID::from_u128(<u128>::from_le_bytes(bytes.as_self()))
    }
    fn from_sf_be_bytes(bytes: T) -> Self {
        UUID::from_u128(<u128>::from_be_bytes(bytes.as_self()))
    }
    fn from_sf_ne_bytes(bytes: T) -> Self {
        UUID::from_u128(<u128>::from_ne_bytes(bytes.as_self()))
    }
}

impl ToBytes<[u8; 16]> for UUID {
    fn to_sf_le_bytes(&self) -> [u8; 16] {
        self.0.to_le_bytes()
    }
    fn to_sf_be_bytes(&self) -> [u8; 16] {
        self.0.to_be_bytes()
    }
    fn to_sf_ne_bytes(&self) -> [u8; 16] {
        self.0.to_ne_bytes()
    }
}
impl ByteSize for SHA256 {
    const BYTE_SIZE: usize = 32;
}

impl<T> FromBytes<T> for SHA256
where
    T: AsArraySelf<32>,
{
    fn from_sf_le_bytes(bytes: T) -> Self {
        Self::from_raw(bytes.as_self().as_chunks::<4>().0.iter().enumerate().fold(
            [0; 8],
            |mut num, (i, chunk)| {
                num[i] = <u32>::from_le_bytes(*chunk);
                num
            },
        ))
    }
    fn from_sf_be_bytes(bytes: T) -> Self {
        Self::from_raw(bytes.as_self().as_chunks::<4>().0.iter().enumerate().fold(
            [0; 8],
            |mut num, (i, chunk)| {
                num[i] = <u32>::from_be_bytes(*chunk);
                num
            },
        ))
    }
    fn from_sf_ne_bytes(bytes: T) -> Self {
        Self::from_raw(bytes.as_self().as_chunks::<4>().0.iter().enumerate().fold(
            [0; 8],
            |mut num, (i, chunk)| {
                num[i] = <u32>::from_ne_bytes(*chunk);
                num
            },
        ))
    }
}

impl ToBytes<[u8; 32]> for SHA256 {
    fn to_sf_le_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(self.inner_bytes().iter().map(|num| num.to_le_bytes()))
            .for_each(|(chunk, num_chunk)| chunk.copy_from_slice(&num_chunk));
        bytes
    }
    fn to_sf_be_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(self.inner_bytes().iter().map(|num| num.to_be_bytes()))
            .for_each(|(chunk, num_chunk)| chunk.copy_from_slice(&num_chunk));
        bytes
    }
    fn to_sf_ne_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(self.inner_bytes().iter().map(|num| num.to_ne_bytes()))
            .for_each(|(chunk, num_chunk)| chunk.copy_from_slice(&num_chunk));
        bytes
    }
}

impl ByteSize for bool {
    const BYTE_SIZE: usize = 1;
}

impl<T> FromBytes<T> for bool
where
    T: AsArraySelf<1>,
{
    fn from_sf_le_bytes(bytes: T) -> Self {
        if <u8>::from_le_bytes(bytes.as_self()) == 0 {
            false
        } else {
            true
        }
    }
    fn from_sf_be_bytes(bytes: T) -> Self {
        if <u8>::from_be_bytes(bytes.as_self()) == 0 {
            false
        } else {
            true
        }
    }
    fn from_sf_ne_bytes(bytes: T) -> Self {
        if <u8>::from_ne_bytes(bytes.as_self()) == 0 {
            false
        } else {
            true
        }
    }
}

impl ToBytes<[u8; 1]> for bool {
    fn to_sf_le_bytes(&self) -> [u8; 1] {
        if *self {
            1u8.to_sf_le_bytes()
        } else {
            0u8.to_sf_le_bytes()
        }
    }
    fn to_sf_be_bytes(&self) -> [u8; 1] {
        if *self {
            1u8.to_sf_be_bytes()
        } else {
            0u8.to_sf_be_bytes()
        }
    }
    fn to_sf_ne_bytes(&self) -> [u8; 1] {
        if *self {
            1u8.to_sf_ne_bytes()
        } else {
            0u8.to_sf_ne_bytes()
        }
    }
}
impl ByteSize for char {
    const BYTE_SIZE: usize = 1;
}

impl<T> FromBytes<T> for char
where
    T: AsArraySelf<1>,
{
    fn from_sf_le_bytes(bytes: T) -> Self {
        <u8>::from_le_bytes(bytes.as_self()) as char
    }
    fn from_sf_be_bytes(bytes: T) -> Self {
        <u8>::from_be_bytes(bytes.as_self()) as char
    }
    fn from_sf_ne_bytes(bytes: T) -> Self {
        <u8>::from_ne_bytes(bytes.as_self()) as char
    }
}

impl ToBytes<[u8; 1]> for char {
    fn to_sf_le_bytes(&self) -> [u8; 1] {
        (*self as u8).to_le_bytes()
    }
    fn to_sf_be_bytes(&self) -> [u8; 1] {
        (*self as u8).to_be_bytes()
    }
    fn to_sf_ne_bytes(&self) -> [u8; 1] {
        (*self as u8).to_ne_bytes()
    }
}
impl ByteSize for usize {
    const BYTE_SIZE: usize = 8;
}

impl<T> FromBytes<T> for usize
where
    T: AsArraySelf<8>,
{
    fn from_sf_le_bytes(bytes: T) -> Self {
        <u64>::from_le_bytes(bytes.as_self()) as usize
    }
    fn from_sf_be_bytes(bytes: T) -> Self {
        <u64>::from_be_bytes(bytes.as_self()) as usize
    }
    fn from_sf_ne_bytes(bytes: T) -> Self {
        <u64>::from_ne_bytes(bytes.as_self()) as usize
    }
}

impl ToBytes<[u8; 8]> for usize {
    fn to_sf_le_bytes(&self) -> [u8; 8] {
        (*self as u64).to_le_bytes()
    }
    fn to_sf_be_bytes(&self) -> [u8; 8] {
        (*self as u64).to_be_bytes()
    }
    fn to_sf_ne_bytes(&self) -> [u8; 8] {
        (*self as u64).to_ne_bytes()
    }
}

impl ByteSize for isize {
    const BYTE_SIZE: usize = 8;
}

impl<T> FromBytes<T> for isize
where
    T: AsArraySelf<8>,
{
    fn from_sf_le_bytes(bytes: T) -> Self {
        <i64>::from_le_bytes(bytes.as_self()) as isize
    }
    fn from_sf_be_bytes(bytes: T) -> Self {
        <i64>::from_be_bytes(bytes.as_self()) as isize
    }
    fn from_sf_ne_bytes(bytes: T) -> Self {
        <i64>::from_ne_bytes(bytes.as_self()) as isize
    }
}

impl ToBytes<[u8; 8]> for isize {
    fn to_sf_le_bytes(&self) -> [u8; 8] {
        (*self as i64).to_le_bytes()
    }
    fn to_sf_be_bytes(&self) -> [u8; 8] {
        (*self as i64).to_be_bytes()
    }
    fn to_sf_ne_bytes(&self) -> [u8; 8] {
        (*self as i64).to_ne_bytes()
    }
}

impl ByteSize for f32 {
    const BYTE_SIZE: usize = 4;
}

impl<T> FromBytes<T> for f32
where
    T: AsArraySelf<4>,
{
    fn from_sf_le_bytes(bytes: T) -> Self {
        <f32>::from_le_bytes(bytes.as_self())
    }
    fn from_sf_be_bytes(bytes: T) -> Self {
        <f32>::from_be_bytes(bytes.as_self())
    }
    fn from_sf_ne_bytes(bytes: T) -> Self {
        <f32>::from_ne_bytes(bytes.as_self())
    }
}

impl ToBytes<[u8; 4]> for f32 {
    fn to_sf_le_bytes(&self) -> [u8; 4] {
        self.to_le_bytes()
    }
    fn to_sf_be_bytes(&self) -> [u8; 4] {
        self.to_be_bytes()
    }
    fn to_sf_ne_bytes(&self) -> [u8; 4] {
        self.to_ne_bytes()
    }
}

impl ByteSize for f64 {
    const BYTE_SIZE: usize = 8;
}

impl<T> FromBytes<T> for f64
where
    T: AsArraySelf<8>,
{
    fn from_sf_le_bytes(bytes: T) -> Self {
        <f64>::from_le_bytes(bytes.as_self())
    }
    fn from_sf_be_bytes(bytes: T) -> Self {
        <f64>::from_be_bytes(bytes.as_self())
    }
    fn from_sf_ne_bytes(bytes: T) -> Self {
        <f64>::from_ne_bytes(bytes.as_self())
    }
}

impl ToBytes<[u8; 8]> for f64 {
    fn to_sf_le_bytes(&self) -> [u8; 8] {
        self.to_le_bytes()
    }
    fn to_sf_be_bytes(&self) -> [u8; 8] {
        self.to_be_bytes()
    }
    fn to_sf_ne_bytes(&self) -> [u8; 8] {
        self.to_ne_bytes()
    }
}

impl ByteSize for UTC {
    const BYTE_SIZE: usize = 8;
}

impl<T> FromBytes<T> for UTC
where
    T: AsArraySelf<8>,
{
    fn from_sf_le_bytes(bytes: T) -> Self {
        UTC::from_unix_epoch_millis(<u64>::from_le_bytes(bytes.as_self()))
    }
    fn from_sf_be_bytes(bytes: T) -> Self {
        UTC::from_unix_epoch_millis(<u64>::from_be_bytes(bytes.as_self()))
    }
    fn from_sf_ne_bytes(bytes: T) -> Self {
        UTC::from_unix_epoch_millis(<u64>::from_ne_bytes(bytes.as_self()))
    }
}

impl ToBytes<[u8; 8]> for UTC {
    fn to_sf_le_bytes(&self) -> [u8; 8] {
        self.to_unix_epoch_millis().to_sf_le_bytes()
    }
    fn to_sf_be_bytes(&self) -> [u8; 8] {
        self.to_unix_epoch_millis().to_sf_be_bytes()
    }
    fn to_sf_ne_bytes(&self) -> [u8; 8] {
        self.to_unix_epoch_millis().to_sf_ne_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let t = 4u8;
        let bytes = t.to_sf_le_bytes();
        assert!(<u8>::from_sf_le_bytes(bytes) == t);
    }
}

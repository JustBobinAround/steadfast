use std::{cmp::Ordering, str::FromStr};
use steadfast_bytes::{
    AsArraySelf, ByteSize, BytesErr, FromBytes, ToBytes, TryReadBytes, TryWriteBytes, TypeCode,
    TypeCoded,
};
use steadfast_rand::Random;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UUID(pub u128);

impl From<UUID> for steadfast_time::UTC {
    fn from(value: UUID) -> Self {
        steadfast_time::UTC::from_unix_epoch_millis(value.extract_timestamp())
    }
}

impl UUID {
    pub fn as_u128(&self) -> u128 {
        self.0
    }

    pub fn from_u128(n: u128) -> Self {
        Self(n)
    }

    /// Encodes a table hash into UUID with the current unix timestamp
    ///
    /// ```text
    ///  0                   1                   2                   3
    ///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |                           unix_ts_ms                          |
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |          unix_ts_ms           |  ver  |0 0 0 0 0 0 0 0 0 0 0 0|
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |                          table_hash                           |
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |                          table_hash                           |
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// ```
    pub fn from_table_hash(table_hash: u64) -> Result<Self, ()> {
        let t_ms = Self::current_time()?;
        Ok(UUID::default().encode_time(t_ms).encode_id(table_hash))
    }

    /// Sets rand_b section of uuid, forces rand_a to b 0 and uuid version to be v7.
    pub fn encode_id(mut self, id: u64) -> Self {
        self.0 = self.0 | id as u128;
        self
    }

    fn current_time() -> Result<u64, ()> {
        use std::time::{SystemTime, UNIX_EPOCH};

        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ())?
            .as_millis() as u64)
    }

    /// See RFC 9562, section 5.7
    ///
    /// ```text
    ///  0 a                 1                   2                   3
    ///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |                           unix_ts_ms                          |
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |          unix_ts_ms           |  ver  |       rand_a          |
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |var|                        rand_b                             |
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// |                            rand_b                             |
    /// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /// ```
    /// See rand module to see how random nums are generated
    pub fn rand_v7() -> Result<Self, ()> {
        let t_ms = Self::current_time()?;
        let rand_a = <u16>::rand().map_err(|_| ())?;
        let version: u16 = 0x7 << 12;
        let top = ((version | rand_a) as u128) << 64;

        let bottom = <u64>::rand().map_err(|_| ())? as u128;

        Ok(UUID(top | bottom).encode_time(t_ms))
    }

    pub fn encode_time(mut self, t_ms: u64) -> Self {
        const MASK: u64 = 0xFFFF_FFFF_FFFF_0000;
        self.0 = ((t_ms & MASK) as u128) << 64 | self.0;
        self
    }

    pub fn extract_timestamp(&self) -> u64 {
        const MASK: u64 = 0xFFFF_FFFF_FFFF_0000;
        ((self.0 >> 64) as u64) & MASK
    }
}

impl Default for UUID {
    fn default() -> Self {
        UUID(0)
    }
}

/// See RFC 9562, section 4
///
/// # ABNF
/// ```text
/// UUID     = 4hexOctet "-"
///            2hexOctet "-"
///            2hexOctet "-"
///            2hexOctet "-"
///            6hexOctet
/// hexOctet = HEXDIG HEXDIG
/// DIGIT    = %x30-39
/// HEXDIG   = DIGIT / "A" / "B" / "C" / "D" / "E" / "F"
/// ```
impl std::fmt::Display for UUID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let top = (self.0 >> 64) as u64;
        let data_1 = (top >> 32) as u32;
        let data_2 = (top >> 16) as u16;
        let data_3 = top as u16;
        write!(f, "{:08x}-{:04x}-{:04x}-", data_1, data_2, data_3,)?;

        let bottom = (self.0 as u64).to_be_bytes();

        for b in bottom {
            write!(f, "{:02x}", b)?;
        }

        Ok(())
    }
}

/// See RFC 9562, section 4
///
/// # ABNF
/// ```text
/// UUID     = 4hexOctet "-"
///            2hexOctet "-"
///            2hexOctet "-"
///            2hexOctet "-"
///            6hexOctet
/// hexOctet = HEXDIG HEXDIG
/// DIGIT    = %x30-39
/// HEXDIG   = DIGIT / "A" / "B" / "C" / "D" / "E" / "F"
/// ```
impl FromStr for UUID {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 35 {
            return Err(());
        }

        fn is_dash(c: &str) -> Result<(), ()> {
            if c == "-" { Ok(()) } else { Err(()) }
        }

        let (s_data_1, remainder) = s.split_at(8);
        let (dash, remainder) = remainder.split_at(1);
        is_dash(dash)?; //should we even care about this check?
        let (s_data_2, remainder) = remainder.split_at(4);
        let (dash, remainder) = remainder.split_at(1);
        is_dash(dash)?; //should we even care about this check?
        let (s_data_3, remainder) = remainder.split_at(4);
        let (dash, remainder) = remainder.split_at(1);
        is_dash(dash)?; //should we even care about this check?
        let (s_data_4, _) = remainder.split_at(16);

        let data_1 = u32::from_str_radix(s_data_1, 16).map_err(|_| ())?;
        let data_2 = u16::from_str_radix(s_data_2, 16).map_err(|_| ())?;
        let data_3 = u16::from_str_radix(s_data_3, 16).map_err(|_| ())?;

        let mut data_4 = [0_u8; 8];
        for idx in 0..data_4.len() {
            let s = &s_data_4[idx * 2..(idx * 2) + 2];
            data_4[idx] = u8::from_str_radix(s, 16).map_err(|_| ())?;
        }
        let data_4 = <u64>::from_be_bytes(data_4) as u128;

        let top =
            ((((data_1 as u64) << 32) | ((data_2 as u64) << 16) | data_3 as u64) as u128) << 64;

        Ok(UUID(top | data_4))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_rand() {
        let uuid = UUID::rand_v7();
        assert!(uuid != Ok(UUID(0)));
    }

    #[test]
    fn test_uuid_encoding() {
        let uuid_a = UUID::rand_v7().unwrap();
        let uuid_b = UUID::from_str(&uuid_a.to_string()).unwrap();
        assert!(uuid_a == uuid_b);
    }

    #[test]
    fn test_time_encoding() {
        let t_ms = 12093472938478 & 0xFFFF_FFFF_FFFF_0000; // can only store 48 bits
        let uuid = UUID::default().encode_time(t_ms);
        assert_eq!(t_ms, uuid.extract_timestamp());
    }
}

impl ByteSize for UUID {
    const BYTE_SIZE: usize = 16;
}
impl TypeCoded for UUID {
    const TYPE_CODE: TypeCode = TypeCode::Extension(17);
}
impl TryReadBytes for UUID {
    fn try_read_bytes_le<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr> {
        Ok(Self::from_u128(<u128>::try_read_bytes_le(
            stream, checksum,
        )?))
    }
    fn try_read_bytes_be<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr> {
        Ok(Self::from_u128(<u128>::try_read_bytes_be(
            stream, checksum,
        )?))
    }
    fn try_read_bytes_ne<R: std::io::Read>(
        stream: &mut R,
        checksum: &mut usize,
    ) -> Result<Self, BytesErr> {
        Ok(Self::from_u128(<u128>::try_read_bytes_ne(
            stream, checksum,
        )?))
    }
}

impl<T> FromBytes<T> for UUID
where
    T: AsArraySelf<16>,
{
    fn from_bytes_le(bytes: T) -> Self {
        UUID::from_u128(<u128>::from_le_bytes(bytes.as_array_self()))
    }
    fn from_bytes_be(bytes: T) -> Self {
        UUID::from_u128(<u128>::from_be_bytes(bytes.as_array_self()))
    }
    fn from_bytes_ne(bytes: T) -> Self {
        UUID::from_u128(<u128>::from_ne_bytes(bytes.as_array_self()))
    }
}

impl TryWriteBytes for UUID {
    fn try_write_bytes_le<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
        Ok(stream.write(&self.0.to_le_bytes())?)
    }
    fn try_write_bytes_be<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
        Ok(stream.write(&self.0.to_be_bytes())?)
    }
    fn try_write_bytes_ne<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
        Ok(stream.write(&self.0.to_ne_bytes())?)
    }
}

impl ToBytes<[u8; 16]> for UUID {
    fn to_bytes_le(&self) -> [u8; 16] {
        self.0.to_le_bytes()
    }
    fn to_bytes_be(&self) -> [u8; 16] {
        self.0.to_be_bytes()
    }
    fn to_bytes_ne(&self) -> [u8; 16] {
        self.0.to_ne_bytes()
    }
}

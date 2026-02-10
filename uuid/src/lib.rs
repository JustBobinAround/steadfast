use rand::Random;
use std::{cmp::Ordering, str::FromStr};

#[repr(C)]
#[derive(Clone, Debug, Hash)]
pub struct UUID {
    pub data_1: u32,
    pub data_2: u16,
    pub data_3: u16,
    pub data_4: [u8; 8],
}

impl PartialEq for UUID {
    fn eq(&self, other: &Self) -> bool {
        self.data_1 == other.data_1
            && self.data_2 == other.data_2
            && self.data_3 == other.data_3
            && self.data_4 == other.data_4
    }
}

impl Eq for UUID {}

impl PartialOrd for UUID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Ordering is built around data_1 > data_2 > data_3 > data_4
///
/// This probably what rust does with derive, but I
/// want to make sure ord is guaranteed by comparing time first
/// for v7 uuids where data_1 & data_2 are time stamps
impl Ord for UUID {
    fn cmp(&self, other: &Self) -> Ordering {
        self.data_1
            .cmp(&other.data_1)
            .then_with(|| self.data_2.cmp(&other.data_2))
            .then_with(|| self.data_3.cmp(&other.data_3))
            .then_with(|| self.data_4.cmp(&other.data_4))
    }
}

impl UUID {
    pub fn new(data_1: u32, data_2: u16, data_3: u16, data_4: [u8; 8]) -> Self {
        UUID {
            data_1,
            data_2,
            data_3,
            data_4,
        }
    }

    pub fn as_u128(&self) -> u128 {
        let data_1 = (self.data_1 as u128) << 96;
        let data_2 = (self.data_2 as u128) << 80;
        let data_3 = (self.data_3 as u128) << 64;
        let data_4 = <u64>::from_le_bytes(self.data_4) as u128;

        data_1 | data_2 | data_3 | data_4
    }

    pub fn from_u128(n: u128) -> Self {
        UUID {
            data_1: (n >> 96) as u32,
            data_2: (n >> 80) as u16,
            data_3: (n >> 64) as u16,
            data_4: (n as u64).to_le_bytes(),
        }
    }

    pub fn as_token_string(&self) -> String {
        format!(
            "UUID::new({},{},{},[{}])",
            self.data_1,
            self.data_2,
            self.data_3,
            self.data_4
                .iter()
                .map(|s| format!("{},", s))
                .collect::<String>()
        )
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

    /// Decodes a table hash from a UUID. Version check must be done by user.
    pub fn as_table_hash(&self) -> u64 {
        u64::from_le_bytes(self.data_4)
    }

    /// Sets rand_b section of uuid, forces rand_a to b 0 and uuid version to be v7.
    pub fn encode_id(mut self, id: u64) -> Self {
        self.data_3 = 0x7 << 12;
        self.data_4 = id.to_le_bytes();

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
        let rand_a = u16::rand().map_err(|_| ())?;
        let version: u16 = 0x7 << 12;
        let data_3 = version | rand_a;

        let data_4 = <[u8; 8]>::rand().map_err(|_| ())?;
        // data_4[0] = 1;
        // data_4[1] = 0;

        Ok(UUID {
            data_1: 0,
            data_2: 0,
            data_3,
            data_4,
        }
        .encode_time(t_ms))
    }

    pub fn encode_time(mut self, t_ms: u64) -> Self {
        self.data_1 = (t_ms >> 16) as u32;
        self.data_2 = t_ms as u16;
        self
    }

    pub fn extract_timestamp(&self) -> u64 {
        ((self.data_1 as u64) << 16) | (self.data_2 as u64)
    }
}

impl Default for UUID {
    fn default() -> Self {
        UUID {
            data_1: 0,
            data_2: 0,
            data_3: 0,
            data_4: [0_u8; 8],
        }
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
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-",
            self.data_1, self.data_2, self.data_3,
        )?;

        for b in self.data_4 {
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

        Ok(UUID::new(data_1, data_2, data_3, data_4))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_rand() {
        let uuid = UUID::rand_v7();
        assert!(
            uuid != Ok(UUID {
                data_1: 0,
                data_2: 0,
                data_3: 0,
                data_4: [0, 0, 0, 0, 0, 0, 0, 0],
            })
        );
    }

    #[test]
    fn test_time_encoding() {
        let t_ms = 12093472938478;
        let uuid = UUID::rand_v7().unwrap().encode_time(t_ms);
        assert_eq!(t_ms, uuid.extract_timestamp());
    }
}

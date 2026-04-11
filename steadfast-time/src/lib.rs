use std::fmt::Display;
use std::marker::PhantomData;
use std::time::Duration;
use steadfast_bytes::{AsArraySelf, ByteSize, FromBytes, ToBytes, TypeCode};
pub enum TimeErrSF {
    FailedToFetch,
}
pub trait TimeZone {}
pub struct DateTime<TZ: TimeZone> {
    _tz: PhantomData<TZ>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct UTC {
    time_since_unix_epoch: Duration,
}

impl Display for UTC {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!("iso8601 formatting for UTC not implemented yet")
    }
}

impl UTC {
    pub fn new() -> Result<Self, TimeErrSF> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let time_since_unix_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| TimeErrSF::FailedToFetch)?;

        Ok(UTC {
            time_since_unix_epoch,
        })
    }

    pub fn from_unix_epoch_millis(millis: u64) -> Self {
        Self::from_unix_epoch_duration(Duration::from_millis(millis))
    }

    pub fn to_unix_epoch_millis(&self) -> u64 {
        self.time_since_unix_epoch.as_millis() as u64
    }

    pub fn from_unix_epoch_duration(duration: Duration) -> Self {
        UTC {
            time_since_unix_epoch: duration,
        }
    }
}

impl ByteSize for UTC {
    const BYTE_SIZE: usize = 8;
    const TYPE_CODE: TypeCode = TypeCode::Extension(18);
}

impl<T> FromBytes<T> for UTC
where
    T: AsArraySelf<8>,
{
    fn from_bytes_le(bytes: T) -> Self {
        UTC::from_unix_epoch_millis(<u64>::from_le_bytes(bytes.as_array_self()))
    }
    fn from_bytes_be(bytes: T) -> Self {
        UTC::from_unix_epoch_millis(<u64>::from_be_bytes(bytes.as_array_self()))
    }
    fn from_bytes_ne(bytes: T) -> Self {
        UTC::from_unix_epoch_millis(<u64>::from_ne_bytes(bytes.as_array_self()))
    }
}

impl ToBytes<[u8; 8]> for UTC {
    fn to_bytes_le(&self) -> [u8; 8] {
        self.to_unix_epoch_millis().to_le_bytes()
    }
    fn to_bytes_be(&self) -> [u8; 8] {
        self.to_unix_epoch_millis().to_be_bytes()
    }
    fn to_bytes_ne(&self) -> [u8; 8] {
        self.to_unix_epoch_millis().to_ne_bytes()
    }
}

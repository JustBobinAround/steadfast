use std::fmt::Display;
use std::marker::PhantomData;
use std::time::Duration;
use steadfast_bytes::{
    AsArraySelf, ByteSize, BytesErr, FromBytes, ToBytes, TryReadBytes, TryWriteBytes, TypeCode,
    TypeCoded,
};
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
}
impl TypeCoded for UTC {
    const TYPE_CODE: TypeCode = TypeCode::Extension(18);
}

macro_rules! impl_trb_utc {
    ($fn_name: ident, $trb: ident) => {
        fn $fn_name<R: std::io::Read>(
            stream: &mut R,
            checksum: &mut usize,
        ) -> Result<Self, BytesErr> {
            Ok(UTC::from_unix_epoch_millis(<u64>::$trb(stream, checksum)?))
        }
    };
}

impl TryReadBytes for UTC {
    impl_trb_utc!(try_read_bytes_le, try_read_bytes_le);
    impl_trb_utc!(try_read_bytes_be, try_read_bytes_be);
    impl_trb_utc!(try_read_bytes_ne, try_read_bytes_ne);
}

macro_rules! impl_twb_utc {
    ($fn_name: ident, $twb: ident) => {
        fn $fn_name<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
            Ok(stream.write(&self.to_unix_epoch_millis().$twb())?)
        }
    };
}

impl TryWriteBytes for UTC {
    impl_twb_utc!(try_write_bytes_le, to_le_bytes);
    impl_twb_utc!(try_write_bytes_be, to_be_bytes);
    impl_twb_utc!(try_write_bytes_ne, to_ne_bytes);
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

use std::fmt::Display;
use std::marker::PhantomData;
use std::time::Duration;
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

    pub fn to_le_bytes(&self) -> [u8; 8] {
        (self.time_since_unix_epoch.as_millis() as u64).to_le_bytes()
    }

    pub fn from_le_bytes(bytes: [u8; 8]) -> Self {
        let millis = <u64>::from_le_bytes(bytes);
        Self::from_unix_epoch_millis(millis)
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

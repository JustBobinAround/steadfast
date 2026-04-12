use std::io::Read;

#[derive(Debug)]
pub enum RandErr {
    IoError(std::io::Error),
}

impl From<std::io::Error> for RandErr {
    fn from(value: std::io::Error) -> Self {
        RandErr::IoError(value)
    }
}

fn entropy(b: &mut [u8]) -> Result<(), RandErr> {
    std::fs::File::open("/dev/urandom")?.read_exact(b)?;
    Ok(())
}

pub trait Random: Sized {
    fn rand() -> Result<Self, RandErr>;
}

macro_rules! impl_random {
    ($t: ty, $bytes: literal) => {
        impl Random for $t {
            fn rand() -> Result<Self, RandErr> {
                let mut bytes: [u8; $bytes] = [0; $bytes];
                entropy(&mut bytes)?;
                Ok(<$t>::from_be_bytes(bytes))
            }
        }
    };
}

impl_random!(u8, 1);
impl_random!(u16, 2);
impl_random!(u32, 4);
impl_random!(u64, 8);
impl_random!(usize, 8);
impl_random!(u128, 16);
impl_random!(i8, 1);
impl_random!(i16, 2);
impl_random!(i32, 4);
impl_random!(i64, 8);
impl_random!(isize, 8);
impl_random!(i128, 16);

impl<const N: usize> Random for [u8; N] {
    fn rand() -> Result<Self, RandErr> {
        let mut bytes = [0u8; N];
        entropy(&mut bytes)?;
        Ok(bytes)
    }
}

impl<const N: usize> Random for [i8; N] {
    fn rand() -> Result<Self, RandErr> {
        let mut bytes = [0u8; N];
        entropy(&mut bytes)?;

        Ok(bytes.map(|b| b as i8))
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_entropy() {
//         let mut buf = [0, 0, 0, 0];
//         entropy(&mut buf);
//     }
// }

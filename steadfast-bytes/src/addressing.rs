use super::{FromBytes, ToBytes};
use std::io::SeekFrom;

#[repr(transparent)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageAddr<const PAGE_SIZE: usize>(u64);
impl<const PAGE_SIZE: usize> PageAddr<PAGE_SIZE> {
    pub fn new(page_addr: u64) -> Self {
        assert!(page_addr % PAGE_SIZE as u64 == 0);
        Self(page_addr)
    }

    pub fn from_mem_addr(mem_addr: u64) -> Self {
        Self::new((mem_addr / PAGE_SIZE as u64) * PAGE_SIZE as u64)
    }

    pub fn into_inner(self) -> u64 {
        self.0
    }
}

impl<const PAGE_SIZE: usize> Into<SeekFrom> for PageAddr<PAGE_SIZE> {
    fn into(self) -> SeekFrom {
        SeekFrom::Start(self.0)
    }
}

impl<const PAGE_SIZE: usize> ToBytes<[u8; 8]> for PageAddr<PAGE_SIZE> {
    fn to_bytes_le(&self) -> [u8; 8] {
        self.0.to_le_bytes()
    }
    fn to_bytes_be(&self) -> [u8; 8] {
        self.0.to_be_bytes()
    }
    fn to_bytes_ne(&self) -> [u8; 8] {
        self.0.to_ne_bytes()
    }
}

impl<const PAGE_SIZE: usize> FromBytes<[u8; 8]> for PageAddr<PAGE_SIZE> {
    fn from_bytes_le(bytes: [u8; 8]) -> Self {
        Self::new(<u64>::from_le_bytes(bytes))
    }
    fn from_bytes_be(bytes: [u8; 8]) -> Self {
        Self::new(<u64>::from_be_bytes(bytes))
    }
    fn from_bytes_ne(bytes: [u8; 8]) -> Self {
        Self::new(<u64>::from_ne_bytes(bytes))
    }
}

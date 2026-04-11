use crate::tables::STable;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    path::Path,
};
use steadfast_bytes::FromBytes;
use steadfast_crypt::SHA256;

pub struct FieldKey(SHA256);

#[derive(Debug)]
pub enum FieldMapErr {
    FailedToOpen,
    FailedToRead,
    EOF,
}

#[derive(Debug)]
pub struct FieldMap<'a, const PAGE_SIZE: usize> {
    file: &'a mut File,
    entries: Vec<(u64, SHA256)>,
}

impl<'a, const PAGE_SIZE: usize> FieldMap<'a, PAGE_SIZE> {
    const COMPTIME_SIZE_CHECK: () = assert!(
        PAGE_SIZE >= 64 && (PAGE_SIZE - 1 < (PAGE_SIZE ^ (PAGE_SIZE - 1))),
        "PAGE_SIZE must be greater than 64 and a power of 2."
    );
    const ADDR_OFFSET: usize = 1;
    const INIT_CAPACITY: usize = 256;

    pub fn open_map_file(file_path: &str) -> Result<File, FieldMapErr> {
        let _ = Self::COMPTIME_SIZE_CHECK;
        let map_path = Path::new(file_path);

        OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&map_path)
            .map_err(|_| FieldMapErr::FailedToOpen)
    }

    fn read_next_file_entry(&mut self) -> Result<(u64, SHA256), FieldMapErr> {
        let mem_addr_buf = self.read_exact::<8>()?;
        if mem_addr_buf[0] == 0 {
            self.file
                .seek(SeekFrom::Current(32))
                .map_err(|_| FieldMapErr::FailedToRead)?;
            return Ok((0, SHA256::default()));
        }
        let mem_addr = <u64>::from_le_bytes(mem_addr_buf);
        let sha256_buf = self.read_exact::<32>()?;
        <SHA256>::from_bytes_le(sha256_buf);
        todo!()
    }

    pub fn init_entries(mut self) -> Result<Self, FieldMapErr> {
        let capacity_buf = self.read_exact::<8>()?;
        let capacity = <u64>::from_le_bytes(capacity_buf);
        if capacity != 0 {
            self.entries = Vec::with_capacity(capacity as usize);
        } else {
            self.entries = Vec::with_capacity(Self::INIT_CAPACITY);
            self.entries
                .resize(Self::INIT_CAPACITY, (0, SHA256::default()));
        }

        Ok(self)
    }
    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], FieldMapErr> {
        let mut buf = [0u8; N];
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|_| FieldMapErr::FailedToRead)?;
        self.file
            .read_exact(&mut buf)
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::UnexpectedEof => FieldMapErr::EOF,
                _ => FieldMapErr::FailedToRead,
            })?;

        Ok(buf)
    }
    pub fn new(file: &'a mut File) -> Result<Self, FieldMapErr> {
        let _ = Self::COMPTIME_SIZE_CHECK;
        FieldMap {
            file,
            entries: Vec::with_capacity(0),
        }
        .init_entries()
    }
}

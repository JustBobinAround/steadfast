use crate::page_addr::PageAddr;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};
use steadfast_bytes::{
    ByteSize, BytesErr, ReadByteStream, SizedBytes, TryReadBytes, TryWriteBytes, TypeCode,
    TypeCoded, WriteByteStream,
};
use steadfast_crypt::SHA256;

#[derive(Debug)]
pub enum FieldMapErr {
    BytesErr(BytesErr),
    IoError(std::io::Error),
}

impl From<std::io::Error> for FieldMapErr {
    fn from(err: std::io::Error) -> Self {
        FieldMapErr::IoError(err)
    }
}

impl From<BytesErr> for FieldMapErr {
    fn from(err: BytesErr) -> Self {
        FieldMapErr::BytesErr(err)
    }
}

#[derive(Debug, Clone)]
pub struct FieldEntry {
    mem_addr: u64,
    field_id: SHA256,
}

impl FieldEntry {
    const EMPTY_ADDR: u64 = 0;
}

impl ByteSize for FieldEntry {
    const BYTE_SIZE: usize = u64::BYTE_SIZE + SHA256::BYTE_SIZE;
}

impl TypeCoded for FieldEntry {
    const TYPE_CODE: TypeCode = TypeCode::Extension(20);
}

macro_rules! impl_trb_field_entry {
    ($fn_name: ident) => {
        fn $fn_name<R: std::io::Read>(
            stream: &mut R,
            checksum: &mut usize,
        ) -> Result<Self, steadfast_bytes::BytesErr> {
            Ok(Self {
                mem_addr: <u64>::$fn_name(stream, checksum)?,
                field_id: <SHA256>::$fn_name(stream, checksum)?,
            })
        }
    };
}

impl TryReadBytes for FieldEntry {
    impl_trb_field_entry!(try_read_bytes_le);
    impl_trb_field_entry!(try_read_bytes_be);
    impl_trb_field_entry!(try_read_bytes_ne);
}

macro_rules! impl_twb_field_entry {
    ($fn_name: ident) => {
        fn $fn_name<W: std::io::Write>(
            &self,
            stream: &mut W,
        ) -> Result<usize, steadfast_bytes::BytesErr> {
            Ok(self.mem_addr.$fn_name(stream)? + self.field_id.$fn_name(stream)?)
        }
    };
}

impl TryWriteBytes for FieldEntry {
    impl_twb_field_entry!(try_write_bytes_le);
    impl_twb_field_entry!(try_write_bytes_be);
    impl_twb_field_entry!(try_write_bytes_ne);
}

impl Default for FieldEntry {
    fn default() -> Self {
        FieldEntry {
            mem_addr: 0,
            field_id: SHA256::default(),
        }
    }
}

#[derive(Debug)]
pub struct FieldMap<'a, const PAGE_SIZE: usize, T: Read + Write + Seek> {
    stream: &'a mut T,
    entries: Vec<FieldEntry>,
}

macro_rules! impl_wbss_field_map {
    ($fn_name: ident) => {
        fn $fn_name<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
            self.entries.$fn_name(stream)
        }
    };
}

impl<'a, const PAGE_SIZE: usize, T: Read + Write + Seek> WriteByteStream<SizedBytes>
    for FieldMap<'a, PAGE_SIZE, T>
{
    impl_wbss_field_map!(write_byte_stream_le);
    impl_wbss_field_map!(write_byte_stream_be);
    impl_wbss_field_map!(write_byte_stream_ne);
}

impl<'a, const PAGE_SIZE: usize, T: Read + Write + Seek> FieldMap<'a, PAGE_SIZE, T> {
    /// Initial capacity of the field map for new files/buffers
    const INIT_CAPACITY: usize = 256;
    /// Allows FieldEntry::EMPTY_ADDR (aka 0) to be used as an empty entry marker
    const MEM_OFFSET: u64 = 1;

    /// Helper function to open a file with the correct flags to be used with FieldMap
    pub fn open_map_file(file_path: &str) -> Result<File, FieldMapErr> {
        // let _ = Self::COMPTIME_SIZE_CHECK;
        let map_path = Path::new(file_path);

        Ok(OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&map_path)?)
    }

    /// Creates a new FieldMap. Initially allocs with room for 256 field entries.
    pub fn new(stream: &'a mut T) -> Result<Self, FieldMapErr> {
        FieldMap {
            stream,
            entries: Vec::with_capacity(0),
        }
        .init_entries()
    }

    fn init_entries(mut self) -> Result<Self, FieldMapErr> {
        let file_len = self.stream.seek(SeekFrom::End(0))?;
        if file_len == 0 {
            self.entries = Vec::with_capacity(Self::INIT_CAPACITY);
            self.entries
                .resize_with(Self::INIT_CAPACITY, FieldEntry::default);
            self.entries.write_byte_stream_le(self.stream)?;
        } else {
            self.stream.seek(SeekFrom::Start(0))?;
            let mut checksum = 0;
            self.entries = <Vec<FieldEntry>>::read_byte_stream_le(self.stream, &mut checksum)?;
            BytesErr::compare_checksums(file_len as usize, checksum)?;
        }

        Ok(self)
    }

    fn resize(&mut self, mem_addr: u64, field_id: SHA256) -> Result<(), FieldMapErr> {
        let mut old_entries = Vec::with_capacity(self.entries.capacity() * 2);
        old_entries.resize_with(old_entries.capacity(), FieldEntry::default);
        self.stream.seek(SeekFrom::Start(0))?;
        old_entries.write_byte_stream_le(self.stream)?;

        std::mem::swap(&mut self.entries, &mut old_entries);

        for FieldEntry { mem_addr, field_id } in old_entries {
            if mem_addr != FieldEntry::EMPTY_ADDR {
                // Need to reinsert with out the offset to avoid cyclic mem shifting
                self.insert(field_id, mem_addr - Self::MEM_OFFSET)?;
            }
        }

        self.insert(field_id, mem_addr)?;

        Ok(())
    }

    pub fn insert(&mut self, field_id: SHA256, mem_addr: u64) -> Result<(), FieldMapErr> {
        const VEC_HEADER_LEN: u64 = 10;
        let mut idx = field_id.inner_bytes()[0] as usize % self.entries.capacity();
        while idx < self.entries.len() && self.entries[idx].mem_addr != FieldEntry::EMPTY_ADDR {
            idx += 1;
        }

        if idx == self.entries.len() {
            self.resize(mem_addr, field_id)?;
        } else {
            self.entries[idx] = FieldEntry {
                mem_addr: mem_addr + Self::MEM_OFFSET,
                field_id,
            };
            self.stream.seek(SeekFrom::Start(
                VEC_HEADER_LEN + (idx * FieldEntry::BYTE_SIZE) as u64,
            ))?;
            self.entries[idx].try_write_bytes_le(self.stream)?;
        }

        Ok(())
    }

    pub fn get(&self, field_id: &SHA256) -> Option<PageAddr<PAGE_SIZE>> {
        let mut idx = field_id.inner_bytes()[0] as usize % self.entries.capacity();
        while idx < self.entries.len() && self.entries[idx].mem_addr != FieldEntry::EMPTY_ADDR {
            if &self.entries[idx].field_id == field_id {
                return Some(PageAddr::new(self.entries[idx].mem_addr - Self::MEM_OFFSET));
            }
            idx += 1;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mass_resize_and_get() {
        let mut c = std::io::Cursor::new(Vec::new());
        let mut map = FieldMap::<64, _>::new(&mut c).unwrap();

        for i in 0..5000 {
            map.insert(
                SHA256::from_raw([i as u32 * 8, 100, 100, 100, 100, 100, 100, i as u32]),
                i * 64,
            )
            .unwrap();
        }
        c.set_position(0);

        let map = FieldMap::<64, _>::new(&mut c).unwrap();
        assert_eq!(
            19200,
            map.get(&SHA256::from_raw([
                300 * 8,
                100,
                100,
                100,
                100,
                100,
                100,
                300
            ]))
            .unwrap()
            .into_inner()
        );
    }
}

mod address_entry;
mod b_tree;
mod field_map;
pub mod optional_ord;
mod page_addr;
mod tables;
use crate::optional_ord::FieldOrd;
pub use crate::{
    address_entry::AddressEntry,
    b_tree::{BTreeIndex, IndexErr},
    field_map::{FieldMap, FieldMapErr},
    page_addr::PageAddr,
    tables::{STable, TableRecord},
};
use std::{
    any::Any,
    cmp::Ordering,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};
use steadfast_bytes::{ByteSize, BytesErr, ReadByteStream, WriteByteStream};

// use steadfast_serializer::DataHolder;
use steadfast_uuid::UUID;

#[derive(Debug)]
pub enum DatabaseErr {
    EOF,
    TimeWentBackwards,
    FailedToWrite,
    FailedToRead,
    FailedToOpen,
    NoEntryFound,
    InvalidTableType,
    FailedToDeserialize,
    IndexErr(IndexErr),
    FieldMapErr(FieldMapErr),
    BytesErr(BytesErr),
    IoError(std::io::Error),
}

impl From<FieldMapErr> for DatabaseErr {
    fn from(value: FieldMapErr) -> Self {
        Self::FieldMapErr(value)
    }
}

impl From<BytesErr> for DatabaseErr {
    fn from(value: BytesErr) -> Self {
        Self::BytesErr(value)
    }
}

impl From<std::io::Error> for DatabaseErr {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<IndexErr> for DatabaseErr {
    fn from(value: IndexErr) -> Self {
        Self::IndexErr(value)
    }
}

#[derive(Debug)]
pub struct Database<'a, const PAGE_SIZE: usize, T: Read + Write + Seek> {
    db_file: &'a mut T,
    file_size: usize,
    read_offset: usize,
    index: BTreeIndex<'a, PAGE_SIZE, T>,
    field_map: FieldMap<'a, PAGE_SIZE, T>,
}

impl<'a, const PAGE_SIZE: usize, T: Read + Write + Seek> Database<'a, PAGE_SIZE, T> {
    pub fn open_db_file(path: &str) -> Result<File, DatabaseErr> {
        let db_path = Path::new(path);

        OpenOptions::new()
            .append(true)
            .read(true)
            .create(true)
            .open(&db_path)
            .map_err(|_| DatabaseErr::FailedToOpen)
    }

    const UUID_BSIZE: usize = 16;
    const DATA_LEN_BSIZE: usize = 8;
    const LAST_UPDATE_BSIZE: usize = 8;
    const ENTRY_HEADER_OFFSET: usize =
        Self::UUID_BSIZE + Self::DATA_LEN_BSIZE + Self::LAST_UPDATE_BSIZE;

    pub fn new(
        db_file: &'a mut T,
        map_file: &'a mut T,
        idx_file: &'a mut T,
    ) -> Result<Self, DatabaseErr> {
        // let index = HashMap::new();
        let file_size = 0;
        let read_offset = 0;

        Ok(Database {
            db_file,
            file_size,
            read_offset,
            index: BTreeIndex::new(idx_file)?, // index,
            field_map: FieldMap::new(map_file)?,
        })
    }

    pub fn read_record<TT: STable>(
        &mut self,
        mem_addr: u64,
    ) -> Result<TableRecord<TT>, DatabaseErr> {
        self.db_file.seek(SeekFrom::Start(mem_addr))?;
        let mut _checksum = 0;
        Ok(TableRecord::<TT>::read_byte_stream_le(
            self.db_file,
            &mut _checksum,
        )?)
    }

    pub fn index_record<TT: STable>(
        &mut self,
        mem_addr: u64,
        tr: TableRecord<TT>,
    ) -> Result<(), DatabaseErr> {
        for (_field_name, field_id) in TableRecord::<TT>::indexed_fields() {
            match self.field_map.get(field_id) {
                Some(field_page_addr) => {
                    self.index
                        .insert(field_page_addr, *tr.sys_uuid(), todo!())?
                }
                None => {}
            }
        }
        Ok(())
    }

    pub fn fetch_first_record_eq<TT: STable>(
        &mut self,
        field_name: &str,
        val: &dyn Any,
    ) -> Result<TableRecord<TT>, DatabaseErr> {
        const STRONG_TY_U64_OFFSET: i64 = -1 * (<u64>::BYTE_SIZE as i64 + 1);
        let mut current_offset = self.db_file.seek(SeekFrom::End(0))?;
        while current_offset > 0 {
            current_offset = self.db_file.seek(SeekFrom::Current(STRONG_TY_U64_OFFSET))?;
            let mut checksum = 0;
            let tr_byte_len = <u64>::read_byte_stream_le(self.db_file, &mut checksum)?;
            current_offset -= tr_byte_len + checksum as u64;
            self.db_file.seek(SeekFrom::Start(current_offset))?;
            checksum = 0;
            match <TableRecord<TT>>::read_byte_stream_le(self.db_file, &mut checksum) {
                Ok(tr) => match tr.cmp_with_field(field_name, val) {
                    Some(Ordering::Equal) => {}
                    _ => {}
                },
                Err(e) => {}
            }
            checksum = 0;
        }

        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
}

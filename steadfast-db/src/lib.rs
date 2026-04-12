mod address_entry;
mod b_tree;
mod field_map;
mod tables;
pub use crate::{
    address_entry::AddressEntry,
    b_tree::{BTreeIndex, IndexErr},
    field_map::{FieldMap, FieldMapErr},
    tables::STable,
};
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};
use steadfast_bytes::ToBytes;
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
}

impl From<FieldMapErr> for DatabaseErr {
    fn from(value: FieldMapErr) -> Self {
        Self::FieldMapErr(value)
    }
}

impl From<IndexErr> for DatabaseErr {
    fn from(value: IndexErr) -> Self {
        Self::IndexErr(value)
    }
}

#[derive(Debug)]
pub struct Database<'a, T: Read + Write + Seek> {
    _db_file: &'a File,
    writer: BufWriter<&'a File>,
    reader: BufReader<&'a File>,
    file_size: usize,
    read_offset: usize,
    index: BTreeIndex<'a, 4096, T>,
    // field_map: FieldMap<'a, 4096>,
}

impl<'a, T: Read + Write + Seek> Database<'a, T> {
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
        db_file: &'a File,
        map_file: &'a mut File,
        idx_file: &'a mut T,
    ) -> Result<Self, DatabaseErr> {
        let writer = BufWriter::new(db_file);
        let reader = BufReader::new(db_file);
        // let index = HashMap::new();
        let file_size = 0;
        let read_offset = 0;

        Ok(Database {
            _db_file: db_file,
            writer,
            reader,
            file_size,
            read_offset,
            index: BTreeIndex::new(idx_file)?, // index,
                                               // field_map: FieldMap::new(map_file)?,
        })
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

mod address_entry;
mod b_tree;
mod field_map;
pub mod optional_ord;
mod page_addr;
mod tables;
use crate::optional_ord::FieldOrd;
use crate::tables::TableLinkHeader;
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
    io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write},
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
    record_buf: Cursor<Vec<u8>>,
    index: BTreeIndex<'a, PAGE_SIZE, T>,
    field_map: FieldMap<'a, PAGE_SIZE, T>,
}

impl<'a, const PAGE_SIZE: usize, T: Read + Write + Seek + std::fmt::Debug>
    Database<'a, PAGE_SIZE, T>
{
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
            record_buf: Cursor::new(Vec::with_capacity(PAGE_SIZE)),
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

    pub fn fetch_records_eq<TT: STable + std::fmt::Debug>(
        &mut self,
        field_name: &str,
        val: &dyn Any,
    ) -> Result<Vec<TableRecord<TT>>, DatabaseErr> {
        //     207,
        //     216,
        //     99, )| trs: -117
        //     197,
        //     207,
        //     80, )| trs: -127
        let mut records = Vec::new();
        const STRONG_TY_U64_OFFSET: i64 = -(<u64>::BYTE_SIZE as i64 + 1);
        // [[[u64]tlh][inner_record]][u64]
        //                           ^---|
        let mut current_offset = self.db_file.seek(SeekFrom::End(STRONG_TY_U64_OFFSET))?;
        eprintln!("{:#?}", self.db_file.stream_position());
        let mut checksum = 0;
        // [[[u64]tlh][inner_record]][u64]
        //                           |----^
        let mut tr_byte_len = (STRONG_TY_U64_OFFSET
            - (<usize>::read_byte_stream_le(self.db_file, &mut checksum)? as i64))
            + STRONG_TY_U64_OFFSET;
        eprintln!("{:#?}", self.db_file.stream_position());

        while (current_offset as i64 - tr_byte_len) > 0 {
            checksum = 0;

            // [[[u64]tlh][inner_record]][u64]
            //   ^----------------------------|
            current_offset = self.db_file.seek(SeekFrom::Current(tr_byte_len))?;
            eprintln!(
                "{:#?}| trs: {}",
                self.db_file.stream_position(),
                tr_byte_len
            );

            // [[[u64]tlh][inner_record]][u64]
            //   |--------^
            let tlh = <TableLinkHeader>::read_byte_stream_le(self.db_file, &mut checksum)?;
            eprintln!("{:#?}", self.db_file.stream_position());

            if tlh.type_hash == TT::TYPE_HASH {
                // [[[u64]tlh][inner_record]][u64]
                //            |--------------^
                let inner_record = <TT>::read_byte_stream_le(self.db_file, &mut checksum)?;
                eprintln!(
                    ">>{:#?}:{:#?}",
                    self.db_file.stream_position(),
                    inner_record
                );
                // [[[u64]tlh][inner_record]][[[u64]tlh][inner_record]]
                //   ^------------------------------------------------|
                tr_byte_len = 0 - (checksum + tlh.prior_record_len + <u64>::BYTE_SIZE + 1) as i64;
                records.push(TableRecord::<TT>::from_parts(tlh, inner_record));
            } else {
                eprintln!("huh");
                // [[[u64]tlh][inner_record]][[[u64]tlh][inner_record]]
                //   ^----------------------------------|
                tr_byte_len = STRONG_TY_U64_OFFSET - (checksum + tlh.prior_record_len) as i64;
            }
        }
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        use steadfast_macros::{InternalTableSF, ReadByteStreamInternal, WriteByteStreamInternal};
        #[derive(
            Debug,
            InternalTableSF,
            ReadByteStreamInternal,
            WriteByteStreamInternal,
            PartialOrd,
            PartialEq,
        )]
        struct TestStruct {
            some_field: u64,
        }

        let tra = TableRecord::new(TestStruct { some_field: 42 }).unwrap();
        let trb = TableRecord::new(TestStruct { some_field: 68 }).unwrap();

        let mut db_file = Cursor::new(Vec::new());
        tra.write_byte_stream_le(&mut db_file).unwrap();
        tra.write_byte_stream_le(&mut db_file).unwrap();

        let mut idx_file = Cursor::new(Vec::new());
        let mut map_file = Cursor::new(Vec::new());

        let mut db = Database::<64, _>::new(&mut db_file, &mut map_file, &mut idx_file).unwrap();
        let records: Vec<TableRecord<TestStruct>> = db.fetch_records_eq("some_field", &42).unwrap();
        assert_eq!(records[0], tra);
    }
}

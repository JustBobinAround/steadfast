mod address_entry;
mod b_tree;
mod field_map;
mod tables;
use crate::{
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
use steadfast_serializer::{DataHolder, Deserialize, Serialize};
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
pub struct Database<'a> {
    _db_file: &'a File,
    writer: BufWriter<&'a File>,
    reader: BufReader<&'a File>,
    file_size: usize,
    read_offset: usize,
    index: BTreeIndex<'a, 4096>,
    field_map: FieldMap<'a, 4096>,
}

impl<'a> Database<'a> {
    pub fn open_db_file(path: &str) -> Result<File, DatabaseErr> {
        let db_path = Path::new(path);

        OpenOptions::new()
            .append(true)
            .read(true)
            .create(true)
            .open(&db_path)
            .map_err(|_| DatabaseErr::FailedToOpen)
    }

    fn current_time() -> Result<usize, DatabaseErr> {
        use std::time::{SystemTime, UNIX_EPOCH};

        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| DatabaseErr::TimeWentBackwards)?
            .as_millis() as usize)
    }

    // fn insert_entry(&mut self, entry: AddressEntry) {
    //     self.index.insert(entry.uuid, entry);
    // }

    fn write_bytes_exact(&mut self, bytes: &[u8]) -> Result<(), DatabaseErr> {
        let bytes_written = self
            .writer
            .write(bytes)
            .map_err(|_| DatabaseErr::FailedToWrite)?;
        if bytes_written < bytes.len() {
            Err(DatabaseErr::FailedToWrite)
        } else {
            Ok(())
        }
    }

    fn flush_writer(&mut self) -> Result<(), DatabaseErr> {
        self.writer
            .flush()
            .map_err(|_| DatabaseErr::FailedToWrite)?;

        Ok(())
    }

    pub fn append_entry<T: STable>(&mut self, uuid: UUID, data: T) -> Result<(), DatabaseErr> {
        match self.read_entry::<T>(uuid) {
            Ok(current_data) => {
                if &current_data == &data {
                    return Ok(());
                }
            }
            _ => {}
        }
        let data = data.serialize();
        let mut buf = data.to_bytes(Vec::new());
        let last_update = Self::current_time()?;
        if uuid.extract_timestamp() > last_update as u64 {
            //time went backwards some how
            return Err(DatabaseErr::TimeWentBackwards);
        }
        // let table_id = T::table_id();
        // let entry = AddressEntry {
        //     uuid,
        //     address: self.file_size,
        //     last_update,
        //     table_id,
        // };

        let uuid_bytes = &uuid.as_u128().to_le_bytes();
        let data_len_bytes = &buf.len().to_le_bytes();
        let last_update_bytes = &last_update.to_le_bytes();
        let table_id_bytes = T::TABLE_ID.to_bytes_le();
        let table_type_bytes = T::TYPE_HASH.to_bytes_le();
        self.write_bytes_exact(uuid_bytes)?;
        self.write_bytes_exact(&table_id_bytes)?;
        self.write_bytes_exact(&table_type_bytes)?;
        self.write_bytes_exact(data_len_bytes)?;
        self.write_bytes_exact(last_update_bytes)?;
        self.write_bytes_exact(&buf)?;
        self.flush_writer()?;

        // self.insert_entry(entry);

        self.file_size +=
            &buf.len() + uuid_bytes.len() + data_len_bytes.len() + last_update_bytes.len();

        Ok(())
    }

    const UUID_BSIZE: usize = 16;
    const DATA_LEN_BSIZE: usize = 8;
    const LAST_UPDATE_BSIZE: usize = 8;
    const ENTRY_HEADER_OFFSET: usize =
        Self::UUID_BSIZE + Self::DATA_LEN_BSIZE + Self::LAST_UPDATE_BSIZE;

    pub fn read_entry<T: STable>(&mut self, uuid: UUID) -> Result<T, DatabaseErr> {
        let address = todo!();

        let (_, data_len) = self.read_address_entry_at(address)?;
        let bytes = self.read_entry_data(data_len)?;
        let dh = DataHolder::from_bytes(&bytes).map_err(|_| DatabaseErr::FailedToDeserialize)?;
        T::deserialize(dh.1).map_err(|_| DatabaseErr::FailedToDeserialize)
    }

    fn index_next_entry(&mut self) -> Result<(), DatabaseErr> {
        let (entry, data_len) = self.read_next_address_entry()?;
        // self.index.insert(entry.uuid, entry);
        self.reader
            .seek_relative(data_len as i64)
            .map_err(|_| DatabaseErr::FailedToRead)?;
        self.read_offset += data_len;

        Ok(())
    }

    fn read_address_entry_at(
        &mut self,
        address: usize,
    ) -> Result<(AddressEntry, usize), DatabaseErr> {
        self.seek_reader(SeekFrom::Start(address as u64))?;
        self.read_offset = address;
        self.read_next_address_entry()
    }

    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], DatabaseErr> {
        let mut buf = [0u8; N];
        self.reader
            .read_exact(&mut buf)
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::UnexpectedEof => DatabaseErr::EOF,
                _ => DatabaseErr::FailedToRead,
            })?;

        Ok(buf)
    }

    fn read_next_address_entry(&mut self) -> Result<(AddressEntry, usize), DatabaseErr> {
        let uuid_bytes = self.read_exact::<{ Database::UUID_BSIZE }>()?;
        let data_len_bytes = self.read_exact::<{ Database::DATA_LEN_BSIZE }>()?;
        let last_update_bytes = self.read_exact::<{ Database::LAST_UPDATE_BSIZE }>()?;
        let table_hash_bytes = self.read_exact::<{ Database::UUID_BSIZE }>()?;

        let data_len = usize::from_le_bytes(data_len_bytes);
        let uuid = UUID::from_u128(u128::from_le_bytes(uuid_bytes));
        let last_update = usize::from_le_bytes(last_update_bytes);
        let table_id = usize::from_le_bytes(last_update_bytes);
        let entry = AddressEntry {
            uuid,
            address: self.read_offset,
            last_update,
            // table_id,
        };

        self.read_offset += Self::ENTRY_HEADER_OFFSET;

        Ok((entry, data_len))
    }

    fn seek_reader(&mut self, seek: SeekFrom) -> Result<(), DatabaseErr> {
        self.reader
            .seek(seek)
            .map_err(|_| DatabaseErr::FailedToRead)?;
        Ok(())
    }

    fn read_entry_data(&mut self, data_len: usize) -> Result<Vec<u8>, DatabaseErr> {
        let mut data = vec![0; data_len];
        self.reader
            .read_exact(&mut data[..data_len])
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::UnexpectedEof => DatabaseErr::EOF,
                _ => DatabaseErr::FailedToRead,
            })?;

        self.read_offset += data_len;

        Ok(data)
    }

    fn read_entry_data_at(
        &mut self,
        address: usize,
        data_len: usize,
    ) -> Result<Vec<u8>, DatabaseErr> {
        self.seek_reader(SeekFrom::Start(address as u64))?;

        self.read_entry_data(data_len)
    }

    fn init_index(mut self) -> Result<Self, DatabaseErr> {
        self.seek_reader(SeekFrom::Start(0))?;
        self.read_offset = 0;

        loop {
            match self.index_next_entry() {
                Err(DatabaseErr::EOF) => break,
                Err(e) => return Err(e),
                _ => {}
            }
        }

        Ok(self)
    }

    pub fn new(
        db_file: &'a File,
        map_file: &'a mut File,
        idx_file: &'a mut File,
    ) -> Result<Self, DatabaseErr> {
        let writer = BufWriter::new(db_file);
        let reader = BufReader::new(db_file);
        // let index = HashMap::new();
        let file_size = 0;
        let read_offset = 0;

        let db = Database {
            _db_file: db_file,
            writer,
            reader,
            file_size,
            read_offset,
            index: BTreeIndex::new(idx_file)?, // index,
            field_map: FieldMap::new(map_file)?,
        }
        .init_index()?;

        Ok(db)
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

mod b_tree;
mod db_bytes;
mod field_map;
mod tables;
use crate::tables::STable;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};
use steadfast_crypt::SHA256;
use steadfast_serializer::{DataHolder, Deserialize, Serialize};
use steadfast_uuid::UUID;

#[repr(C)]
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddressEntry {
    pub uuid: UUID,
    pub address: usize,
    pub last_update: usize,
    // pub table_id: SHA256,
}

impl AddressEntry {
    pub const BYTE_SIZE: usize = 32;

    pub fn dealloc_entry(&self) -> Self {
        let mut uuid = self.uuid.clone();
        uuid.0 = uuid.0 & 0x0000_0000_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF;

        AddressEntry {
            uuid,
            address: self.address,
            last_update: self.last_update,
            // table_id: self.table_id.clone(), //TODO: see if we need this clone
        }
    }

    pub fn is_deallocated(&self) -> bool {
        self.uuid.extract_timestamp() == 0
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.uuid
            .as_u128()
            .to_le_bytes()
            .into_iter()
            .chain(
                self.address
                    .to_le_bytes()
                    .into_iter()
                    .chain(self.last_update.to_le_bytes().into_iter()),
            )
            .enumerate()
            .fold([0u8; 32], |mut buf, (i, b)| {
                buf[i] = b;
                buf
            })
    }

    // pub fn from_bytes(buf: [u8; 32]) -> Result<Self, ()> {
    //     let uuid = UUID::from_u128(<u128>::from_le_bytes(
    //         <[u8; 16]>::try_from(&buf[0..16]).expect("guarenteed 0..16 slice failed to convert"),
    //     ));

    //     let address = <usize>::from_le_bytes(
    //         <[u8; 8]>::try_from(&buf[16..24]).expect("guarenteed 16..24 slice failed to convert"),
    //     );
    //     let last_update = <usize>::from_le_bytes(
    //         <[u8; 8]>::try_from(&buf[24..32]).expect("guarenteed 24..32 slice failed to convert"),
    //     );
    //     let table_id = <usize>::from_le_bytes(
    //         <[u8; 8]>::try_from(&buf[32..40]).expect("guarenteed 32..40 slice failed to convert"),
    //     );
    //     Ok(Self {
    //         uuid,
    //         address,
    //         last_update,
    //         table_id,
    //     })
    // }
}

impl Default for AddressEntry {
    fn default() -> Self {
        AddressEntry {
            uuid: UUID::default(),
            address: 0,
            last_update: 0,
            // table_id: 0,
        }
    }
}

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
}

#[derive(Debug)]
pub struct Database<'a> {
    _db_file: &'a File,
    writer: BufWriter<&'a File>,
    reader: BufReader<&'a File>,
    file_size: usize,
    read_offset: usize,
    // index: HashMap<UUID, AddressEntry>,
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
        let mut buf = Vec::new();
        data.to_bytes(&mut buf);
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
        let table_id_bytes = T::TABLE_ID.to_le_bytes();
        let table_type_bytes = T::TYPE_HASH.to_le_bytes();
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

    pub fn new(db_file: &'a File) -> Result<Self, DatabaseErr> {
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
            // index,
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

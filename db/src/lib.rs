use json::IntoJson;
use std::{
    collections::{BTreeMap, HashMap},
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    os::unix::fs::FileExt,
    path::Path,
    sync::{Arc, RwLock},
};
use uuid::UUID;

#[repr(C)]
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddressEntry {
    pub uuid: UUID,
    pub address: usize,
    pub last_update: usize,
}

impl AddressEntry {
    pub const BYTE_SIZE: usize = 32;

    pub fn dealloc_entry(&self) -> Self {
        let mut uuid = self.uuid.clone();
        uuid.data_1 = 0;

        AddressEntry {
            uuid,
            address: self.address,
            last_update: self.last_update,
        }
    }

    pub fn is_deallocated(&self) -> bool {
        self.uuid.data_1 == 0
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

    pub fn from_bytes(buf: [u8; 32]) -> Self {
        let uuid = UUID::from_u128(<u128>::from_le_bytes(
            <[u8; 16]>::try_from(&buf[0..16]).expect("guarenteed 0..16 slice failed to convert"),
        ));

        let address = <usize>::from_le_bytes(
            <[u8; 8]>::try_from(&buf[16..24]).expect("guarenteed 16..24 slice failed to convert"),
        );
        let last_update = <usize>::from_le_bytes(
            <[u8; 8]>::try_from(&buf[24..32]).expect("guarenteed 16..24 slice failed to convert"),
        );
        Self {
            uuid,
            address,
            last_update,
        }
    }
}

impl Default for AddressEntry {
    fn default() -> Self {
        AddressEntry {
            uuid: UUID {
                // data_1 should never be 0 because that would be 1970 unix sys time
                data_1: 0,
                data_2: 0,
                data_3: 0,
                data_4: [0; 8],
            },
            address: 0,
            last_update: 0,
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
}

#[derive(Debug)]
pub struct Database<'a> {
    db_file: &'a File,
    writer: BufWriter<&'a File>,
    reader: BufReader<&'a File>,
    file_size: usize,
    read_offset: usize,
    index: HashMap<UUID, AddressEntry>,
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

    fn insert_entry(&mut self, entry: AddressEntry) {
        self.index.insert(entry.uuid, entry);
    }

    pub fn append_entry(&mut self, uuid: UUID, data: &[u8]) -> Result<(), DatabaseErr> {
        match self.read_entry(uuid) {
            Ok(current_data) => {
                if &current_data == data {
                    return Ok(());
                }
            }
            _ => {}
        }
        let last_update = Self::current_time()?;
        if uuid.extract_timestamp() > last_update as u64 {
            //time went backwards some how
            return Err(DatabaseErr::TimeWentBackwards);
        }
        let entry = AddressEntry {
            uuid,
            address: self.file_size,
            last_update,
        };

        let uuid_bytes = &uuid.as_u128().to_le_bytes();
        let data_len_bytes = &data.len().to_le_bytes();
        let last_update_bytes = &last_update.to_le_bytes();
        self.writer
            .write(uuid_bytes)
            .map_err(|_| DatabaseErr::FailedToWrite)?;
        self.writer
            .write(data_len_bytes)
            .map_err(|_| DatabaseErr::FailedToWrite)?;
        self.writer
            .write(last_update_bytes)
            .map_err(|_| DatabaseErr::FailedToWrite)?;
        self.writer
            .write(&data)
            .map_err(|_| DatabaseErr::FailedToWrite)?;
        self.writer
            .flush()
            .map_err(|_| DatabaseErr::FailedToWrite)?;

        self.insert_entry(entry);

        self.file_size +=
            data.len() + uuid_bytes.len() + data_len_bytes.len() + last_update_bytes.len();

        Ok(())
    }

    const UUID_BSIZE: usize = 16;
    const DATA_LEN_BSIZE: usize = 8;
    const LAST_UPDATE_BSIZE: usize = 8;
    const ENTRY_HEADER_OFFSET: usize =
        Self::UUID_BSIZE + Self::DATA_LEN_BSIZE + Self::LAST_UPDATE_BSIZE;

    pub fn read_entry(&mut self, uuid: UUID) -> Result<Vec<u8>, DatabaseErr> {
        let address = match self.index.get(&uuid) {
            Some(address_entry) => address_entry.address,
            None => return Err(DatabaseErr::NoEntryFound),
        };

        let (_, data_len) = self.read_address_entry_at(address)?;
        self.read_entry_data(data_len)
    }

    fn index_next_entry(&mut self) -> Result<(), DatabaseErr> {
        let (entry, data_len) = self.read_next_address_entry()?;
        self.index.insert(entry.uuid, entry);
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
        self.reader
            .seek(SeekFrom::Start(address as u64))
            .map_err(|_| DatabaseErr::FailedToRead)?;
        self.read_offset = address;
        self.read_next_address_entry()
    }

    fn read_next_address_entry(&mut self) -> Result<(AddressEntry, usize), DatabaseErr> {
        let mut uuid_bytes = [0u8; Self::UUID_BSIZE];
        self.reader
            .read_exact(&mut uuid_bytes)
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::UnexpectedEof => DatabaseErr::EOF,
                _ => DatabaseErr::FailedToRead,
            })?;

        let mut data_len_bytes = [0u8; Self::DATA_LEN_BSIZE];
        self.reader
            .read_exact(&mut data_len_bytes)
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::UnexpectedEof => DatabaseErr::EOF,
                _ => DatabaseErr::FailedToRead,
            })?;

        let mut last_update_bytes = [0u8; Self::LAST_UPDATE_BSIZE];
        self.reader
            .read_exact(&mut last_update_bytes)
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::UnexpectedEof => DatabaseErr::EOF,
                _ => DatabaseErr::FailedToRead,
            })?;

        let data_len = usize::from_le_bytes(data_len_bytes);
        let uuid = UUID::from_u128(u128::from_le_bytes(uuid_bytes));
        let last_update = usize::from_le_bytes(last_update_bytes);
        let entry = AddressEntry {
            uuid,
            address: self.read_offset,
            last_update,
        };

        self.read_offset += Self::ENTRY_HEADER_OFFSET;

        Ok((entry, data_len))
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
        self.reader
            .seek(SeekFrom::Start(address as u64))
            .map_err(|_| DatabaseErr::FailedToRead)?;

        self.read_entry_data(data_len)
    }

    fn init_index(mut self) -> Result<Self, DatabaseErr> {
        self.reader
            .seek(SeekFrom::Start(0))
            .map_err(|_| DatabaseErr::FailedToRead)?;
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
        let index = HashMap::new();
        let file_size = 0;
        let read_offset = 0;

        let db = Database {
            db_file,
            writer,
            reader,
            file_size,
            read_offset,
            index,
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

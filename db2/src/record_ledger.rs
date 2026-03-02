use super::record_index::RecordIndex;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

pub struct RecordLedger {
    record_index: RecordIndex,
    ledger_file: File,
    index_file: File,
    wal_file: File,
}

impl RecordLedger {
    pub fn new(path: &str) -> Result<Self, ()> {
        let ledger_path = Path::new(path);
        let index_path = ledger_path.with_extension("zero_idx");
        let wal_path = ledger_path.with_extension("zero_wal");

        let mut ledger_file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&ledger_path)
            .map_err(|_| ())?;
        ledger_file.seek(SeekFrom::Start(0)).map_err(|_| ())?;
        ledger_file.unlock().map_err(|_| ())?;

        let mut index_file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&index_path)
            .map_err(|_| ())?;
        index_file.seek(SeekFrom::Start(0)).map_err(|_| ())?;
        index_file.unlock().map_err(|_| ())?;

        let mut wal_file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&wal_path)
            .map_err(|_| ())?;
        wal_file.seek(SeekFrom::Start(0)).map_err(|_| ())?;
        wal_file.unlock().map_err(|_| ())?;

        let record_index = RecordIndex::new();

        Ok(RecordLedger {
            record_index,
            ledger_file,
            index_file,
            wal_file,
        })
    }
}

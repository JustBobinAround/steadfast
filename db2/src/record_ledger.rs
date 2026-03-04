use json::{IntoJson, JsonVal};

use crate::record_index::{AddressEntry, EntryType, IndexErr};

use super::record_index::RecordIndex;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

pub struct LedgerEntry {
    json_obj: JsonVal,
    update_time: usize,
}
impl std::fmt::Display for LedgerEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{},", self.update_time, self.json_obj)
    }
}

pub struct RecordLedger {
    record_index: RecordIndex,
    ledger_file: File,
    index_file: File,
    last_update_time: usize,
    ledger_end_offset: usize,
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

        let record_index = RecordIndex::new();

        Ok(RecordLedger {
            record_index,
            ledger_file,
            index_file,
            last_update_time: 0,
            ledger_end_offset: 0,
        })
    }

    pub fn resize(&mut self, entry: AddressEntry) -> Result<(), ()> {
        crate::lock!(self.index_file => {
            self.index_file.seek(SeekFrom::Start(0)).map_err(|_| ())?;
            self.index_file.write(&entry.last_update.to_be_bytes()).map_err(|_| ())?;
            Ok(())
        })
    }

    pub fn append_wal(&mut self, entry: LedgerEntry) -> Result<(), ()> {
        crate::lock!(self.index_file => {
            self.index_file.seek(SeekFrom::Start(0)).map_err(|_| ())?;
            self.index_file.write(&entry.update_time.to_be_bytes()).map_err(|_| ())?;
            self.index_file.seek(SeekFrom::End(0)).map_err(|_| ())?;
            self.index_file.write(entry.to_string().as_bytes()).map_err(|_| ())?;
            Ok(())
        })
    }

    pub fn insert_record(&mut self, record: impl IntoJson) -> Result<(), ()> {
        let entry = self
            .record_index
            .insert_allocation(self.ledger_end_offset)
            .map_err(|_| ())?;

        let entry = match entry {
            EntryType::NeedsResize(entry) => {
                self.resize(entry);
                entry
            }
            EntryType::Normal(entry) => entry,
        };

        let ledger_entry = LedgerEntry {
            json_obj: record.to_json_val(),
            update_time: entry.last_update,
        };
        self.append_wal(ledger_entry)?;
        Ok(())
    }
}

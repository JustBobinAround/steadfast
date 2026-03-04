use uuid::UUID;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct AddressEntry {
    pub uuid: UUID,
    pub last_file_offset: usize,
    pub last_update: usize,
}

impl Copy for AddressEntry {}
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
            last_file_offset: 0,
            last_update: 0,
        }
    }
}

impl AddressEntry {
    pub const BYTE_SIZE: usize = 32;
    pub fn to_bytes(&self) -> [u8; 32] {
        self.uuid
            .as_u128()
            .to_le_bytes()
            .into_iter()
            .chain(
                self.last_file_offset
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

        let last_file_offset = <usize>::from_le_bytes(
            <[u8; 8]>::try_from(&buf[16..24]).expect("guarenteed 16..24 slice failed to convert"),
        );
        let last_update = <usize>::from_le_bytes(
            <[u8; 8]>::try_from(&buf[24..32]).expect("guarenteed 16..24 slice failed to convert"),
        );
        Self {
            uuid,
            last_file_offset,
            last_update,
        }
    }
}

pub enum IndexErr {
    FailedToFetchUUID,
    FailedToFetchSysTime,
}

pub enum EntryType {
    Normal(AddressEntry),
    NeedsResize(AddressEntry),
}

pub struct RecordIndex {
    entries: Vec<AddressEntry>,
}

impl RecordIndex {
    pub fn new() -> RecordIndex {
        let mut entries = Vec::with_capacity(1028);
        entries.resize_with(1028, || AddressEntry::default());
        RecordIndex { entries }
    }

    // pub const fn resize<const NN: usize>(self) -> RecordIndex<NN> {}

    fn uuid_idx(&self, uuid: &UUID) -> usize {
        (uuid.as_u128() % self.entries.capacity() as u128) as usize
    }

    pub fn has_collision(&self, idx: usize, uuid: &UUID) -> bool {
        if self.entries.len() > idx {
            self.entries[idx].uuid.data_1 > 0 && &self.entries[idx].uuid != uuid
        } else {
            false
        }
    }

    fn insert_entry(&mut self, entry: AddressEntry) -> Result<EntryType, IndexErr> {
        // self.db.sync_address_map()?;
        let mut needs_resize = false;
        if self.entries.len() > self.entries.capacity() / 2 {
            self.resize()?;
            needs_resize = true;
        }
        let mut idx = self.uuid_idx(&entry.uuid);
        let idx = loop {
            if self.has_collision(idx, &entry.uuid) {
                idx += 1;
                if idx >= self.entries.len() {
                    self.resize()?;
                    needs_resize = true;
                }
            } else {
                break idx;
            }
        };

        self.entries[idx] = entry;

        if needs_resize {
            Ok(EntryType::NeedsResize(entry))
        } else {
            Ok(EntryType::Normal(entry))
        }
    }

    const MAX_ATTEMPTS: u8 = 5;

    pub fn insert_allocation(&mut self, last_file_offset: usize) -> Result<EntryType, IndexErr> {
        let mut attempts = 0;
        let uuid = loop {
            let uuid = UUID::rand_v7().map_err(|_| IndexErr::FailedToFetchUUID)?;
            let idx = self.uuid_idx(&uuid);

            if self.has_collision(idx, &uuid) {
                attempts += 1;
            } else {
                break uuid;
            }

            if attempts > Self::MAX_ATTEMPTS {
                break uuid;
            }
        };

        let last_update = uuid.extract_timestamp() as usize;

        let entry = AddressEntry {
            uuid,
            last_file_offset,
            last_update,
        };

        self.insert_entry(entry)
    }

    fn current_time() -> Result<u64, IndexErr> {
        use std::time::{SystemTime, UNIX_EPOCH};

        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| IndexErr::FailedToFetchSysTime)?
            .as_millis() as u64)
    }

    pub fn update_offset(
        &mut self,
        uuid: UUID,
        last_file_offset: usize,
    ) -> Result<EntryType, IndexErr> {
        let last_update = Self::current_time()? as usize;
        let entry = AddressEntry {
            uuid,
            last_file_offset,
            last_update,
        };

        self.insert_entry(entry)
    }

    pub fn resize(&mut self) -> Result<(), IndexErr> {
        let mut entries = Vec::with_capacity(self.entries.capacity() * 2);
        entries.resize_with(self.entries.capacity() * 2, || AddressEntry::default());
        std::mem::swap(&mut self.entries, &mut entries);
        for entry in entries {
            self.insert_entry(entry)?;
        }

        Ok(())
    }
}

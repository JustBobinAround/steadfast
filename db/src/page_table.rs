use crate::DatabaseBuffer;

use std::collections::BTreeMap;

use uuid::UUID;

pub type Address = usize;
pub type MemSize = usize;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct AddressEntry {
    pub uuid: UUID,
    pub address: Address,
    pub size: MemSize,
}

impl AddressEntry {
    pub const BYTE_SIZE: usize = 32;

    pub fn dealloc_entry(&self) -> Self {
        let mut uuid = self.uuid.clone();
        uuid.data_1 = 0;

        AddressEntry {
            uuid,
            address: self.address,
            size: self.size,
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
                    .chain(self.size.to_le_bytes().into_iter()),
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
        let size = <usize>::from_le_bytes(
            <[u8; 8]>::try_from(&buf[24..32]).expect("guarenteed 16..24 slice failed to convert"),
        );
        Self {
            uuid,
            address,
            size,
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
            size: 0,
        }
    }
}

#[derive(Debug)]
pub struct AddressMap {
    // db.as_ref().unwrap().address_map_entries: Vec<AddressEntry>,
    db: DatabaseBuffer,
    reserved_count: usize,
}

impl AddressMap {
    const MAX_ATTEMPTS: u8 = 5;
    const PADDING: usize = 4096;

    pub fn byte_size(&self) -> usize {
        self.db.address_map_entries.capacity() * AddressEntry::BYTE_SIZE
    }

    pub fn start_address(&self) -> usize {
        Self::PADDING
    }

    pub fn end_address(&self) -> usize {
        Self::PADDING + self.byte_size()
    }

    pub fn is_entry_address(&self, address: usize) -> bool {
        let start = Self::PADDING;
        let end = start + self.byte_size();
        address >= start && address < end
    }

    pub fn new(path: &str) -> Result<Self, ()> {
        let db = DatabaseBuffer::new(path)?;

        Ok(AddressMap {
            db,
            reserved_count: 0,
        })
    }

    fn uuid_idx(&self, uuid: &UUID) -> usize {
        (uuid.as_u128() % self.db.address_map_entries.capacity() as u128) as usize
    }

    pub fn has_collision(&self, idx: usize) -> bool {
        if self.db.address_map_entries.len() > idx {
            self.db.address_map_entries[idx].uuid.data_1 > 0
        } else {
            false
        }
    }

    pub fn check_or_grow(&mut self, idx: usize) {
        if self.db.address_map_entries.len() <= idx {
            self.db
                .address_map_entries
                .resize_with(idx + 1, || AddressEntry::default());
        }
    }

    pub fn resize(&mut self) -> Result<(), ()> {
        // let total_used = self.total_used
        //     - ((self.db.address_map_entries.capacity() * AddressEntry::BYTE_SIZE) + Self::PADDING);
        // let db = self.db;
        let mut old_entries = self.db.resize_entry_alloc()?;
        self.db.freed_ranges = BTreeMap::new();
        let mut total_used = (old_entries.capacity() * 2 * AddressEntry::BYTE_SIZE) + Self::PADDING;
        self.db.set_total_used(total_used)?;
        self.reserved_count = 0;
        // self.total_used = total_used + self.byte_size() + Self::PADDING;
        for entry in old_entries.iter_mut() {
            if entry.uuid.data_1 > 0 {
                if entry.address < (self.byte_size() + Self::PADDING) {
                    self.db.move_data(entry.address, total_used, entry.size)?;
                    entry.address = total_used;
                    total_used += entry.size;
                    self.db.set_total_used(total_used)?;
                }
            }
        }
        for entry in old_entries {
            self.insert_entry(entry)?;
        }
        Ok(())
    }

    fn insert_entry(&mut self, entry: AddressEntry) -> Result<&AddressEntry, ()> {
        self.db.sync_address_map()?;
        if self.reserved_count > self.db.address_map_entries.capacity() / 2 {
            self.resize()?;
        }
        let mut idx = self.uuid_idx(&entry.uuid);
        let idx = loop {
            if self.has_collision(idx) {
                idx += 1;
                if idx >= self.db.address_map_entries.capacity() {
                    self.resize()?;
                    idx = self.uuid_idx(&entry.uuid);
                }
            } else {
                break idx;
            }
        };

        // self.check_or_grow(idx);

        let offset = Self::PADDING + (idx * AddressEntry::BYTE_SIZE);
        self.db.write_at(&entry.to_bytes(), offset)?;
        // self.db.as_ref().unwrap().address_map_entries[idx] = entry; // db write will update this
        self.reserved_count += 1;

        Ok(&self.db.address_map_entries[idx])
    }

    pub fn insert_allocation(&mut self, size: usize) -> Result<&AddressEntry, ()> {
        self.db.sync_address_map()?;
        let address = self.db.freed_ranges.range(size..).next();
        eprintln!(">><{:#?}", self.db.freed_ranges);
        eprintln!(">><{:#?}", address);

        let address = address.map(|entry| *entry.1).unwrap_or({
            let address = self.db.total_used;
            eprintln!("{}", address);
            self.db.set_total_used(address + size)?;
            address
        });
        eprintln!(">>{:#?}", address);

        let mut attempts = 0;
        let uuid = loop {
            let uuid = UUID::rand_v7()?;
            let idx = self.uuid_idx(&uuid);

            if self.has_collision(idx) {
                attempts += 1;
            } else {
                break uuid;
            }

            if attempts > Self::MAX_ATTEMPTS {
                break uuid;
            }
        };

        let entry = AddressEntry {
            uuid,
            address,
            size,
        };

        Ok(self.insert_entry(entry)?)
    }

    pub fn set_bytes_at(&mut self, idx: usize, bytes: [u8; 32]) {
        self.check_or_grow(idx);
        self.db.address_map_entries[idx] = AddressEntry::from_bytes(bytes);
    }

    pub fn get(&mut self, uuid: &UUID) -> Result<Option<&AddressEntry>, ()> {
        eprintln!("{:#?}", self.db.freed_ranges);
        self.db.sync_address_map()?;
        let mut idx = self.uuid_idx(uuid);
        if idx >= self.db.address_map_entries.len() {
            Ok(None)
        } else {
            let mut found_entry = None;
            while idx < self.db.address_map_entries.len() {
                if self.db.address_map_entries[idx].is_deallocated() {
                    break;
                }
                if &self.db.address_map_entries[idx].uuid == uuid {
                    found_entry = Some(&self.db.address_map_entries[idx]);
                    break;
                }
                idx += 1;
            }

            Ok(found_entry)
        }
    }

    pub fn remove_allocation(&mut self, uuid: &UUID) -> Result<Option<AddressEntry>, ()> {
        self.db.sync_address_map()?;
        let mut idx = self.uuid_idx(uuid);
        let org_idx = idx;
        if idx >= self.db.address_map_entries.len() {
            Ok(None)
        } else {
            let mut found_entry = None;
            while idx < self.db.address_map_entries.len() {
                if self.db.address_map_entries[idx].is_deallocated() {
                    break;
                }
                if &self.db.address_map_entries[idx].uuid == uuid {
                    found_entry = Some(idx);
                    break;
                }
                idx += 1;
            }

            if let Some(idx) = found_entry {
                let mut sec_idx = idx + 1;
                while sec_idx < self.db.address_map_entries.len() {
                    if self.db.address_map_entries[sec_idx].is_deallocated()
                        || self.uuid_idx(&self.db.address_map_entries[sec_idx].uuid) != org_idx
                    {
                        break;
                    }
                    sec_idx += 1;
                }
                sec_idx -= 1;
                let default = self.db.address_map_entries[idx].dealloc_entry();
                let from_offset = Self::PADDING + (idx * AddressEntry::BYTE_SIZE);
                let to_offset = Self::PADDING + (sec_idx * AddressEntry::BYTE_SIZE);
                self.db
                    .move_data_with(to_offset, from_offset, default.to_bytes())?;
                Ok(Some(default))
            } else {
                Ok(None)
            }
        }
    }
}

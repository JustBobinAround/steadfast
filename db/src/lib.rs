pub mod bytes;
pub mod page_table;

use crate::page_table::AddressEntry;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::FileExt,
    path::Path,
    sync::{Arc, RwLock},
};

pub enum UpdateOp {
    Resize { to: usize },
    WriteBytes { at: usize, data: Vec<u8> },
}

impl UpdateOp {
    const RESIZE: u8 = 0;
    const WRITE_BYTES: u8 = 1;
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Resize { to } => {
                to.to_le_bytes()
                    .into_iter()
                    .fold(vec![Self::RESIZE], |mut bytes, b| {
                        bytes.push(b);
                        bytes
                    })
            }
            Self::WriteBytes { at, data } => at
                .to_le_bytes()
                .into_iter()
                .chain(data.len().to_le_bytes().into_iter().chain(data))
                .fold(vec![Self::WRITE_BYTES], |mut bytes, b| {
                    bytes.push(b);
                    bytes
                }),
        }
    }
}

pub enum PageState {
    DirtyEmpty,
    Empty,
    CanWrite,
    Overwritten,
    Collides,
}

impl PageState {
    pub const UNLOCKED: usize = 0x0000_0000_0000_0000;
    pub const LOCK_SHARED: usize = 0xFF00_0000_0000_0000;
    pub const LOCKED: usize = 0xFFFF_0000_0000_0000;
}

pub type Page = [u8; 4096];

#[derive(Debug)]
pub struct DatabaseBuffer {
    db_file: File,
    wal_file: File,
    page_buf: HashMap<usize, Arc<RwLock<Page>>>,
    commit: usize,
    ledger_version: usize,
    pub address_map_entries: Vec<AddressEntry>,
    pub total_used: usize,
}

impl DatabaseBuffer {
    pub fn new(path: &str) -> Result<Self, ()> {
        let db_path = Path::new(path);
        let wal_path = db_path.with_extension("zero_wal");

        let mut wal_file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&wal_path)
            .map_err(|_| ())?;
        wal_file.seek(SeekFrom::Start(16)).map_err(|_| ())?;
        wal_file.unlock().map_err(|_| ())?;

        let db_file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&db_path)
            .map_err(|_| ())?;
        db_file.unlock().map_err(|_| ())?;

        // db.set_total_used(total_used)?;

        let mut db = DatabaseBuffer {
            db_file,
            wal_file,
            page_buf: HashMap::with_capacity(1000),
            commit: 0,
            ledger_version: 0,
            address_map_entries: Vec::with_capacity(0),
            total_used: 0,
        };

        db.read_total_used()?;
        db.read_entry_alloc()?;

        // eprintln!("{:#?}", db);

        Ok(db)
    }

    pub fn check_or_grow(&mut self, idx: usize) {
        if self.address_map_entries.len() <= idx {
            self.address_map_entries
                .resize_with(idx + 1, || AddressEntry::default());
        }
    }

    pub fn address_entries_byte_size(&self) -> usize {
        self.address_map_entries.capacity() * AddressEntry::BYTE_SIZE
    }

    pub fn address_entries_page_size(&self) -> usize {
        self.address_entries_byte_size() >> 12
    }

    pub fn is_entry_address(&self, address: usize) -> bool {
        let start = 4096;
        let end = start + self.address_entries_byte_size();
        address >= start && address < end
    }

    pub fn is_entry_page(&self, page_num: usize) -> bool {
        let start = 1;
        let end = start + self.address_entries_page_size();
        page_num >= start && page_num < end
    }
    pub fn is_header_page(&self, page_num: usize) -> bool {
        page_num == 0
    }
    pub const FILE_TYPE_OFFSET: usize = 0;
    pub const FILE_VERSION_OFFSET: usize = 8;
    pub const MOD_COUNT_OFFSET: usize = 16;
    pub const TOTAL_USED_OFFSET: usize = 24;
    pub const ENTRY_ALLOC_OFFSET: usize = 32;

    pub fn address_map_bound(&self) -> usize {
        4096 + self.address_entries_byte_size()
    }

    pub fn sync_address_map(&mut self) -> Result<(), ()> {
        self.cache_sectors(0..self.address_map_bound())
    }

    pub fn wal_file_version(&mut self) -> Result<(usize, usize), ()> {
        let mut buf = [0u8; 8];
        let bytes_read = self.wal_file.read_at(&mut buf, 0).map_err(|_| ())?;
        if bytes_read == 0 {
            return Ok((0, 0));
        }
        if bytes_read != 8 {
            return Err(());
        }
        let commit = <usize>::from_le_bytes(buf);

        let bytes_read = self.wal_file.read_at(&mut buf, 8).map_err(|_| ())?;
        if bytes_read != 8 {
            return Err(());
        }

        let ledger_version = <usize>::from_le_bytes(buf);

        Ok((commit, ledger_version))
    }

    pub fn page_iter<
        F: FnMut(&mut Self, Arc<RwLock<Page>>, usize, usize, usize) -> Result<(), ()>,
    >(
        &mut self,
        start_address: usize,
        end_address: usize,
        mut f: F,
    ) -> Result<(), ()> {
        let start_page = start_address >> 12;
        let start_offset = start_address % 4096;
        let end_page = end_address >> 12;
        let end_offset = end_address % 4096;

        for page_num in start_page..=end_page {
            let page = match self.page_buf.get(&page_num) {
                Some(page) => page.clone(),
                None => {
                    let mut page_buf = [0u8; 4096];
                    self.db_file
                        .read_at(&mut page_buf, (page_num << 12) as u64)
                        .map_err(|_| ())?;
                    let page = Arc::new(RwLock::new(page_buf));
                    self.page_buf.insert(page_num, page.clone());
                    page
                }
            };
            let start_offset = if page_num == start_page {
                start_offset
            } else {
                0
            };

            let end_offset = if page_num == end_page {
                end_offset
            } else {
                4096
            };
            f(self, page, page_num, start_offset, end_offset)?;
        }

        Ok(())
    }

    pub fn sync_wal(&mut self) -> Result<(), ()> {
        self.db_file.lock_shared().map_err(|_| ())?;
        let (commit, version) = self.wal_file_version()?;

        if commit > self.commit {
            self.commit = commit;
            self.ledger_version = 0;
            self.wal_file.seek(SeekFrom::Start(16)).map_err(|_| ())?;
            self.page_buf.clear();
        }

        let start_v = self.ledger_version + 1;
        for _ in start_v..version {
            let mut buf = [0u8; 16];
            let bytes_read = self.wal_file.read(&mut buf).map_err(|_| ())?;
            if bytes_read != buf.len() {
                return Err(());
            }

            // TODO: const range trait stuff
            let address_b =
                <[u8; 8]>::try_from(&buf[0..8]).expect("guarenteed 0..8 slice failed to convert");
            let size_b =
                <[u8; 8]>::try_from(&buf[8..16]).expect("guarenteed 8..16 slice failed to convert");

            let start_address = <usize>::from_le_bytes(address_b);

            let size = <usize>::from_le_bytes(size_b);
            let end_address = start_address + size;

            self.page_iter(
                start_address,
                end_address,
                |this, page, page_num, start_offset, end_offset| {
                    page.write()
                        .ok()
                        .map(|mut page| {
                            if !this.handle_special_pages(
                                page_num,
                                &page,
                                start_offset,
                                end_offset,
                            )? {
                                this.wal_file
                                    .read(&mut page[start_offset..end_offset])
                                    .map_err(|_| ())?;
                            }
                            Ok(())
                        })
                        .unwrap_or(Err(()))
                },
            )?;
        }
        self.ledger_version = version;
        self.db_file.unlock().map_err(|_| ())?;

        Ok(())
    }

    pub fn set_total_used(&mut self, total_used: usize) -> Result<(), ()> {
        let total_used_b = total_used.to_le_bytes();
        self.write_at(&total_used_b, Self::TOTAL_USED_OFFSET)
    }

    pub fn read_total_used(&mut self) -> Result<(), ()> {
        let default_total_used = (128 * AddressEntry::BYTE_SIZE) + 4096;
        let mut total_used_b = [0u8; 8];
        let bytes_read = self.read_at(&mut total_used_b, Self::TOTAL_USED_OFFSET)?;
        self.total_used = if bytes_read != 8 {
            default_total_used
        } else {
            <usize>::from_le_bytes(total_used_b)
        };
        Ok(())
    }

    pub fn resize_entry_alloc(&mut self) -> Result<Vec<AddressEntry>, ()> {
        let new_size = self.address_map_entries.capacity() * 2;
        let mut new_entries = Vec::with_capacity(new_size);
        new_entries.resize_with(new_size, || AddressEntry::default());

        std::mem::swap(&mut self.address_map_entries, &mut new_entries);
        self.write_at(&new_size.to_le_bytes(), Self::ENTRY_ALLOC_OFFSET)?;

        Ok(new_entries)
    }

    pub fn read_entry_alloc(&mut self) -> Result<(), ()> {
        let mut entry_alloc_b = [0u8; 8];
        let bytes_read = self.read_at(&mut entry_alloc_b, Self::ENTRY_ALLOC_OFFSET)?;

        let size = if bytes_read != 8 {
            128
        } else {
            let tmp_size = <usize>::from_le_bytes(entry_alloc_b);
            if tmp_size == 0 { 128 } else { tmp_size }
        };

        eprintln!("size: {}", size);
        let mut new_entries = Vec::with_capacity(size);
        new_entries.resize_with(size, || AddressEntry::default());
        std::mem::swap(&mut self.address_map_entries, &mut new_entries);

        Ok(())
    }

    pub fn handle_special_pages(
        &mut self,
        page_num: usize,
        page: &Page,
        start_offset: usize,
        end_offset: usize,
    ) -> Result<bool, ()> {
        let is_entry = self.is_entry_page(page_num);
        if is_entry {
            if (end_offset - start_offset) % AddressEntry::BYTE_SIZE > 0 {
                return Err(());
            }
            let idx_offset = start_offset / AddressEntry::BYTE_SIZE;
            for (i, chunk) in page[start_offset..end_offset]
                .chunks(AddressEntry::BYTE_SIZE)
                .enumerate()
            {
                let idx = ((page_num - 1) * (4096 / 32)) + i + idx_offset;
                let chunk = <[u8; 32]>::try_from(chunk)
                    .expect("32 byte chunk assertion failed for address map entry");
                self.address_map_entries[idx] = AddressEntry::from_bytes(chunk);
            }
        } else if self.is_header_page(page_num) {
            let is_modifying_total_used = start_offset <= Self::TOTAL_USED_OFFSET
                && end_offset >= Self::TOTAL_USED_OFFSET + 8;
            if is_modifying_total_used {
                let chunk = <[u8; 8]>::try_from(
                    &page[Self::TOTAL_USED_OFFSET..Self::TOTAL_USED_OFFSET + 8],
                )
                .expect("8 byte chunk assertion failed for total_used field");
                self.total_used = <usize>::from_le_bytes(chunk);
            }
        }
        Ok(is_entry)
    }

    pub fn cache_sectors(&mut self, address_range: std::ops::Range<usize>) -> Result<(), ()> {
        self.wal_file.lock_shared().map_err(|_| ())?;
        self.sync_wal()?;
        self.db_file.lock_shared().map_err(|_| ())?;

        // This will force the page range into the cache
        self.page_iter(
            address_range.start,
            address_range.end,
            |this, page, page_num, start_offset, end_offset| {
                page.read()
                    .ok()
                    .map(|page| {
                        this.handle_special_pages(page_num, &*page, start_offset, end_offset)?;
                        Ok(())
                    })
                    .unwrap_or(Err(()))
            },
        )?;

        self.db_file.unlock().map_err(|_| ())?;
        self.wal_file.unlock().map_err(|_| ())
    }

    pub fn read_at(&mut self, buf: &mut [u8], start_address: usize) -> Result<usize, ()> {
        if self.page_buf.len() > 700 {
            self.page_buf.clear();
        }
        self.wal_file.lock_shared().map_err(|_| ())?;
        self.sync_wal()?;
        self.db_file.lock_shared().map_err(|_| ())?;

        let end_address = start_address + buf.len();

        let mut buf_idx = 0;
        self.page_iter(
            start_address,
            end_address,
            |this, page, page_num, start_offset, end_offset| {
                page.read()
                    .ok()
                    .map(|page| {
                        this.handle_special_pages(page_num, &page, start_offset, end_offset)?;
                        for b in page[start_offset..end_offset].iter() {
                            buf[buf_idx] = *b;
                            buf_idx += 1;
                        }
                        Ok(())
                    })
                    .unwrap_or(Err(()))
            },
        )?;

        self.db_file.unlock().map_err(|_| ())?;
        self.wal_file.unlock().map_err(|_| ())?;
        Ok(buf_idx)
    }

    pub fn move_data(
        &mut self,
        from_address: usize,
        to_address: usize,
        size: usize,
    ) -> Result<(), ()> {
        let mut buf = vec![0u8; size];
        if self.page_buf.len() > 700 {
            self.page_buf.clear();
        }
        self.wal_file.lock().map_err(|_| ())?;
        self.sync_wal()?;
        self.db_file.lock_shared().map_err(|_| ())?;

        let end_address = from_address + buf.len();

        let mut buf_idx = 0;
        self.page_iter(
            from_address,
            end_address,
            |_, page, _, start_offset, end_offset| {
                page.read()
                    .ok()
                    .map(|page| {
                        for b in page[start_offset..end_offset].iter() {
                            buf[buf_idx] = *b;
                            buf_idx += 1;
                        }
                        Ok(())
                    })
                    .unwrap_or(Err(()))
            },
        )?;

        self.db_file.unlock().map_err(|_| ())?;
        self.ledger_version += 1;
        self.wal_file
            .write_at(&self.ledger_version.to_le_bytes(), 8)
            .map_err(|_| ())?;
        self.wal_file
            .write(&to_address.to_le_bytes())
            .map_err(|_| ())?;
        self.wal_file
            .write(&buf.len().to_le_bytes())
            .map_err(|_| ())?;
        self.wal_file.write(&buf).map_err(|_| ())?;

        self.ledger_version += 1;
        let buf = vec![0u8; size]; // TODO, there is a better way to do this part

        self.wal_file
            .write(&to_address.to_le_bytes())
            .map_err(|_| ())?;
        self.wal_file
            .write(&buf.len().to_le_bytes())
            .map_err(|_| ())?;
        self.wal_file.write(&buf).map_err(|_| ())?;

        if self.ledger_version > 100 {
            self.flush_wal()?;
        }

        self.wal_file.unlock().map_err(|_| ())
    }

    pub fn write_at(&mut self, buf: &[u8], start_address: usize) -> Result<(), ()> {
        self.wal_file.lock().map_err(|_| ())?;
        self.sync_wal()?;
        self.ledger_version += 1;
        self.wal_file
            .write_at(&self.ledger_version.to_le_bytes(), 8)
            .map_err(|_| ())?;
        self.wal_file
            .write(&start_address.to_le_bytes())
            .map_err(|_| ())?;
        self.wal_file
            .write(&buf.len().to_le_bytes())
            .map_err(|_| ())?;
        self.wal_file.write(buf).map_err(|_| ())?;

        let size = buf.len();
        let end_address = start_address + size;

        self.page_iter(
            start_address,
            end_address,
            |this, page, page_num, start_offset, end_offset| {
                page.write()
                    .ok()
                    .map(|mut page| {
                        let mut buf_idx = 0;
                        for idx in start_offset..end_offset {
                            page[idx] = buf[buf_idx];
                            buf_idx += 1;
                        }
                        this.handle_special_pages(page_num, &page, start_offset, end_offset)?;
                        Ok(())
                    })
                    .unwrap_or(Err(()))
            },
        )?;

        if self.ledger_version > 100 {
            self.flush_wal()?;
        }
        self.wal_file.unlock().map_err(|_| ())
    }

    pub fn flush_wal(&mut self) -> Result<(), ()> {
        self.db_file.lock().map_err(|_| ())?;
        self.wal_file.seek(SeekFrom::Start(16)).map_err(|_| ())?;
        for _ in 0..self.ledger_version {
            let mut buf = [0u8; 16];
            let bytes_read = self.wal_file.read(&mut buf).map_err(|_| ())?;
            if bytes_read != buf.len() {
                return Err(());
            }

            // TODO: const range trait stuff
            let address_b =
                <[u8; 8]>::try_from(&buf[0..8]).expect("guarenteed 0..8 slice failed to convert");
            let size_b =
                <[u8; 8]>::try_from(&buf[8..16]).expect("guarenteed 8..16 slice failed to convert");

            let start_address = <usize>::from_le_bytes(address_b);
            let size = <usize>::from_le_bytes(size_b);
            let end_address = start_address + size;

            self.page_iter(
                start_address,
                end_address,
                |this, page, page_num, start_offset, end_offset| {
                    page.write()
                        .ok()
                        .map(|mut page| {
                            this.wal_file
                                .read(&mut page[start_offset..end_offset])
                                .map_err(|_| ())?;
                            let phys_addr = (page_num << 12) + start_offset;
                            this.db_file
                                .write_at(&mut page[start_offset..end_offset], phys_addr as u64)
                                .map_err(|_| ())?;
                            Ok(())
                        })
                        .unwrap_or(Err(()))
                },
            )?;
        }
        self.commit = 0;
        self.ledger_version = 0;

        self.wal_file.set_len(16).map_err(|_| ())?;
        self.wal_file.seek(SeekFrom::Start(0)).map_err(|_| ())?;
        self.wal_file
            .write(&self.commit.to_le_bytes())
            .map_err(|_| ())?;
        self.wal_file
            .write(&self.ledger_version.to_le_bytes())
            .map_err(|_| ())?;

        self.db_file.unlock().map_err(|_| ())
    }
}

pub struct FileRW {
    file: Arc<RwLock<File>>,
}

impl FileRW {
    pub fn new(path: &Path) -> Result<Self, ()> {
        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(path)
            .map_err(|_| ())?;
        file.unlock().map_err(|_| ())?;

        let file = Arc::new(RwLock::new(file));
        Ok(FileRW { file })
    }

    pub fn aquire(&self) -> Arc<RwLock<File>> {
        self.file.clone()
    }

    pub fn read<T, F: FnMut(&File) -> Result<T, ()>>(&self, mut f: F) -> Result<T, ()> {
        match self.file.read() {
            Ok(file) => {
                file.lock_shared().map_err(|_| ())?;
                let t = f(&file); // handle errs after release of file
                file.unlock().map_err(|_| ())?;
                Ok(t?)
            }
            Err(_) => Err(()),
        }
    }

    pub fn read_mut<T, F: FnMut(&mut File) -> Result<T, ()>>(&self, mut f: F) -> Result<T, ()> {
        match self.file.write() {
            Ok(mut file) => {
                file.lock_shared().map_err(|_| ())?;
                let t = f(&mut file);
                file.unlock().map_err(|_| ())?;
                Ok(t?)
            }
            Err(_) => Err(()),
        }
    }

    pub fn write_mut<T, F: FnMut(&mut File) -> Result<T, ()>>(&self, mut f: F) -> Result<T, ()> {
        match self.file.write() {
            Ok(mut file) => {
                file.lock().map_err(|_| ())?;
                let t = f(&mut file);
                file.unlock().map_err(|_| ())?;
                Ok(t?)
            }
            Err(_) => Err(()),
        }
    }
}

use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
};
use steadfast_bytes::{ByteSize, FromBytes, ToBytes};
use steadfast_uuid::UUID;

const RESERVED_PAGE_BYTES: usize = 16;

#[repr(transparent)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageAddr<const PAGE_SIZE: usize>(u64);
impl<const PAGE_SIZE: usize> PageAddr<PAGE_SIZE> {
    pub fn new(page_addr: u64) -> Self {
        assert!(page_addr % PAGE_SIZE as u64 == 0);
        Self(page_addr)
    }
}

pub trait PageBuffer<const PAGE_SIZE: usize>: Sized {
    fn to_page_buffer(&self) -> (PageAddr<PAGE_SIZE>, [u8; PAGE_SIZE]);
    fn from_page_buffer(
        page_addr: PageAddr<PAGE_SIZE>,
        page_buf: [u8; PAGE_SIZE],
    ) -> Result<Self, IndexErr>;
    fn set_page_addr(&mut self, page_addr: PageAddr<PAGE_SIZE>);
    fn alloc_node(
        &mut self,
        b_tree: &mut BTreeIndex<PAGE_SIZE>,
    ) -> Result<PageAddr<PAGE_SIZE>, IndexErr> {
        let file_len = b_tree
            .file
            .seek(SeekFrom::End(0))
            .map_err(|_| IndexErr::FailedToRead)?;

        self.set_page_addr(PageAddr::new(file_len));

        let (_, page_buf) = self.to_page_buffer();

        b_tree
            .file
            .write_all(&page_buf)
            .map_err(|_| IndexErr::FailedToWrite)?;

        Ok(PageAddr::new(file_len))
    }
    fn write_node(
        &mut self,
        b_tree: &mut BTreeIndex<PAGE_SIZE>,
    ) -> Result<PageAddr<PAGE_SIZE>, IndexErr> {
        let (page_addr, page_buf) = self.to_page_buffer();

        b_tree
            .file
            .seek(SeekFrom::Start(page_addr.0))
            .map_err(|_| IndexErr::FailedToWrite)?;
        b_tree
            .file
            .write_all(&page_buf)
            .map_err(|_| IndexErr::FailedToWrite)?;

        Ok(page_addr)
    }
    fn first(&self) -> UUID;
    fn page_addr(&self) -> PageAddr<PAGE_SIZE>;
}

impl<const PAGE_SIZE: usize> PageBuffer<PAGE_SIZE> for Leaf<PAGE_SIZE> {
    fn first(&self) -> UUID {
        self.entries[0]
    }
    fn page_addr(&self) -> PageAddr<PAGE_SIZE> {
        self.page_addr
    }
    fn to_page_buffer(&self) -> (PageAddr<PAGE_SIZE>, [u8; PAGE_SIZE]) {
        let mut page_buf = [0u8; PAGE_SIZE];
        page_buf[0] = Node::<PAGE_SIZE>::LEAF;
        if let Some(PageAddr(next_leaf_addr)) = self.next_leaf_addr {
            for (i, b) in next_leaf_addr.to_bytes_le().iter().enumerate() {
                page_buf[<u64>::BYTE_SIZE + i] = *b;
            }
        }
        let entry_buf = &mut page_buf[RESERVED_PAGE_BYTES..];

        for (i, uuid) in self.entries.iter().enumerate() {
            for (j, b) in uuid.to_bytes_le().iter().enumerate() {
                entry_buf[(i * Self::ENTRY_SIZE) + j] = *b;
            }
        }

        (self.page_addr, page_buf)
    }
    fn from_page_buffer(
        page_addr: PageAddr<PAGE_SIZE>,
        page_buf: [u8; PAGE_SIZE],
    ) -> Result<Self, IndexErr> {
        let next_leaf_addr_buf = page_buf[<u64>::BYTE_SIZE..RESERVED_PAGE_BYTES]
            .first_chunk::<8>()
            .expect("Failed to pull next_leaf_addr_buf from page_buf.");
        let next_leaf_addr = match <u64>::from_bytes_le(*next_leaf_addr_buf) {
            i if i > 0 => Some(PageAddr::new(i)),
            _ => None,
        };
        let entries = page_buf[RESERVED_PAGE_BYTES..]
            .chunks_exact(Self::ENTRY_SIZE)
            .map_while(|chunk| {
                if chunk != Self::NULL_ENTRY {
                    Some(chunk)
                } else {
                    None
                }
            })
            .fold(
                Vec::with_capacity(Self::MAX_ENTRIES),
                |mut entries, chunk| {
                    // dbg!(chunk);
                    let uuid_buf = chunk
                        .first_chunk::<{ UUID::BYTE_SIZE }>()
                        .expect("Failed to pull UUID chunk from entry_buf");
                    entries.push(<UUID>::from_bytes_le(*uuid_buf));
                    entries
                },
            );

        Ok(Leaf {
            page_addr,
            entries,
            next_leaf_addr,
        })
    }
    fn set_page_addr(&mut self, page_addr: PageAddr<PAGE_SIZE>) {
        self.page_addr = page_addr;
    }
}
impl<const PAGE_SIZE: usize> PageBuffer<PAGE_SIZE> for Branch<PAGE_SIZE> {
    fn first(&self) -> UUID {
        self.entries[0].0
    }
    fn page_addr(&self) -> PageAddr<PAGE_SIZE> {
        self.page_addr
    }
    fn to_page_buffer(&self) -> (PageAddr<PAGE_SIZE>, [u8; PAGE_SIZE]) {
        let mut page_buf = [0u8; PAGE_SIZE];
        page_buf[0] = Node::<PAGE_SIZE>::BRANCH;
        for (i, b) in self.last_page.0.to_bytes_le().iter().enumerate() {
            page_buf[<u64>::BYTE_SIZE + i] = *b;
        }
        let entry_buf = &mut page_buf[RESERVED_PAGE_BYTES..];

        for (i, (uuid, PageAddr(sub_node_addr))) in self.entries.iter().enumerate() {
            for (j, b) in uuid.to_bytes_le().iter().enumerate() {
                entry_buf[(i * Self::ENTRY_SIZE) + j] = *b;
            }
            for (j, b) in sub_node_addr.to_bytes_le().iter().enumerate() {
                entry_buf[(i * Self::ENTRY_SIZE) + j + UUID::BYTE_SIZE] = *b;
            }
        }

        (self.page_addr, page_buf)
    }
    fn from_page_buffer(
        page_addr: PageAddr<PAGE_SIZE>,
        page_buf: [u8; PAGE_SIZE],
    ) -> Result<Self, IndexErr> {
        let last_page_buf = page_buf[<u64>::BYTE_SIZE..RESERVED_PAGE_BYTES]
            .first_chunk::<8>()
            .expect("Failed to pull last_page_buf from page_buf.");
        let last_page = PageAddr::new(<u64>::from_bytes_le(*last_page_buf));
        assert!(
            last_page.0 > 0,
            "A page branch must always have a last_page."
        );
        let entries = page_buf[RESERVED_PAGE_BYTES..]
            .chunks_exact(Self::ENTRY_SIZE)
            .map_while(|chunk| {
                if chunk != Self::NULL_ENTRY {
                    Some(chunk)
                } else {
                    None
                }
            })
            .fold(
                Vec::with_capacity(Self::MAX_ENTRIES),
                |mut entries, chunk| {
                    let uuid_buf = chunk
                        .first_chunk::<{ UUID::BYTE_SIZE }>()
                        .expect("Failed to pull UUID chunk from entry_buf");
                    let addr_buf = chunk[UUID::BYTE_SIZE..]
                        .first_chunk::<{ <u64>::BYTE_SIZE }>()
                        .expect("Failed to pull addr_buf chunk from entry_buf");
                    entries.push((
                        <UUID>::from_bytes_le(*uuid_buf),
                        PageAddr::new(<u64>::from_bytes_le(*addr_buf)),
                    ));
                    entries
                },
            );
        Ok(Branch {
            page_addr,
            entries,
            last_page,
        })
    }
    fn set_page_addr(&mut self, page_addr: PageAddr<PAGE_SIZE>) {
        self.page_addr = page_addr;
    }
}
impl<const PAGE_SIZE: usize> PageBuffer<PAGE_SIZE> for Node<PAGE_SIZE> {
    fn first(&self) -> UUID {
        match self {
            Node::Branch(branch) => branch.first(),
            Node::Leaf(leaf) => leaf.first(),
            Node::None => unreachable!("We should never be calling first from a none node."),
        }
    }
    fn page_addr(&self) -> PageAddr<PAGE_SIZE> {
        match self {
            Node::Branch(branch) => branch.page_addr(),
            Node::Leaf(leaf) => leaf.page_addr(),
            Node::None => unreachable!("None cannot have a page addr."),
        }
    }
    fn to_page_buffer(&self) -> (PageAddr<PAGE_SIZE>, [u8; PAGE_SIZE]) {
        match self {
            Node::Branch(branch) => branch.to_page_buffer(),
            Node::Leaf(leaf) => leaf.to_page_buffer(),
            Node::None => unreachable!("We should never be converting empty node to bytes."),
        }
    }
    fn from_page_buffer(
        page_addr: PageAddr<PAGE_SIZE>,
        page_buf: [u8; PAGE_SIZE],
    ) -> Result<Self, IndexErr> {
        match page_buf[0] {
            Node::<PAGE_SIZE>::NONE => Ok(Node::None),
            Node::<PAGE_SIZE>::BRANCH => {
                Ok(Node::Branch(Branch::from_page_buffer(page_addr, page_buf)?))
            }
            Node::<PAGE_SIZE>::LEAF => Ok(Node::Leaf(Leaf::from_page_buffer(page_addr, page_buf)?)),
            _ => Err(IndexErr::UnknownNodeType),
        }
    }
    fn set_page_addr(&mut self, page_addr: PageAddr<PAGE_SIZE>) {
        match self {
            Node::Branch(branch) => {
                branch.set_page_addr(page_addr);
            }
            Node::Leaf(leaf) => {
                leaf.set_page_addr(page_addr);
            }
            Node::None => {}
        }
    }
}

// impl UUID {
//     // const FLAG_MASK: u128 = 0xF000000000000000;
//     // const ALLOC: u128 = 0x1000000000000000;
//     pub fn from_u128(val: u64) -> Self {
//         //TODO: this is to simulate a timestamp always being greater than zero for now
//         // let val = Self::ALLOC | val as u128;
//         Self(val as u128)
//     }

//     pub const fn from_sf_le_bytes(bytes: [u8; UUID::BYTE_SIZE]) -> Self {
//         Self(<u128>::from_sf_le_bytes(bytes))
//     }

//     pub const fn to_bytes_le(&self) -> [u8; UUID::BYTE_SIZE] {
//         self.0.to_bytes_le()
//     }
// }

pub struct Lookup {
    entries: HashMap<UUID, String>,
}

impl Lookup {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: UUID) {
        let val = format!("this is the key: {}", &key);
        self.entries.insert(key, val);
    }

    pub fn get(&self, key: &UUID) -> Option<&String> {
        self.entries.get(key)
    }

    pub fn remove(&mut self, key: &UUID) {
        self.entries.remove(key);
    }
}

#[derive(Debug)]
pub enum IndexErr {
    EOF,
    FailedToRead,
    FailedToWrite,
    FailedToOpen,
    EntryCountLargerThanPage,
    InvalidRecordUUID,
    AllocAttemptOnEmptyNode,
    UnknownNodeType,
    CannotSplitEmptyNode,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Branch<const PAGE_SIZE: usize> {
    page_addr: PageAddr<PAGE_SIZE>,
    entries: Vec<(UUID, PageAddr<PAGE_SIZE>)>,
    last_page: PageAddr<PAGE_SIZE>,
}
impl<const PAGE_SIZE: usize> Branch<PAGE_SIZE> {
    const ENTRY_SIZE: usize = (UUID::BYTE_SIZE + <u64>::BYTE_SIZE);
    const NULL_ENTRY: [u8; UUID::BYTE_SIZE + <u64>::BYTE_SIZE] =
        [0; UUID::BYTE_SIZE + <u64>::BYTE_SIZE];
    const MAX_ENTRIES: usize = (PAGE_SIZE - RESERVED_PAGE_BYTES) / Self::ENTRY_SIZE;
    pub fn new(
        b_tree: &mut BTreeIndex<PAGE_SIZE>,
        side_a: &mut impl PageBuffer<PAGE_SIZE>,
        side_b: &mut impl PageBuffer<PAGE_SIZE>,
    ) -> Result<Self, IndexErr> {
        let page_addr = side_a.page_addr();
        let side_a_addr = side_a.alloc_node(b_tree)?;
        let mut entries = Vec::with_capacity(PAGE_SIZE);
        entries.push((side_b.first(), side_a_addr));
        let mut new_branch = Branch {
            page_addr,
            entries,
            last_page: side_b.page_addr(),
        };
        new_branch.write_node(b_tree)?;
        Ok(new_branch)
    }
    pub fn insert(
        &mut self,
        b_tree: &mut BTreeIndex<PAGE_SIZE>,
        uuid: UUID,
        val: u64,
    ) -> Result<Option<Branch<PAGE_SIZE>>, IndexErr> {
        println!("hit 2");
        if self.entries.len() == self.entries.capacity() {
            println!("hit 2.1");
            Ok(Some(self.split(b_tree)?))
        } else {
            println!("hit 2.3");
            for (sub_uuid, sub_page_addr) in &self.entries {
                match b_tree.val_map.get(&sub_uuid) {
                    Some(sub_val) => {
                        if &val < sub_val {
                            println!("hit 2.4");
                            let mut sub_node = b_tree.read_node(*sub_page_addr)?;
                            sub_node.insert(b_tree, *sub_page_addr, uuid, val)?;
                            return Ok(None);
                        }
                    }
                    None => {}
                }
            }

            println!("hit 2.5");
            let mut sub_node = b_tree.read_node(self.last_page)?;
            sub_node.insert(b_tree, self.last_page, uuid, val)
        }
    }

    pub fn sorted_entry_insert(
        &mut self,
        b_tree: &BTreeIndex<PAGE_SIZE>,
        max_uuid: UUID,
        page_addr: PageAddr<PAGE_SIZE>,
        new_leaf_addr: PageAddr<PAGE_SIZE>,
    ) {
        if let Some(val) = b_tree.val_map.get(&max_uuid) {
            self.entries.push((max_uuid, page_addr));
            for i in self.entries.len() - 1..=1 {
                let (uuid_b, _sub_node_addr) = self.entries[i];
                match b_tree.val_map.get(&uuid_b) {
                    Some(val_b) => {
                        if val_b > &val {
                            self.entries.swap(i, i - 1);
                        } else {
                            break;
                        }
                    }
                    None => {
                        self.entries.swap(i, i - 1);
                    }
                }
            }
            if self.last_page == page_addr {
                self.last_page = new_leaf_addr;
            }
        }
    }

    pub fn split(
        &mut self,
        b_tree: &mut BTreeIndex<PAGE_SIZE>,
    ) -> Result<Branch<PAGE_SIZE>, IndexErr> {
        let new_branch_entries = self.entries.split_off((self.entries.len() + 1) / 2);

        let new_branch_node = Branch::<PAGE_SIZE> {
            page_addr: PageAddr::default(),
            entries: new_branch_entries,
            last_page: self.last_page,
        };

        let (_last_entry_uuid, last_entry_addr) = self
            .entries
            .pop()
            .expect("Node should always have at least one entry during split");

        self.last_page = last_entry_addr;
        // self.write_node(b_tree)?;
        Ok(new_branch_node)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Leaf<const PAGE_SIZE: usize> {
    pub page_addr: PageAddr<PAGE_SIZE>,
    pub entries: Vec<UUID>,
    pub next_leaf_addr: Option<PageAddr<PAGE_SIZE>>,
}

impl<const PAGE_SIZE: usize> Leaf<PAGE_SIZE> {
    const ENTRY_SIZE: usize = UUID::BYTE_SIZE;
    const NULL_ENTRY: [u8; UUID::BYTE_SIZE] = [0; UUID::BYTE_SIZE];
    const MAX_ENTRIES: usize = (PAGE_SIZE - RESERVED_PAGE_BYTES) / Self::ENTRY_SIZE;

    pub fn insert(
        &mut self,
        b_tree: &mut BTreeIndex<PAGE_SIZE>,
        uuid: UUID,
        val: u64,
    ) -> Result<Option<Leaf<PAGE_SIZE>>, IndexErr> {
        println!("hit 3");
        Ok(if self.entries.len() == self.entries.capacity() {
            println!("hit 3.1");
            Some(self.split(b_tree)?)
        } else if self.entries.len() > 0 {
            println!("hit 3.2");
            self.sorted_entry_insert(b_tree, uuid, val);
            self.write_node(b_tree)?;
            None
        } else {
            println!("hit 3.3");
            self.entries.push(uuid);
            self.write_node(b_tree)?;
            None
        })
    }

    pub fn sorted_entry_insert(
        &mut self,
        b_tree: &mut BTreeIndex<PAGE_SIZE>,
        uuid: UUID,
        val: u64,
    ) {
        println!("hit sorted insert");
        self.entries.push(uuid);
        println!("{:#?}", self);
        println!("{:#?}", self.entries.len() - 1..=0);
        for i in (1..self.entries.len()).rev() {
            println!("hit uuid_b");
            let uuid_b = self.entries[i - 1];
            match b_tree.val_map.get(&uuid_b) {
                Some(val_b) => {
                    println!("a:{}>{}", self.entries[i - 1], self.entries[i]);
                    if val_b > &val {
                        self.entries.swap(i, i - 1);
                    } else {
                        break;
                    }
                }

                None => {
                    println!("b:{}>{}", self.entries[i - 1], self.entries[i]);
                    self.entries.swap(i, i - 1);
                }
            }
        }
    }

    pub fn split(
        &mut self,
        b_tree: &mut BTreeIndex<PAGE_SIZE>,
    ) -> Result<Leaf<PAGE_SIZE>, IndexErr> {
        let new_leaf_entries = self.entries.split_off((self.entries.len() + 1) / 2);

        let mut new_leaf_node = Leaf::<PAGE_SIZE> {
            page_addr: PageAddr::default(),
            entries: new_leaf_entries,
            next_leaf_addr: self.next_leaf_addr,
        };

        self.next_leaf_addr = Some(new_leaf_node.alloc_node(b_tree)?);
        // self.write_node(b_tree)?;
        Ok(new_leaf_node)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Node<const PAGE_SIZE: usize> {
    Branch(Branch<PAGE_SIZE>),
    Leaf(Leaf<PAGE_SIZE>),
    None,
}

impl<const PAGE_SIZE: usize> Node<PAGE_SIZE> {
    pub const NONE: u8 = 0;
    pub const BRANCH: u8 = 1;
    pub const LEAF: u8 = 2;

    pub fn is_none(&self) -> bool {
        match self {
            Node::None => true,
            _ => false,
        }
    }
    pub fn insert(
        &mut self,
        b_tree: &mut BTreeIndex<PAGE_SIZE>,
        page_addr: PageAddr<PAGE_SIZE>,
        uuid: UUID,
        val: u64,
    ) -> Result<Option<Branch<PAGE_SIZE>>, IndexErr> {
        println!("hit 1");
        match self {
            Node::Branch(branch) => match branch.insert(b_tree, uuid, val)? {
                Some(ref mut new_branch) => {
                    let mut parent_branch = Branch::new(b_tree, branch, new_branch)?;
                    parent_branch.insert(b_tree, uuid, val)
                }
                None => Ok(None),
            },
            Node::Leaf(leaf) => match leaf.insert(b_tree, uuid, val)? {
                Some(ref mut new_leaf) => {
                    let mut parent_branch = Branch::new(b_tree, leaf, new_leaf)?;
                    parent_branch.insert(b_tree, uuid, val)
                }
                None => Ok(None),
            },
            Node::None => {
                println!("page_addr: {:#?}", page_addr);
                b_tree.insert_from_none(page_addr, uuid)?;
                Ok(None)
            }
        }
    }
}

#[derive(Debug)]
pub enum InsertOp<const PAGE_SIZE: usize> {
    LeafHadRoom {
        had_mutation: bool,
    },
    RequestingSplit {
        had_mutation: bool,
        min_uuid: UUID,
        new_node: Node<PAGE_SIZE>,
    },
    NothingToDo,
}

#[derive(Debug)]
pub struct BTreeIndex<'a, const PAGE_SIZE: usize> {
    file: &'a mut File,
    val_map: HashMap<UUID, u64>,
}

impl<'a, const PAGE_SIZE: usize> BTreeIndex<'a, PAGE_SIZE> {
    const COMPTIME_SIZE_CHECK: () = assert!(
        PAGE_SIZE >= 64 && (PAGE_SIZE - 1 < (PAGE_SIZE ^ (PAGE_SIZE - 1))),
        "PAGE_SIZE must be greater than 64 and a power of 2."
    );
    pub fn open_idx_file(path: &str) -> Result<File, IndexErr> {
        let _ = Self::COMPTIME_SIZE_CHECK;
        let db_path = Path::new(path);

        OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .open(&db_path)
            .map_err(|_| IndexErr::FailedToOpen)
    }
    pub fn truncate_idx_file(path: &str) -> Result<File, IndexErr> {
        let _ = Self::COMPTIME_SIZE_CHECK;
        let db_path = Path::new(path);

        OpenOptions::new()
            .write(true)
            .truncate(true)
            .read(true)
            .create(true)
            .open(&db_path)
            .map_err(|_| IndexErr::FailedToOpen)
    }

    fn read_exact<const N: usize>(&mut self) -> Result<[u8; N], IndexErr> {
        let mut buf = [0u8; N];
        self.file
            .read_exact(&mut buf)
            .map_err(|err| match err.kind() {
                std::io::ErrorKind::UnexpectedEof => IndexErr::EOF,
                _ => IndexErr::FailedToRead,
            })?;

        Ok(buf)
    }

    pub fn new(idx_file: &'a mut File) -> Result<Self, IndexErr> {
        let _ = Self::COMPTIME_SIZE_CHECK;
        Ok(BTreeIndex {
            file: idx_file,
            val_map: HashMap::new(),
        })
    }

    pub fn read_node(
        &mut self,
        page_addr: PageAddr<PAGE_SIZE>,
    ) -> Result<Node<PAGE_SIZE>, IndexErr> {
        self.file
            .seek(SeekFrom::Start(page_addr.0))
            .map_err(|_| IndexErr::FailedToRead)?;

        let page_buf = match self.read_exact::<PAGE_SIZE>() {
            Ok(buf) => buf,
            Err(IndexErr::EOF) => return Ok(Node::None),
            Err(e) => return Err(e),
        };

        let node = Node::from_page_buffer(page_addr, page_buf);
        // eprintln!("{:#?}", node);

        node
    }

    pub fn write_node(&mut self, node: &Node<PAGE_SIZE>) -> Result<PageAddr<PAGE_SIZE>, IndexErr> {
        let (page_addr, page_buf) =
            <Node<PAGE_SIZE> as PageBuffer<PAGE_SIZE>>::to_page_buffer(node);

        self.file
            .seek(SeekFrom::Start(page_addr.0))
            .map_err(|_| IndexErr::FailedToWrite)?;
        self.file
            .write_all(&page_buf)
            .map_err(|_| IndexErr::FailedToWrite)?;

        Ok(page_addr)
    }

    fn eq_search_branch(
        &mut self,
        val: u64,
        entries: Vec<(UUID, PageAddr<PAGE_SIZE>)>,
        last_page: PageAddr<PAGE_SIZE>,
    ) -> Result<Option<UUID>, IndexErr> {
        for (uuid, sub_node_addr) in entries {
            if let Some(found_val) = self.val_map.get(&uuid) {
                if found_val == &val {
                    return Ok(Some(uuid));
                } else if &val < found_val {
                    return self.eq_search(sub_node_addr, val);
                }
            }
        }

        if last_page > PageAddr(0) {
            match self.read_node(last_page)? {
                Node::None => Ok(None),
                Node::Leaf(Leaf {
                    page_addr: _,
                    entries,
                    next_leaf_addr: _,
                }) => self.eq_search_leaf(val, entries),
                Node::Branch(Branch {
                    page_addr: _,
                    entries,
                    last_page,
                }) => self.eq_search_branch(val, entries, last_page),
            }
        } else {
            Ok(None)
        }
    }

    fn eq_search_leaf(&mut self, val: u64, entries: Vec<UUID>) -> Result<Option<UUID>, IndexErr> {
        for uuid in entries {
            if let Some(found_val) = self.val_map.get(&uuid) {
                if found_val == &val {
                    return Ok(Some(uuid));
                }
            }
        }
        Ok(None)
    }

    pub fn eq_search(
        &mut self,
        page_addr: PageAddr<PAGE_SIZE>,
        val: u64,
    ) -> Result<Option<UUID>, IndexErr> {
        match self.read_node(page_addr)? {
            Node::Branch(Branch {
                page_addr: _,
                entries,
                last_page,
            }) => self.eq_search_branch(val, entries, last_page),
            Node::Leaf(Leaf {
                page_addr: _,
                entries,
                next_leaf_addr: _,
            }) => self.eq_search_leaf(val, entries),
            Node::None => Ok(None),
        }
    }

    pub fn insert_from_none(
        &mut self,
        page_addr: PageAddr<PAGE_SIZE>,
        uuid: UUID,
    ) -> Result<(), IndexErr> {
        println!("hit 4");
        // new node is dropped after write. No point in wasting mem
        let mut entries = Vec::with_capacity(1);
        entries.push(uuid);

        let mut new_node = Node::Leaf(Leaf {
            page_addr,
            entries,
            next_leaf_addr: None,
        });

        new_node.alloc_node(self)?;

        Ok(())
    }

    pub fn insert(
        &mut self,
        page_addr: PageAddr<PAGE_SIZE>,
        uuid: UUID,
        val: u64,
    ) -> Result<(), IndexErr> {
        println!("hit 0:{:#?}", uuid);
        self.val_map.insert(uuid, val);
        let mut node = self.read_node(page_addr)?;
        node.insert(self, page_addr, uuid, val)?;
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     #[test]
//     fn insert_from_none_test() {
//         let mut idx_file = BTreeIndex::<64>::truncate_idx_file("./target/test1.idx").unwrap();
//         let mut idx = BTreeIndex::<64>::new(&mut idx_file).unwrap();

//         for i in 1..8u64 {
//             idx.val_map.insert(UUID::from_u128(i * 2), i);
//         }

//         // eprintln!("{:#?}", idx.eq_search(PageAddr::new(0), 512));

//         idx.insert(PageAddr::new(0), UUID::from_u128(2), 1)
//             .expect("oof");
//         assert_eq!(
//             idx.read_node(PageAddr(0)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(0),
//                 entries: vec![UUID::from_u128(2)],
//                 next_leaf_addr: None
//             })
//         );
//     }

//     #[test]
//     fn insert_from_single_leaf_test() {
//         let mut idx_file = BTreeIndex::<64>::truncate_idx_file("./target/test2.idx").unwrap();
//         let mut idx = BTreeIndex::<64>::new(&mut idx_file).unwrap();

//         for i in 1..4 {
//             idx.val_map.insert(UUID::from_u128(i * 2), i);
//             idx.insert(PageAddr::new(0), UUID::from_u128(i * 2), i)
//                 .expect("oof");
//         }

//         assert_eq!(
//             idx.read_node(PageAddr(0)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(0),
//                 entries: vec![UUID::from_u128(2), UUID::from_u128(4), UUID::from_u128(6),],
//                 next_leaf_addr: None
//             })
//         );
//     }
//     #[test]
//     fn single_leaf_split_test() {
//         let mut idx_file = BTreeIndex::<64>::truncate_idx_file("./target/test3.idx").unwrap();
//         let mut idx = BTreeIndex::<64>::new(&mut idx_file).unwrap();

//         for i in 1..5 {
//             idx.val_map.insert(UUID::from_u128(i * 2), i);
//             idx.insert(PageAddr::new(0), UUID::from_u128(i * 2), i)
//                 .expect("oof");
//         }

//         assert_eq!(
//             idx.read_node(PageAddr(0)).unwrap(),
//             Node::<64>::Branch(Branch {
//                 page_addr: PageAddr(0),
//                 entries: vec![(UUID::from_u128(6), PageAddr(128))],
//                 last_page: PageAddr(64),
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(64)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(64),
//                 entries: vec![UUID::from_u128(6), UUID::from_u128(8),],
//                 next_leaf_addr: None
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(128)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(128),
//                 entries: vec![UUID::from_u128(2), UUID::from_u128(4)],
//                 next_leaf_addr: Some(PageAddr(64))
//             })
//         );
//     }

//     #[test]
//     fn single_branch_inserts_test_2() {
//         let mut idx_file = BTreeIndex::<64>::truncate_idx_file("./target/test4.idx").unwrap();
//         let mut idx = BTreeIndex::<64>::new(&mut idx_file).unwrap();

//         for i in 1..6 {
//             idx.val_map.insert(UUID::from_u128(i * 2), i);
//             idx.insert(PageAddr::new(0), UUID::from_u128(i * 2), i)
//                 .expect("oof");
//         }
//         idx.val_map.insert(UUID::from_u128(3), 2);
//         idx.insert(PageAddr::new(0), UUID::from_u128(3), 2)
//             .expect("oof");

//         assert_eq!(
//             idx.read_node(PageAddr(0)).unwrap(),
//             Node::<64>::Branch(Branch {
//                 page_addr: PageAddr(0),
//                 entries: vec![(UUID::from_u128(6), PageAddr(128))],
//                 last_page: PageAddr(64),
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(64)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(64),
//                 entries: vec![UUID::from_u128(6), UUID::from_u128(8), UUID::from_u128(10)],
//                 next_leaf_addr: None
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(128)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(128),
//                 entries: vec![UUID::from_u128(2), UUID::from_u128(4), UUID::from_u128(3)],
//                 next_leaf_addr: Some(PageAddr(64))
//             })
//         );
//     }
//     #[test]
//     fn single_branch_inserts_test_1() {
//         let mut idx_file = BTreeIndex::<64>::truncate_idx_file("./target/test5.idx").unwrap();
//         let mut idx = BTreeIndex::<64>::new(&mut idx_file).unwrap();

//         for i in 1..6 {
//             idx.val_map.insert(UUID::from_u128(i * 2), i);
//             idx.insert(PageAddr::new(0), UUID::from_u128(i * 2), i)
//                 .expect("oof");
//         }
//         idx.val_map.insert(UUID::from_u128(3), 2);
//         idx.insert(PageAddr::new(0), UUID::from_u128(3), 2)
//             .expect("oof");
//         idx.val_map.insert(UUID::from_u128(77), 0);
//         idx.insert(PageAddr::new(0), UUID::from_u128(77), 0)
//             .expect("oof");

//         assert_eq!(
//             idx.read_node(PageAddr(0)).unwrap(),
//             Node::<64>::Branch(Branch {
//                 page_addr: PageAddr(0),
//                 entries: vec![(UUID::from_u128(6), PageAddr(128))],
//                 last_page: PageAddr(64),
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(64)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(64),
//                 entries: vec![UUID::from_u128(6), UUID::from_u128(8), UUID::from_u128(10)],
//                 next_leaf_addr: None
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(128)).unwrap(),
//             Node::<64>::Branch(Branch {
//                 page_addr: PageAddr(128),
//                 entries: vec![(UUID::from_u128(3), PageAddr(256))],
//                 last_page: PageAddr(192)
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(192)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(192),
//                 entries: vec![UUID::from_u128(3)],
//                 next_leaf_addr: Some(PageAddr(64))
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(256)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(256),
//                 entries: vec![UUID::from_u128(77), UUID::from_u128(2), UUID::from_u128(4),],
//                 next_leaf_addr: Some(PageAddr(192))
//             })
//         );
//     }
//     #[test]
//     fn double_branch_inserts_test() {
//         let mut idx_file = BTreeIndex::<64>::truncate_idx_file("./target/test6.idx").unwrap();
//         let mut idx = BTreeIndex::<64>::new(&mut idx_file).unwrap();

//         for i in 1..6 {
//             idx.val_map.insert(UUID::from_u128(i * 2), i);
//             idx.insert(PageAddr::new(0), UUID::from_u128(i * 2), i)
//                 .expect("oof");
//         }
//         idx.val_map.insert(UUID::from_u128(3), 2);
//         idx.insert(PageAddr::new(0), UUID::from_u128(3), 2)
//             .expect("oof");
//         idx.val_map.insert(UUID::from_u128(77), 0);
//         idx.insert(PageAddr::new(0), UUID::from_u128(77), 0)
//             .expect("oof");
//         idx.val_map.insert(UUID::from_u128(88), 0);
//         idx.insert(PageAddr::new(0), UUID::from_u128(88), 0)
//             .expect("oof");
//         idx.val_map.insert(UUID::from_u128(5), 2);
//         idx.insert(PageAddr::new(0), UUID::from_u128(5), 2)
//             .expect("oof");

//         assert_eq!(
//             idx.read_node(PageAddr(0)).unwrap(),
//             Node::<64>::Branch(Branch {
//                 page_addr: PageAddr(0),
//                 entries: vec![(UUID::from_u128(6), PageAddr(128))],
//                 last_page: PageAddr(64),
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(64)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(64),
//                 entries: vec![UUID::from_u128(6), UUID::from_u128(8), UUID::from_u128(10)],
//                 next_leaf_addr: None
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(128)).unwrap(),
//             Node::<64>::Branch(Branch {
//                 page_addr: PageAddr(128),
//                 entries: vec![(UUID::from_u128(3), PageAddr(256))],
//                 last_page: PageAddr(192)
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(192)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(192),
//                 entries: vec![UUID::from_u128(3), UUID::from_u128(5)],
//                 next_leaf_addr: Some(PageAddr(64))
//             })
//         );

//         assert_eq!(
//             idx.read_node(PageAddr(256)).unwrap(),
//             Node::<64>::Branch(Branch {
//                 page_addr: PageAddr(256),
//                 entries: vec![(UUID::from_u128(4), PageAddr(384))],
//                 last_page: PageAddr(320)
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(320)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(320),
//                 entries: vec![UUID::from_u128(4),],
//                 next_leaf_addr: Some(PageAddr(192))
//             })
//         );
//         assert_eq!(
//             idx.read_node(PageAddr(384)).unwrap(),
//             Node::<64>::Leaf(Leaf {
//                 page_addr: PageAddr(384),
//                 entries: vec![UUID::from_u128(77), UUID::from_u128(88), UUID::from_u128(2)],
//                 next_leaf_addr: Some(PageAddr(320))
//             })
//         );

//         assert_eq!(
//             idx.eq_search(PageAddr(0), 4).unwrap(),
//             Some(UUID::from_u128(8))
//         );
//     }
// }

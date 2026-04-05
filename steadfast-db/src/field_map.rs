use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

// #[derive(Debug)]
// pub struct FieldMap<'a, const PAGE_SIZE: usize> {
//     file: &'a mut File,
//     val_map: HashMap<UUID, u64>,
// }

use steadfast_bytes::{ByteSize, ToBytes};
use steadfast_uuid::UUID;

#[repr(C)]
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddressEntry {
    pub uuid: UUID,
    pub address: usize,
    pub last_update: usize,
}

impl ByteSize for AddressEntry {
    const BYTE_SIZE: usize = 32;
}

impl ToBytes<[u8; 32]> for AddressEntry {
    fn to_bytes_le(&self) -> [u8; 32] {
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
    fn to_bytes_be(&self) -> [u8; 32] {
        self.uuid
            .as_u128()
            .to_be_bytes()
            .into_iter()
            .chain(
                self.address
                    .to_be_bytes()
                    .into_iter()
                    .chain(self.last_update.to_be_bytes().into_iter()),
            )
            .enumerate()
            .fold([0u8; 32], |mut buf, (i, b)| {
                buf[i] = b;
                buf
            })
    }
    fn to_bytes_ne(&self) -> [u8; 32] {
        self.uuid
            .as_u128()
            .to_ne_bytes()
            .into_iter()
            .chain(
                self.address
                    .to_ne_bytes()
                    .into_iter()
                    .chain(self.last_update.to_ne_bytes().into_iter()),
            )
            .enumerate()
            .fold([0u8; 32], |mut buf, (i, b)| {
                buf[i] = b;
                buf
            })
    }
}

impl AddressEntry {
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

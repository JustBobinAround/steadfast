use core::cmp::Ordering;
use std::collections::BTreeMap;
use std::marker::PhantomData;

use super::Database;
use steadfast_crypt::SHA256;
use steadfast_macros::{sha256_from_tokens, InternalTableSF};
use steadfast_serializer::{DataHolder, Deserialize, PrimType, Serialize};
use steadfast_time::UTC;
use steadfast_uuid::UUID;
pub trait STable: Serialize + Deserialize + PartialOrd + PartialEq {
    fn table_name() -> &'static str;
    fn table_display_name() -> &'static str;
    fn map_indexed_field_hash(field_name: &str) -> Option<SHA256>;
    fn indexed_fields() -> &'static [(&'static str, SHA256)];
    fn cmp_field(&self, other: &Self, field_name: &str) -> Option<Ordering>;
    const TABLE_ID: SHA256;
    const TYPE_HASH: SHA256;
}

#[derive(PartialEq, Eq, PartialOrd, Ord, InternalTableSF)]
pub struct TestStruct {
    #[indexed]
    test_field: u32,
}

impl Deserialize for TestStruct {
    fn deserialize(dh: DataHolder) -> Result<Self, ()> {
        todo!()
    }
}

impl Serialize for TestStruct {
    fn serialize(self) -> DataHolder {
        todo!()
    }
}

struct TableRef<T: STable> {
    sys_uuid: UUID,
    _table_ty: PhantomData<T>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct TableRecord<T: STable> {
    sys_created_on: UTC,
    sys_updated_on: UTC,
    sys_uuid: UUID,
    inner_record: T,
}

impl<T: STable> TableRecord<T> {
    pub fn sys_created_on(&self) -> &UTC {
        &self.sys_created_on
    }
    pub fn sys_updated_on(&self) -> &UTC {
        &self.sys_updated_on
    }
    pub fn sys_uuid(&self) -> &UUID {
        &self.sys_uuid
    }
}

impl<T: STable> STable for TableRecord<T> {
    fn table_name() -> &'static str {
        T::table_name()
    }

    fn table_display_name() -> &'static str {
        T::table_display_name()
    }

    fn map_indexed_field_hash(field_name: &str) -> Option<SHA256> {
        T::map_indexed_field_hash(field_name)
    }

    fn indexed_fields() -> &'static [(&'static str, SHA256)] {
        T::indexed_fields()
    }

    fn cmp_field(&self, other: &Self, field_name: &str) -> Option<Ordering> {
        match field_name {
            "sys_created_on" => Some(self.sys_created_on.cmp(other.sys_created_on())),
            "sys_updated_on" => Some(self.sys_updated_on.cmp(other.sys_updated_on())),
            "sys_uuid" => Some(self.sys_uuid.cmp(other.sys_uuid())),
            _ => self.inner_record.cmp_field(&other.inner_record, field_name),
        }
    }

    const TABLE_ID: SHA256 = T::TABLE_ID;
    const TYPE_HASH: SHA256 = T::TYPE_HASH;
}

impl<T: STable> Deserialize for TableRecord<T> {
    fn deserialize(dh: steadfast_serializer::DataHolder) -> Result<Self, ()> {
        match dh {
            DataHolder::Struct(mut fields) => {
                let (sys_uuid, sys_created_on) = match fields.remove("sys_uuid") {
                    Some(DataHolder::Primitive(PrimType::UUID(sys_uuid))) => {
                        let sys_created_on =
                            UTC::from_unix_epoch_millis(sys_uuid.extract_timestamp());
                        (sys_uuid, sys_created_on)
                    }
                    _ => return Err(()),
                };
                let sys_updated_on = match fields.remove("sys_updated_on") {
                    Some(DataHolder::Primitive(PrimType::UTC(sys_update_on))) => sys_update_on,
                    _ => return Err(()),
                };
                let inner_record = match fields.remove("inner") {
                    Some(inner_record) => T::deserialize(inner_record)?,
                    _ => return Err(()),
                };
                Ok(Self {
                    sys_created_on,
                    sys_updated_on,
                    sys_uuid,
                    inner_record,
                })
            }
            _ => Err(()),
        }
    }
}

impl<T: STable> Serialize for TableRecord<T> {
    fn serialize(self) -> steadfast_serializer::DataHolder {
        let mut map = BTreeMap::new();
        map.insert(
            String::from("sys_uuid"),
            DataHolder::Primitive(PrimType::UUID(self.sys_uuid)),
        );
        map.insert(
            String::from("sys_updated_on"),
            DataHolder::Primitive(PrimType::UTC(self.sys_updated_on)),
        );
        map.insert(String::from("inner_record"), self.inner_record.serialize());

        DataHolder::Struct(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmp_field() {
        let a = TestStruct { test_field: 0 };

        let b = TestStruct { test_field: 3 };
        assert_eq!(a.cmp_field(&b, "test_field"), Some(Ordering::Less))
    }
}

use std::collections::BTreeMap;
use std::marker::PhantomData;

use steadfast_crypt::SHA256;
use steadfast_macros::InternalTableSF;
use steadfast_serializer::{DataHolder, Deserialize, PrimType, Serialize};
use steadfast_uuid::UUID;
pub trait STable: Serialize + Deserialize {
    fn table_name() -> &'static str;
    fn table_display_name() -> &'static str;
    fn index_field_maps(field_name: &str) -> Option<SHA256>;
    const TABLE_ID: SHA256;
    const TYPE_HASH: SHA256;
}

struct TableRef<T: STable> {
    sys_uuid: UUID,
    _table_ty: T,
}

#[derive(PartialEq)]
struct TableRecord<T: STable> {
    sys_created_on: u32,
    sys_updated_on: u32,
    sys_uuid: UUID,
    inner_record: T,
}

impl<T: STable> STable for TableRecord<T> {
    fn table_name() -> &'static str {
        T::table_name()
    }

    fn table_display_name() -> &'static str {
        T::table_display_name()
    }

    fn index_field_maps(field_name: &str) -> Option<SHA256> {
        match field_name {
            "sys_created_on" => todo!(),
            "sys_updated_on" => todo!(),
            _ => T::index_field_maps(field_name),
        }
    }

    const TABLE_ID: SHA256 = T::TABLE_ID;
    const TYPE_HASH: SHA256 = T::TYPE_HASH;
}

impl<T: STable> Deserialize for TableRecord<T> {
    fn deserialize(dh: steadfast_serializer::DataHolder) -> Result<Self, ()> {
        match dh {
            DataHolder::Struct(mut fields) => {
                let sys_created_on = match fields.remove("sys_created_on") {
                    Some(DataHolder::Primitive(PrimType::U32(sys_created_on))) => sys_created_on,
                    _ => return Err(()),
                };
                let sys_updated_on = match fields.remove("sys_updated_on") {
                    Some(DataHolder::Primitive(PrimType::U32(sys_update_on))) => sys_update_on,
                    _ => return Err(()),
                };
                let sys_uuid = match fields.remove("sys_uuid") {
                    Some(DataHolder::Primitive(PrimType::U128(sys_uuid))) => {
                        UUID::from_u128(sys_uuid)
                    }
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
            String::from("sys_created_on"),
            self.sys_created_on.serialize(),
        );
        map.insert(
            String::from("sys_updated_on"),
            self.sys_updated_on.serialize(),
        );
        map.insert(String::from("inner_record"), self.inner_record.serialize());

        DataHolder::Struct(map)
    }
}

// impl<T: ZeroTable> ZeroTable for TableReference<T> {
//     fn table_name() -> &'static str {
//         T::table_name()
//     }

//     fn table_version_hash() -> UUID {
//         T::table_version_hash()
//     }
// }

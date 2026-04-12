use core::cmp::Ordering;
use std::marker::PhantomData;
use steadfast_bytes::{DynBytes, ReadByteStream as RBS, TypeCode, WriteByteStream as WBS};
use steadfast_crypt::SHA256;
use steadfast_time::UTC;
use steadfast_uuid::UUID;

pub trait STable: RBS<DynBytes> + WBS<DynBytes> + PartialOrd + PartialEq {
    fn table_name() -> &'static str;
    fn table_display_name() -> &'static str;
    fn map_indexed_field_hash(field_name: &str) -> Option<SHA256>;
    fn indexed_fields() -> &'static [(&'static str, SHA256)];
    fn cmp_field(&self, other: &Self, field_name: &str) -> Option<Ordering>;
    const TABLE_ID: SHA256;
    const TYPE_HASH: SHA256;
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

    pub fn new(inner_record: T) -> Result<TableRecord<T>, ()> {
        let sys_uuid = UUID::rand_v7().map_err(|_| ())?;
        todo!()
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

macro_rules! impl_rbsd_tr {
    ($fn_name: ident) => {
        fn $fn_name<R: std::io::Read>(
            stream: &mut R,
            checksum: &mut usize,
        ) -> Result<Self, steadfast_bytes::BytesErr> {
            let mut inner_checksum = 0;
            let sys_uuid = <UUID>::$fn_name(stream, &mut inner_checksum)?;
            let sys_created_on: UTC = sys_uuid.into();
            let sys_updated_on = <UTC>::$fn_name(stream, &mut inner_checksum)?;
            let inner_record = <T>::$fn_name(stream, &mut inner_checksum)?;
            let record_byte_len = <usize>::$fn_name(stream, checksum)?;
            *checksum += inner_checksum;
            if record_byte_len == inner_checksum {
                Ok(Self {
                    sys_created_on,
                    sys_updated_on,
                    sys_uuid,
                    inner_record,
                })
            } else {
                Err(steadfast_bytes::BytesErr::ChecksumFailed {
                    expected: inner_checksum,
                    found: record_byte_len,
                })
            }
        }
    };
}

impl<T: STable> RBS<DynBytes> for TableRecord<T> {
    impl_rbsd_tr!(read_byte_stream_le);
    impl_rbsd_tr!(read_byte_stream_be);
    impl_rbsd_tr!(read_byte_stream_ne);
}

macro_rules! impl_wbsd_tr {
    ($fn_name:ident) => {
        fn $fn_name<W: std::io::Write>(
            &self,
            stream: &mut W,
        ) -> Result<usize, steadfast_bytes::BytesErr> {
            let record_len = TypeCode::DynSize.as_u8().$fn_name(stream)?
                + self.sys_uuid.$fn_name(stream)?
                + self.sys_updated_on.$fn_name(stream)?
                + self.inner_record.$fn_name(stream)?;
            let final_len = record_len.$fn_name(stream)? + record_len;
            Ok(final_len)
        }
    };
}

impl<T: STable> WBS<DynBytes> for TableRecord<T> {
    impl_wbsd_tr!(write_byte_stream_le);
    impl_wbsd_tr!(write_byte_stream_be);
    impl_wbsd_tr!(write_byte_stream_ne);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmp_field() {
        use steadfast_macros::{InternalTableSF, ReadByteStreamInternal, WriteByteStreamInternal};
        #[derive(
            Debug,
            InternalTableSF,
            ReadByteStreamInternal,
            WriteByteStreamInternal,
            PartialOrd,
            PartialEq,
        )]
        pub struct TestStruct {
            test_field: u32,
        }
        let a = TestStruct { test_field: 3 };
        let b = TestStruct { test_field: 5 };

        let mut c = std::io::Cursor::new(Vec::new());
        let checksum_a = a.write_byte_stream_le(&mut c).unwrap();
        c.set_position(0);
        let mut checksum_b = 0;
        assert_eq!(
            a,
            <TestStruct>::read_byte_stream_le(&mut c, &mut checksum_b).unwrap()
        );
        assert_eq!(checksum_a, checksum_b);
        assert_eq!(a.cmp_field(&b, "test_field"), Some(Ordering::Less))
    }
}

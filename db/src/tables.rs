use serializer::{Deserialize, Serialize};
pub trait ZeroTable: Serialize + Deserialize {
    fn table_name() -> &'static str;
    fn table_display_name() -> &'static str;
    fn table_id() -> usize;
}

// impl<T: ZeroTable> ZeroTable for TableReference<T> {
//     fn table_name() -> &'static str {
//         T::table_name()
//     }

//     fn table_version_hash() -> UUID {
//         T::table_version_hash()
//     }
// }

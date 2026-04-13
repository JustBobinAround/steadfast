use std::{any::Any, cmp::Ordering};

// use std::cmp::Ordering;

pub trait OptionalOrd {
    fn cmp_or_none(&self, other: &dyn Any) -> Option<Ordering>;
}

impl<T: Any + PartialOrd> OptionalOrd for T {
    fn cmp_or_none(&self, other: &dyn Any) -> Option<Ordering> {
        other.downcast_ref::<T>().and_then(|o| self.partial_cmp(o))
    }
}

pub trait FieldOrd {
    fn cmp_with_field(&self, field_name: &str, val: &dyn Any) -> Option<Ordering>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_optional_ord() {
        assert_eq!("alice".cmp_or_none(&0), None);
        assert_eq!("alice".cmp_or_none(&"bob"), Some(Ordering::Less));
    }

    #[test]
    fn test_field_ord() {
        struct TestStruct {
            a: i32,
            b: String,
        }

        impl FieldOrd for TestStruct {
            fn cmp_with_field(&self, field_name: &str, val: &dyn Any) -> Option<Ordering> {
                match field_name {
                    "a" => self.a.cmp_or_none(val),
                    "b" => self.b.cmp_or_none(val),
                    _ => None,
                }
            }
        }

        let test_vals = TestStruct {
            a: 42,
            b: String::from("alice"),
        };

        assert_eq!(test_vals.cmp_with_field("a", &String::from("alice")), None);
        assert_eq!(test_vals.cmp_with_field("b", &24), None);
        assert_eq!(test_vals.cmp_with_field("a", &24), Some(Ordering::Greater));
        assert_eq!(
            test_vals.cmp_with_field("b", &String::from("bob")),
            Some(Ordering::Less)
        );
    }
}

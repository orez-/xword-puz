// Based on https://github.com/andrewlowndes/serde_literals ,
// but for structs instead of enums.

use core::fmt;
use serde::de::{self, Unexpected, Visitor};

pub struct LitStr<'a>(pub &'a str);

impl<'a, 'de> Visitor<'de> for LitStr<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "the lit {}", self.0)
    }

    fn visit_str<E>(self, s: &str) -> Result<(), E>
    where
        E: de::Error,
    {
        if s == self.0 {
            Ok(())
        } else {
            Err(de::Error::invalid_value(Unexpected::Str(s), &self))
        }
    }
}

#[macro_export]
macro_rules! lit_str {
    ($struct_name:ident, $val:expr) => {
        #[derive(Default, Debug)]
        pub struct $struct_name;

        impl<'de> serde::Deserialize<'de> for $struct_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_str($crate::serde_lit::LitStr($val))?;
                Ok(Self)
            }
        }

        impl serde::Serialize for $struct_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str($val)
            }
        }
    };
}

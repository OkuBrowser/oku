use serde::de::{self, Deserialize, Deserializer, Visitor, value::Error};
use serde::forward_to_deserialize_any;

/// Return the fields of a struct.
/// Taken directly from <https://github.com/serde-rs/serde/issues/1110>
///
pub fn struct_name_and_fields<'de, T>()
-> Result<(&'static str, Option<&'static [&'static str]>), Error>
where
    T: Deserialize<'de>,
{
    struct StructNameAndFieldsDeserializer<'a> {
        name: &'a mut Option<&'static str>,
        fields: &'a mut Option<&'static [&'static str]>,
    }

    impl<'de, 'a> Deserializer<'de> for StructNameAndFieldsDeserializer<'a> {
        type Error = Error;

        fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            Err(de::Error::custom("I'm just here for the fields"))
        }

        fn deserialize_newtype_struct<V>(
            self,
            name: &'static str,
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            *self.name = Some(name);
            visitor.visit_newtype_struct(self)
        }

        fn deserialize_struct<V>(
            self,
            name: &'static str,
            fields: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            *self.name = Some(name);
            *self.fields = Some(fields);
            self.deserialize_any(visitor)
        }

        forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
            byte_buf option unit seq tuple
            tuple_struct map enum identifier ignored_any
        }

        fn deserialize_unit_struct<V>(
            self,
            name: &'static str,
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            self.deserialize_struct(name, &[], visitor)
        }
    }

    let mut name = None;
    let mut fields = None;

    let _ = T::deserialize(StructNameAndFieldsDeserializer {
        name: &mut name,
        fields: &mut fields,
    });

    match name {
        None => Err(de::Error::custom(
            "Expected a named struct. \
            Hint: You cannot use a HashMap<...> in this context because it requires the struct to have a name",
        )),
        Some(name) => {
            validate_identifier(name)?;
            for field in fields.into_iter().flatten() {
                validate_identifier(field)?;
            }

            Ok((name, fields))
        }
    }
}

/// Validate a namespace/class/property name.
///
/// From [DMTF-DSP0004], Appendix F: Unicode Usage:
///
/// > ...<br>
/// > Therefore, all namespace, class and property names are identifiers composed as follows:<br>
/// > Initial identifier characters must be in set S1, where S1 = {U+005F, U+0041...U+005A, U+0061...U+007A, U+0080...U+FFEF) \[This is alphabetic, plus underscore\]<br>
/// > All following characters must be in set S2 where S2 = S1 union {U+0030...U+0039} \[This is alphabetic, underscore, plus Arabic numerals 0 through 9.\]<br>
///
/// [DMTF-DSP0004]:     https://www.dmtf.org/sites/default/files/standards/documents/DSP0004V2.3_final.pdf
fn validate_identifier<E: de::Error>(s: &str) -> Result<&str, E> {
    fn is_s1(ch: char) -> bool {
        match ch {
            '\u{005f}' => true,
            '\u{0041}'..='\u{005A}' => true,
            '\u{0061}'..='\u{007A}' => true,
            '\u{0080}'..='\u{FFEF}' => true,
            _other => false,
        }
    }

    fn is_s2(ch: char) -> bool {
        match ch {
            '\u{0030}'..='\u{0039}' => true,
            _other => is_s1(ch),
        }
    }

    let mut chars = s.chars();
    match chars.next() {
        None => Err(de::Error::custom(
            "An empty string is not a valid namespace, class, or property name",
        )),
        Some(ch) if !is_s1(ch) => Err(de::Error::custom(
            "An identifier must start with '_' or an alphabetic character",
        )),
        Some(_) if !chars.all(is_s2) => Err(de::Error::custom(
            "An identifier must only consist of '_', alphabetic, or numeric characters",
        )),
        Some(_) => Ok(s),
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::Variant;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[test]
    fn it_works() {
        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem {
            Caption: String,
            Name: String,
        }

        let (name, fields) = struct_name_and_fields::<Win32_OperatingSystem>().unwrap();

        assert_eq!(name, "Win32_OperatingSystem");
        assert_eq!(fields.unwrap(), ["Caption", "Name"]);
    }

    #[test]
    fn it_works_with_rename() {
        #[derive(Deserialize, Debug)]
        #[serde(rename = "Win32_OperatingSystem")]
        #[serde(rename_all = "PascalCase")]
        struct Win32OperatingSystem {
            caption: String,
            name: String,
        }

        let (name, fields) = struct_name_and_fields::<Win32OperatingSystem>().unwrap();

        assert_eq!(name, "Win32_OperatingSystem");
        assert_eq!(fields.unwrap(), ["Caption", "Name"]);
    }

    #[test]
    fn it_works_with_flatten() {
        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem_inner {
            Caption: String,
            Name: String,

            #[serde(flatten)]
            extra: HashMap<String, Variant>,
        }

        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem(pub Win32_OperatingSystem_inner);

        let (name, fields) = struct_name_and_fields::<Win32_OperatingSystem>().unwrap();

        assert_eq!(name, "Win32_OperatingSystem");
        assert_eq!(fields, None);
    }

    #[test]
    fn it_fails_for_sqli() {
        #[derive(Deserialize, Debug)]
        #[serde(rename = "Evil\\Struct\\Name")]
        struct EvilStructName {}

        #[derive(Deserialize, Debug)]
        struct EvilFieldName {
            #[serde(rename = "Evil\"Field\"Name")]
            field: String,
        }

        struct_name_and_fields::<EvilStructName>().unwrap_err();
        struct_name_and_fields::<EvilFieldName>().unwrap_err();
    }

    #[test]
    fn it_fails_for_non_structs() {
        let err = struct_name_and_fields::<HashMap<String, Variant>>().unwrap_err();

        assert!(format!("{:?}", err).contains("Expected a named struct"));
    }
}

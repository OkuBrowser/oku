use core::marker::PhantomData;
use crate::core::{ enc, dec };

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct RawValue<'de>(&'de [u8]);

struct RawValueReader<'r, 'de, R>
where R: dec::Read<'de>
{
    reader: &'r mut R,
    readn: usize,
    _phantom: PhantomData<&'de [u8]>
}

impl<'r, 'de, R> RawValueReader<'r, 'de, R>
where R: dec::Read<'de>
{
    #[inline]
    fn new(reader: &'r mut R) -> RawValueReader<'r, 'de, R> {
        RawValueReader {
            reader,
            readn: 0,
            _phantom: PhantomData
        }
    }
}

impl<'de, R> dec::Read<'de> for RawValueReader<'_, 'de, R>
where R: dec::Read<'de>
{
    type Error = R::Error;

    #[inline]
    fn fill<'short>(&'short mut self, want: usize) -> Result<dec::Reference<'de, 'short>, Self::Error> {
        let want = match self.readn.checked_add(want) {
            Some(n) => n,
            None => return Ok(dec::Reference::Long(&[]))
        };

        let buf = match self.reader.fill(want)? {
            dec::Reference::Long(buf)
                if buf.len() >= self.readn => dec::Reference::Long(&buf[self.readn..]),
            dec::Reference::Long(_) => dec::Reference::Long(&[]),
            dec::Reference::Short(buf) => dec::Reference::Short(buf)
        };

        Ok(buf)
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        self.readn += n;
    }

    #[inline]
    fn step_in(&mut self) -> bool {
        self.reader.step_in()
    }

    #[inline]
    fn step_out(&mut self) {
        self.reader.step_out()
    }
}

impl<'de> dec::Decode<'de> for RawValue<'de> {
    #[inline]
    fn decode<R: dec::Read<'de>>(reader: &mut R) -> Result<Self, dec::Error<R::Error>> {
        let name = &"raw-value";

        let mut reader = RawValueReader::new(reader);
        let _ignore = dec::IgnoredAny::decode(&mut reader)?;

        let buf = match reader.reader.fill(reader.readn).map_err(dec::Error::Read)? {
            dec::Reference::Long(buf)
                if buf.len() >= reader.readn => &buf[..reader.readn],
            dec::Reference::Long(buf) => return Err(dec::Error::require_length(name, Some(buf.len()))),
            dec::Reference::Short(_) => return Err(dec::Error::require_borrowed(name))
        };

        reader.reader.advance(reader.readn);

        Ok(RawValue(buf))
    }
}

impl enc::Encode for RawValue<'_> {
    #[inline]
    fn encode<W: enc::Write>(&self, writer: &mut W) -> Result<(), enc::Error<W::Error>> {
        writer.push(self.0).map_err(enc::Error::Write)
    }
}

impl<'de> RawValue<'de> {
    pub fn as_bytes(&self) -> &'de [u8] {
        self.0
    }
}

#[cfg(feature = "use_alloc")]
pub mod boxed {
    use crate::core::Value;
    use crate::core::utils::BufWriter;
    use crate::alloc::boxed::Box;
    use super::*;
    
    #[derive(PartialEq, Eq, Debug, Clone)]
    pub struct BoxedRawValue(Box<[u8]>);

    impl<'de> dec::Decode<'de> for BoxedRawValue {
        #[inline]
        fn decode<R: dec::Read<'de>>(reader: &mut R) -> Result<Self, dec::Error<R::Error>> {
            let value = RawValue::decode(reader)?;
            Ok(BoxedRawValue(Box::from(value.0)))
        }
    }

    impl enc::Encode for BoxedRawValue {
        #[inline]
        fn encode<W: enc::Write>(&self, writer: &mut W) -> Result<(), enc::Error<W::Error>> {
            writer.push(&self.0).map_err(enc::Error::Write)
        }
    }

    impl BoxedRawValue {
        pub fn as_bytes(&self) -> &[u8] {
            &self.0
        }

        pub fn from_value(value: &Value)
            -> Result<BoxedRawValue, enc::Error<crate::alloc::collections::TryReserveError>>
        {
            use crate::alloc::vec::Vec;
            use crate::core::enc::Encode;
            
            let mut writer = BufWriter::new(Vec::new());
            value.encode(&mut writer)?;
            Ok(BoxedRawValue(writer.into_inner().into_boxed_slice()))
        }
    }
}

#[test]
#[cfg(feature = "use_std")]
fn test_raw_value() {
    use crate::core::enc::Encode;
    use crate::core::dec::Decode;
    use crate::core::utils::{ BufWriter, SliceReader };
    use crate::core::types;
    use boxed::BoxedRawValue;

    let buf = {
        let mut buf = BufWriter::new(Vec::new());

        types::Map(&[
            ("bar", types::Map(&[
                ("value", 0x99u32)
            ][..]))
        ][..]).encode(&mut buf).unwrap();

        buf
    };

    // raw value
    {
        let mut reader = SliceReader::new(buf.buffer());
        let map = <types::Map<Vec<(&str, RawValue<'_>)>>>::decode(&mut reader).unwrap();

        assert_eq!(map.0.len(), 1);
        assert_eq!(map.0[0].0, "bar");

        let bar_raw_value = &map.0[0].1;
        assert!(!bar_raw_value.as_bytes().is_empty());

        let buf2 = {
            let mut buf = BufWriter::new(Vec::new());

            types::Map(&[
                ("bar", bar_raw_value)
            ][..]).encode(&mut buf).unwrap();

            buf
        };

        assert_eq!(buf.buffer(), buf2.buffer());

        type Bar<'a> = types::Map<Vec<(&'a str, u32)>>;

        let mut reader = SliceReader::new(buf2.buffer());
        let map2 = <types::Map<Vec<(&str, Bar)>>>::decode(&mut reader).unwrap();

        assert_eq!(map2.0.len(), 1);
        assert_eq!(map2.0[0].0, "bar");

        let bar = &map2.0[0].1;

        assert_eq!(bar.0.len(), 1);
        assert_eq!(bar.0[0].0, "value");
        assert_eq!(bar.0[0].1, 0x99);
    }

    // boxed raw value
    {
        let mut reader = SliceReader::new(buf.buffer());
        let map = <types::Map<Vec<(&str, BoxedRawValue)>>>::decode(&mut reader).unwrap();

        assert_eq!(map.0.len(), 1);
        assert_eq!(map.0[0].0, "bar");

        let bar_raw_value = &map.0[0].1;
        assert!(!bar_raw_value.as_bytes().is_empty());

        // check from value
        {
            use crate::core::Value;
            
            let mut reader = SliceReader::new(buf.buffer());
            let map = <types::Map<Vec<(&str, Value)>>>::decode(&mut reader).unwrap();
            
            let bar_value = &map.0[0].1;
            let bar_raw_value2 = BoxedRawValue::from_value(&bar_value).unwrap();
            assert_eq!(bar_raw_value, &bar_raw_value2);
        }

        let buf2 = {
            let mut buf = BufWriter::new(Vec::new());

            types::Map(&[
                ("bar", bar_raw_value)
            ][..]).encode(&mut buf).unwrap();

            buf
        };

        assert_eq!(buf.buffer(), buf2.buffer());

        type Bar<'a> = types::Map<Vec<(&'a str, u32)>>;

        let mut reader = SliceReader::new(buf2.buffer());
        let map2 = <types::Map<Vec<(&str, Bar)>>>::decode(&mut reader).unwrap();

        assert_eq!(map2.0.len(), 1);
        assert_eq!(map2.0[0].0, "bar");

        let bar = &map2.0[0].1;

        assert_eq!(bar.0.len(), 1);
        assert_eq!(bar.0[0].0, "value");
        assert_eq!(bar.0[0].1, 0x99);        
    }
}

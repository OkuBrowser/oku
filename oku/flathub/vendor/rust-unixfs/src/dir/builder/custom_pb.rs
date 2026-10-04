//! Custom protobuf types which are used in encoding directorys.

use super::NamedLeaf;
use crate::pb::{UnixFs, WriteableCid};
use quick_protobuf::{MessageWrite, Writer, WriterBackend};

/// Newtype which uses the &[Option<(NamedLeaf)>] as Vec<PBLink>.
pub(super) struct CustomFlatUnixFs<'a> {
    pub(super) links: &'a [Option<NamedLeaf>],
    pub(super) data: UnixFs<'a>,
}

impl CustomFlatUnixFs<'_> {
    fn mapped(&self) -> impl Iterator<Item = NamedLeafAsPBLink<'_>> + '_ {
        self.links
            .iter()
            // FIXME: this unwrap here seems dangerious; it seems to follow from
            // `crate::dir::builder::iter::Leaves` assumption that all of these options have
            // already been filled at the previous stages of post-order visit
            .map(|triple| triple.as_ref().map(NamedLeafAsPBLink).unwrap())
    }
}

impl MessageWrite for CustomFlatUnixFs<'_> {
    fn get_size(&self) -> usize {
        use quick_protobuf::sizeofs::*;

        let links = self
            .mapped()
            .map(|link| 1 + sizeof_len(link.get_size()))
            .sum::<usize>();

        links + 1 + sizeof_len(self.data.get_size())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> quick_protobuf::Result<()> {
        self.mapped()
            .try_for_each(|l| w.write_with_tag(18, |w| w.write_message(&l)))?;
        w.write_with_tag(10, |w| w.write_message(&self.data))
    }
}

/// Custom NamedLeaf as PBLink "adapter."
struct NamedLeafAsPBLink<'a>(&'a NamedLeaf);

impl MessageWrite for NamedLeafAsPBLink<'_> {
    fn get_size(&self) -> usize {
        use quick_protobuf::sizeofs::*;

        // ones are the tags
        1 + sizeof_len((self.0).0.len())
            + 1
            + sizeof_len(WriteableCid(&(self.0).1).get_size())
            //+ sizeof_len(self.1.link.to_bytes().len())
            + 1
            + sizeof_varint((self.0).2)
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> quick_protobuf::Result<()> {
        w.write_with_tag(10, |w| w.write_message(&WriteableCid(&(self.0).1)))?;
        //w.write_with_tag(10, |w| w.write_bytes(&self.1.link.to_bytes()))?;
        w.write_with_tag(18, |w| w.write_string((self.0).0.as_str()))?;
        w.write_with_tag(24, |w| w.write_uint64((self.0).2))?;
        Ok(())
    }
}

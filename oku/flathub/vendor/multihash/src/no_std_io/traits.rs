use super::error::{Error, ErrorKind, Result};

/// A minimal backfill of the [`std::io::Read`] for `no_std` environments.
pub trait Read {
    /// Backfill of the [`std::io::Read::read`].
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Backfill of the [`std::io::Read::read_exact`].
    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() {
            Err(Error::new(
                ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ))
        } else {
            Ok(())
        }
    }
}

/// A minimal backfill of the [`std::io::Write`] for `no_std` environments.
pub trait Write {
    /// Backfill of the [`std::io::Write::write`].
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Backfill of the [`std::io::Write::write_all`].
    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => {
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(n) => buf = &buf[n..],
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

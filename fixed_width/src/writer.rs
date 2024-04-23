use crate::{ser, FixedWidth, LineBreak, Result};
use serde::ser::Serialize;
use std::{
    borrow::Cow,
    io::{self, Write},
};

const BUFFER_SIZE: usize = 65_536;

/// A trait to ease converting byte like data into a byte slice. This allows handling these types
/// with one generic function.
pub trait AsByteSlice {
    /// Borrows self as a slice of bytes.
    fn as_byte_slice(&self) -> &[u8];
}

impl AsByteSlice for String {
    /// Borrow a `String` as `&[u8]`
    fn as_byte_slice(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsByteSlice for str {
    /// Borrow a `str` as `&[u8]`
    fn as_byte_slice(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsByteSlice for [u8] {
    /// Borrow a `[u8]` as `&[u8]`
    fn as_byte_slice(&self) -> &[u8] {
        self
    }
}

impl AsByteSlice for Vec<u8> {
    /// Borrow a `Vec<u8>` as `&[u8]`
    fn as_byte_slice(&self) -> &[u8] {
        self
    }
}

impl<'a, T: ?Sized> AsByteSlice for Cow<'a, T>
where
    T: AsByteSlice + ToOwned,
    <T as ToOwned>::Owned: AsByteSlice,
{
    /// Borrow a `Cow` type as `&[u8]`
    fn as_byte_slice(&self) -> &[u8] {
        match *self {
            Cow::Borrowed(v) => v.as_byte_slice(),
            Cow::Owned(ref v) => v.as_byte_slice(),
        }
    }
}

impl<'a, T: ?Sized + AsByteSlice> AsByteSlice for &'a T {
    fn as_byte_slice(&self) -> &[u8] {
        (*self).as_byte_slice()
    }
}

/// A fixed width data writer. It writes data provided in iterators to any type that implements
/// io::Write.
///
/// ### Example
///
/// Writing a `Vec<String>` to a file:
///
/// ```rust
/// use fixed_width;
/// use std::io::Write;
/// use fixed_width::Writer;
///
/// let data = vec![
///     "1234".to_string(),
///     "5678".to_string(),
/// ];
///
/// let mut wrtr = Writer::from_memory();
/// wrtr.write_iter(data.iter());
/// wrtr.flush();
/// ```
pub struct Writer<W: Write> {
    wrtr: io::BufWriter<W>,
    linebreak: LineBreak,
}

impl<W> Writer<W>
where
    W: Write,
{
    /// Creates a new writer from any type that implements io::Write
    pub fn from_writer(wrtr: W) -> Self {
        Self::from_buffer(io::BufWriter::with_capacity(BUFFER_SIZE, wrtr))
    }

    /// Creates a new writer from a io::BufWriter that wraps a type that implements io::Write
    pub fn from_buffer(buf: io::BufWriter<W>) -> Self {
        Self {
            wrtr: buf,
            linebreak: LineBreak::None,
        }
    }

    /// Writes the given iterator of `FixedWidth + Serialize` types to the underlying writer,
    /// optionally inserting linebreaks if specified.
    pub fn write_serialized<T: FixedWidth + Serialize>(
        &mut self,
        records: impl Iterator<Item = T>,
    ) -> Result<()> {
        let mut first_record = true;

        for record in records {
            if !first_record {
                self.write_linebreak()?;
            } else {
                first_record = false;
            }

            ser::to_writer(self, &record)?;
        }

        Ok(())
    }

    /// Writes the given iterator of types that implement AsByteSlice to the underlying writer,
    /// optionally inserting linebreaks if specified.
    pub fn write_iter<T: AsByteSlice>(&mut self, records: impl Iterator<Item = T>) -> Result<()> {
        let mut first_record = true;

        for record in records {
            if !first_record {
                self.write_linebreak()?;
            } else {
                first_record = false;
            }

            self.write_all(record.as_byte_slice())?;
        }

        Ok(())
    }

    /// Writes the linebreak specified to the underlying writer. Does nothing if there is no
    /// linebreak.
    #[inline]
    pub fn write_linebreak(&mut self) -> Result<()> {
        match self.linebreak {
            LineBreak::Newline => {
                self.write_all(b"\n")?;
            }
            LineBreak::CRLF => {
                self.write_all(b"\r\n")?;
            }
            LineBreak::None => {}
        }

        Ok(())
    }

    /// Sets the linebreak desired for this data. Defaults to `LineBreak::None`.
    pub fn linebreak(mut self, linebreak: LineBreak) -> Self {
        self.linebreak = linebreak;
        self
    }
}

impl<W> Write for Writer<W>
where
    W: Write,
{
    /// Writes a buffer into the underlying writer.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.wrtr.write(buf)
    }

    /// flushes the underlying writer.
    fn flush(&mut self) -> io::Result<()> {
        self.wrtr.flush()?;
        Ok(())
    }
}

impl Writer<Vec<u8>> {
    /// Creates a new writer in memory from a `Vec<u8>`.
    pub fn from_memory() -> Self {
        Self::from_writer(Vec::with_capacity(BUFFER_SIZE))
    }
}

impl From<Writer<Vec<u8>>> for Vec<u8> {
    /// Converts the writer into a `Vec<u8>`, but panics if unable to flush to the underlying
    /// writer.
    fn from(mut writer: Writer<Vec<u8>>) -> Self {
        match writer.wrtr.flush() {
            Err(e) => panic!("could not flush bytes: {}", e),
            Ok(()) => writer.wrtr.into_inner().unwrap(),
        }
    }
}

impl From<Writer<Vec<u8>>> for String {
    /// Converts the writer into a `String`, but panics if unable to flush to the underlying
    fn from(mut writer: Writer<Vec<u8>>) -> Self {
        match writer.wrtr.flush() {
            Err(e) => panic!("could not flush bytes: {}", e),
            Ok(()) => String::from_utf8(writer.into()).unwrap(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{FieldSet, FixedWidth};
    use serde_derive::Serialize;

    #[test]
    fn write_to_memory() {
        let records = [
            "1111222233334444".to_string(),
            "1111222233334444".to_string(),
            "1111222233334444".to_string(),
        ];

        let mut wrtr = Writer::from_memory();

        wrtr.write_iter(records.iter()).unwrap();

        let mut expected = b"1111222233334444".to_vec();
        expected.append(&mut b"1111222233334444".to_vec());
        expected.append(&mut b"1111222233334444".to_vec());

        assert_eq!(expected, Into::<Vec<u8>>::into(wrtr));
    }

    #[test]
    fn write_to_writer() {
        let v = vec![16; 0];
        let records = [
            "1111222233334444".to_string(),
            "1111222233334444".to_string(),
            "1111222233334444".to_string(),
        ];

        let mut wrtr = Writer::from_writer(v);

        wrtr.write_iter(records.iter()).unwrap();

        let mut expected = b"1111222233334444".to_vec();
        expected.append(&mut b"1111222233334444".to_vec());
        expected.append(&mut b"1111222233334444".to_vec());

        assert_eq!(expected, Into::<Vec<u8>>::into(wrtr));
    }

    #[derive(Debug, Serialize)]
    struct Test2 {
        a: usize,
        b: String,
    }

    impl FixedWidth for Test2 {
        fn fields() -> FieldSet {
            FieldSet::Seq(vec![FieldSet::new_field(0..3), FieldSet::new_field(3..6)])
        }
    }

    #[test]
    fn serialized_write() {
        let tests = vec![
            Test2 {
                a: 1234,
                b: "foobar".to_string(),
            },
            Test2 {
                a: 12,
                b: "fb".to_string(),
            },
            Test2 {
                a: 123,
                b: "foo".to_string(),
            },
        ];

        let mut w = Writer::from_memory().linebreak(LineBreak::Newline);
        w.write_serialized(tests.into_iter()).unwrap();
        let s: String = w.into();

        assert_eq!(s, "123foo\n12 fb \n123foo");
    }

    #[test]
    fn test_write() {
        let bytes = b"abcd1234";
        let mut w = Writer::from_memory();
        let written = w.write(bytes).unwrap();
        let s: String = w.into();

        assert!(written > 0);
        assert_eq!(s, "abcd1234");
    }
}

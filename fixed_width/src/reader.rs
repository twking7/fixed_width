use crate::{error::Error, LineBreak, Result};
use std::{
    fs,
    io::{self, Read},
    path::Path,
};

const BUFFER_SIZE: usize = 8 * (1 << 10);

/// An iterator of `Vec<u8>` records.
///
/// The lifetime 'a denotes the lifetime of the reader, R.
pub struct ByteReader<'a, R: 'a> {
    r: &'a mut Reader<R>,
}

/// An iterator of `String` records.
///
/// The lifetime 'a denotes the lifetime of the reader, R.
pub struct StringReader<'a, R: 'a> {
    r: &'a mut Reader<R>,
}

/// A fixed width data reader. It parses fixed width data and provides the data via iterators.
///
/// ### Example
///
/// Parsing fixed width data into a struct;
///
/// ```rust
/// use serde_derive::Deserialize;
/// use serde;
/// use fixed_width::{FieldSet, FixedWidth, Reader};
/// use serde::Deserialize;
/// use std::result;
///
/// #[derive(Deserialize)]
/// struct Foo {
///     name: String,
///     age: usize,
/// }
///
/// // can be derived using the `fixed_width_derive` crate.
/// impl FixedWidth for Foo {
///     fn fields() -> FieldSet {
///         FieldSet::Seq(vec![
///             FieldSet::new_field(0..6),
///             FieldSet::new_field(6..10),
///         ])
///     }
/// }
///
/// let data = "foobar1234foobaz6789";
/// let mut reader = Reader::from_string(data).width(10);
///
/// for row in reader.byte_reader().filter_map(result::Result::ok) {
///     let record: Foo = fixed_width::from_bytes(&row).unwrap();
///
///     println!("{}", record.name);
///     println!("{}", record.age);
/// }
/// ```
///
/// ### Example
///
/// Parsing fixed width data into a `HashMap<String, String>`:
///
/// ```rust
/// use serde;
/// use fixed_width::{FieldSet, FixedWidth, Deserializer, Reader};
/// use std::collections::HashMap;
/// use serde::Deserialize;
///
///  let data = "foobar1234foobaz6789";
///  let mut reader = Reader::from_string(data).width(10);
///  let fields = FieldSet::Seq(vec![
///      FieldSet::new_field(0..6).name("name"),
///      FieldSet::new_field(6..10).name("age"),
///  ]);
///
///  for row in reader.byte_reader() {
///      let bytes = row.unwrap();
///      let mut de = Deserializer::new(&bytes, fields.clone());
///      let record: HashMap<String, String> = HashMap::deserialize(&mut de).unwrap();
///
///      println!("{}", record.get("name").unwrap());
///      println!("{}", record.get("age").unwrap());
///  }
/// ```
///
/// ### Example
///
/// Parsing fixed width data into `Vec<String>`:
///
/// ```rust
/// use fixed_width::Reader;
///
/// let data = "foobar1234foobaz6789";
///
/// let mut reader = Reader::from_string(data).width(10);
///
/// for row in reader.string_reader() {
///     println!("{:?}", row);
/// }
/// ```
///
/// ### Example
///
/// Parsing fixed width data into `Vec<Vec<u8>>`:
///
/// ```rust
/// use fixed_width::Reader;
///
/// let data = "foobar1234foobaz6789";
///
/// let mut reader = Reader::from_string(data).width(10);
///
/// for row in reader.byte_reader() {
///     println!("{:?}", row);
/// }
/// ```
///
/// ### Example
///
/// Read each line without copying:
///
/// ```rust
/// use fixed_width::Reader;
///
/// let data = "foobar1234foobaz6789";
///
/// let mut reader = Reader::from_string(data).width(10);
///
/// if let Some(Ok(row)) = reader.next_record() {
///     assert_eq!(row, b"foobar1234");
/// }
///
/// if let Some(Ok(row)) = reader.next_record() {
///     assert_eq!(row, b"foobaz6789");
/// }
/// ```
pub struct Reader<R> {
    rdr: io::BufReader<R>,
    buf: Vec<u8>,
    linebreak_buf: Vec<u8>,
    eof: bool,
    /// The width in bytes of the record. Required in order to parse.
    pub record_width: usize,
    /// The line break that occurs between each record. Defaults to `LineBreak::None`
    pub linebreak: LineBreak,
}

impl<R> Reader<R>
where
    R: Read,
{
    /// Creates a new reader from any type that implements io::Read.
    pub fn from_reader(rdr: R) -> Self {
        Reader {
            rdr: io::BufReader::with_capacity(BUFFER_SIZE, rdr),
            record_width: 0,
            buf: Vec::new(),
            linebreak: LineBreak::None,
            linebreak_buf: Vec::new(),
            eof: false,
        }
    }

    /// Reads each record of the data as a `String`. If the data is not valid UTF-8, then
    /// you should use `byte_reader` instead.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::Reader;
    ///
    /// let mut reader = Reader::from_string("abcd1234").width(8);
    ///
    /// for record in reader.string_reader() {
    ///     assert_eq!(record.unwrap(), "abcd1234")
    /// }
    /// ```
    pub fn string_reader(&mut self) -> StringReader<R> {
        StringReader { r: self }
    }

    /// Reads each record of the data as a `Vec<u8>`.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::Reader;
    ///
    /// let mut reader = Reader::from_bytes("abcd1234".as_bytes()).width(8);
    ///
    /// for record in reader.byte_reader() {
    ///     assert_eq!(record.unwrap(), b"abcd1234".to_vec())
    /// }
    /// ```
    pub fn byte_reader(&mut self) -> ByteReader<R> {
        ByteReader { r: self }
    }

    /// Reads the next record as a byte slice
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::Reader;
    ///
    /// let data = "foobar1234foobaz6789";
    ///
    /// let mut reader = Reader::from_string(data).width(10);
    ///
    /// if let Some(Ok(row)) = reader.next_record() {
    ///     assert_eq!(row, b"foobar1234");
    /// }
    ///
    /// if let Some(Ok(row)) = reader.next_record() {
    ///     assert_eq!(row, b"foobaz6789");
    /// }
    /// ```
    pub fn next_record(&mut self) -> Option<Result<&[u8]>> {
        if self.eof {
            return None;
        }

        match self.fill_buf() {
            Ok(0) => return None,
            Ok(_) => {}
            Err(e) => return Some(Err(e)),
        }

        if let Err(e) = self.read_linebreak() {
            return Some(Err(e));
        }

        Some(Ok(&self.buf))
    }

    /// Defines the width of each record in the file. It is required to set prior to reading
    /// since fixed width data is not self describing. Consumers must tell the reader how many
    /// bytes to read for each field. Do not include linebreaks in the width, you should only
    /// define a width to be the number of bytes in the record data itself.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::Reader;
    /// use std::result;
    ///
    /// let data = "foobar";
    /// let mut reader = Reader::from_string(data).width(3);
    /// let records: Vec<String> = reader.string_reader().filter_map(result::Result::ok).collect();
    ///
    /// assert_eq!(records, vec!["foo".to_string(), "bar".to_string()]);
    /// ```
    ///
    /// ### Example
    ///
    /// With a `LineBreak` specified:
    ///
    /// ```rust
    /// use fixed_width::{LineBreak, Reader};
    /// use std::result;
    ///
    /// let data = "foo\nbar";
    /// let mut reader = Reader::from_string(data).width(3).linebreak(LineBreak::Newline);
    /// let records: Vec<String> = reader.string_reader().filter_map(result::Result::ok).collect();
    ///
    /// assert_eq!(records, vec!["foo".to_string(), "bar".to_string()]);
    /// ```
    pub fn width(mut self, width: usize) -> Self {
        self.buf = vec![0; width];
        self.record_width = width;
        self
    }

    /// Defines the linebreak to use while reading data. Defaults to `LineBreak::None`, which means
    /// there are no bytes between records.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::{LineBreak, Reader};
    /// use std::result;
    ///
    /// let data = "foo\r\nbar";
    /// let mut reader = Reader::from_string(data).width(3).linebreak(LineBreak::CRLF);
    /// let records: Vec<String> = reader.string_reader().filter_map(result::Result::ok).collect();
    ///
    /// assert_eq!(records, vec!["foo".to_string(), "bar".to_string()]);
    /// ```
    pub fn linebreak(mut self, linebreak: LineBreak) -> Self {
        self.linebreak_buf = vec![0; linebreak.byte_width()];
        self.linebreak = linebreak;
        self
    }

    #[inline]
    fn has_linebreak(&self) -> bool {
        match self.linebreak {
            LineBreak::None => false,
            _ => true,
        }
    }

    #[inline]
    fn fill_buf(&mut self) -> Result<usize> {
        match self.rdr.read_exact(&mut self.buf) {
            Ok(_) => Ok(self.record_width),
            Err(e) => match e.kind() {
                io::ErrorKind::UnexpectedEof => {
                    self.eof = true;
                    Ok(0)
                }
                _ => Err(Error::from(e)),
            },
        }
    }

    // TODO: use skip_relative once stable
    #[inline]
    fn read_linebreak(&mut self) -> Result<()> {
        if !self.has_linebreak() {
            return Ok(());
        }

        if let Err(e) = self.rdr.read_exact(&mut self.linebreak_buf) {
            // There will not necessarily be a trailing line break, so if reading the linebreak
            // results in an EOF error, mark the reader done and return without error.
            match e.kind() {
                io::ErrorKind::UnexpectedEof => self.eof = true,
                _ => return Err(Error::from(e)),
            }
        }

        Ok(())
    }
}

impl Reader<fs::File> {
    /// Creates a new reader from a filepath. Will return an io::Error if there are any issues
    /// opening the file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self::from_reader(fs::File::open(path)?))
    }
}

impl Reader<io::Cursor<Vec<u8>>> {
    /// Creates a new reader from a series of bytes.
    pub fn from_bytes<T>(bytes: T) -> Self
    where
        T: Into<Vec<u8>>,
    {
        Self::from_reader(io::Cursor::new(bytes.into()))
    }

    /// Creates a new reader from a `String` or `&str`.
    pub fn from_string<T>(s: T) -> Self
    where
        T: Into<String>,
    {
        Self::from_bytes(s.into().into_bytes())
    }
}

impl<R> Read for Reader<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.rdr.read(buf)
    }
}

impl<'a, R> Iterator for ByteReader<'a, R>
where
    R: Read,
{
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.r
            .next_record()
            .map(|record| record.map(|r| r.to_vec()))
    }
}

impl<'a, R> Iterator for StringReader<'a, R>
where
    R: Read,
{
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        self.r
            .next_record()
            .map(|record| record.map(|r| String::from_utf8_lossy(r).to_string()))
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use super::*;
    use crate::{FieldSet, FixedWidth};
    use serde_derive::Deserialize;
    use std::result;

    #[test]
    fn read_next_record() {
        let s = "111122223333444411112222333344441111222233334444";

        let mut rdr = Reader::from_string(s).width(16);
        let mut count = 0;

        while let Some(r) = rdr.next_record() {
            count += 1;
            assert_eq!(b"1111222233334444", r.unwrap());
        }

        assert_eq!(3, count);
    }

    #[test]
    fn read_from_string() {
        let s = "111122223333444411112222333344441111222233334444";

        let mut rdr = Reader::from_string(s).width(16);

        let rows = rdr
            .string_reader()
            .filter_map(result::Result::ok)
            .collect::<Vec<String>>();

        assert_eq!(rows.len(), 3);

        for row in rows {
            assert_eq!("1111222233334444", row);
        }
    }

    #[test]
    fn read_from_string_with_newlines() {
        let s = "1111222233334444\n1111222233334444\n1111222233334444";

        let mut rdr = Reader::from_string(s)
            .width(16)
            .linebreak(LineBreak::Newline);

        let rows = rdr
            .string_reader()
            .filter_map(result::Result::ok)
            .collect::<Vec<String>>();

        assert_eq!(rows.len(), 3);

        for row in rows {
            assert_eq!("1111222233334444", row);
        }
    }

    #[test]
    fn read_from_string_with_crlf() {
        let s = "1111222233334444\r\n1111222233334444\r\n1111222233334444";

        let mut rdr = Reader::from_string(s).width(16).linebreak(LineBreak::CRLF);

        let rows = rdr
            .string_reader()
            .filter_map(result::Result::ok)
            .collect::<Vec<String>>();

        assert_eq!(rows.len(), 3);

        for row in rows {
            assert_eq!("1111222233334444", row);
        }
    }

    #[test]
    fn read_from_bytes() {
        let b = "111122223333444411112222333344441111222233334444".as_bytes();

        let mut rdr = Reader::from_bytes(b).width(16);

        let rows = rdr
            .string_reader()
            .filter_map(result::Result::ok)
            .collect::<Vec<String>>();

        assert_eq!(rows.len(), 3);

        for row in rows {
            assert_eq!("1111222233334444", row);
        }
    }

    #[test]
    fn read_from_bytes_with_crlf() {
        let b = "1111222233334444\r\n1111222233334444\r\n1111222233334444".as_bytes();

        let mut rdr = Reader::from_bytes(b).width(16).linebreak(LineBreak::CRLF);

        let rows = rdr
            .byte_reader()
            .filter_map(result::Result::ok)
            .collect::<Vec<Vec<u8>>>();

        assert_eq!(rows.len(), 3);

        for row in rows {
            assert_eq!(b"1111222233334444".to_vec(), row);
        }
    }

    #[derive(Deserialize)]
    struct Test {
        a: String,
        b: String,
        c: usize,
    }

    impl FixedWidth for Test {
        fn fields() -> FieldSet {
            FieldSet::Seq(vec![
                FieldSet::new_field(0..4),
                FieldSet::new_field(4..8),
                FieldSet::new_field(8..16),
            ])
        }
    }

    #[test]
    fn test_read() {
        let b = "111122223333444411112222333344441111222233334444".as_bytes();

        let mut rdr = Reader::from_bytes(b);

        let mut buf = Vec::with_capacity(16);
        let bytes_read = rdr.read(&mut buf).unwrap();

        assert_eq!(buf, b[..bytes_read].to_vec());
    }
}

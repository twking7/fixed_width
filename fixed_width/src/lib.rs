/*!
The `fixed_width` crate is designed to facilitate easy reading and writing of fixed width files.
It also provides a few useful abstractions to ease serializing and deserializing data into and out
of fixed width files.

Users of the crate will primarily use
[`Reader`](struct.Reader.html)
to read fixed width data and
[`Writer`](struct.Writer.html)
to write it.

You can read or write data as `Vec<String>` or as `Vec<Vec<u8>>`. If you use serde, then you
can also (de)serialize into and out of structs, HashMaps, etc. Since fixed width files are
not self describing, you will need to define the set of
[`Field`](struct.Field.html)
definitions for your data up front so the (de)serialization code can work.

Several errors may occur while using the library. These are defined in the
[`Error`](struct.Error.html)
type.

# Installing

Start by adding the dependency to your project in `Cargo.toml`:

```toml
fixed_width = "0.1"
```

Then in the root of your project:

```
use fixed_width;
```

There is also the `fixed_width_derive` crate that provides a struct attribute syntax to ease deriving
field definitions for your types. It is optional, but if you wish to use it you can add it to your
project like so in your `Cargo.toml`:

```toml
fixed_width = "0.1"
fixed_width_derive = "0.1"
```

# Usage

Reading a `String`:

```rust
use fixed_width::Reader;
use std::result;

let mut reader = Reader::from_string("record1record2").width(7);

let records: Vec<String> = reader.string_reader()
                                 .filter_map(result::Result::ok)
                                 .collect();
```

Reading a `String` into a `Vec` of user defined structs:

```rust
use serde_derive::Deserialize;
use serde;
use fixed_width::{Reader, FixedWidth, Field};
use std::result;

#[derive(Deserialize)]
struct Person {
    pub name: String,
    pub age: usize,
}

impl FixedWidth for Person {
    fn fields() -> Vec<Field> {
        vec![
            Field::default().range(0..6),
            Field::default().range(6..9),
        ]
    }
}

fn main() {
    let mut reader = Reader::from_string("foobar 25barfoo 35").width(9);
    let records: Vec<Person> = reader.byte_reader()
                                     .filter_map(result::Result::ok)
                                     .map(|bytes| fixed_width::from_bytes(&bytes).unwrap())
                                     .collect();
}
```
!*/
#![crate_name = "fixed_width"]
#![deny(missing_docs)]

use std::{convert, ops::Range, result};
pub use crate::de::{
    from_bytes, from_bytes_with_fields, from_str, from_str_with_fields, DeserializeError,
    Deserializer,
};
pub use crate::{
    error::Error, reader::{ByteReader, Reader, StringReader},
    ser::{to_bytes, to_string, to_writer, to_writer_with_fields, SerializeError, Serializer},
    writer::{AsByteSlice, Writer},
};

mod de;
mod error;
mod reader;
mod ser;
mod writer;

/// Convenience type for `Result` types pertaining to this library.
pub type Result<T> = result::Result<T, error::Error>;

/// Defines fixed width field definitions for a type.
pub trait FixedWidth {
    /// Returns an order independent `Vec` of field definitions.
    fn fields() -> Vec<Field>;
}

/// Justification of a fixed width field.
#[derive(Debug, Clone, PartialEq)]
pub enum Justify {
    /// Justify the field to the left in the record.
    Left,
    /// Justify the field to the right in the record.
    Right,
}

impl From<String> for Justify {
    fn from(s: String) -> Self {
        match s.to_lowercase().trim() {
            "right" => Justify::Right,
            _ => Justify::Left,
        }
    }
}

/// Defines a field in a fixed width record. There can be 1 or more fields in a fixed width record.
#[derive(Debug, Clone)]
pub struct Field {
    /// Name of the field.
    pub name: Option<String>,
    /// Byte range of the field.
    pub range: Range<usize>,
    /// The character to use for padding the field.
    pub pad_with: char,
    /// The justification (Left or Right) of the field.
    pub justify: Justify,
}

impl Default for Field {
    fn default() -> Self {
        Self {
            name: None,
            range: 0..0,
            pad_with: ' ',
            justify: Justify::Left,
        }
    }
}

impl Field {
    /// Sets the width of this field in bytes, as specified by the range (start - end).
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::Field;
    ///
    /// let field = Field::default().range(0..5);
    ///
    /// assert_eq!(field.width(), 5);
    /// ```
    pub fn width(&self) -> usize {
        self.range.end - self.range.start
    }

    /// Sets the name of this field. Mainly used when deserializing into a HashMap to derive the keys.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::Field;
    ///
    /// let field = Field::default().name(Some("thing"));
    ///
    /// assert_eq!(field.name, Some("thing".to_string()));
    /// ```
    pub fn name<T: Into<String>>(mut self, val: Option<T>) -> Self {
        self.name = val.map(convert::Into::into);
        self
    }

    /// Sets the range in bytes of this field. The start value represents the first byte of the field,
    /// and the end represents the last byte + 1 because this is an exclusive Range.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::Field;
    ///
    /// let field = Field::default().range(0..4);
    ///
    /// assert_eq!(field.range, 0..4);
    /// ```
    pub fn range(mut self, val: Range<usize>) -> Self {
        self.range = val;
        self
    }

    /// Sets the character to use as padding the value of this field to its byte width.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::Field;
    ///
    /// let field = Field::default().pad_with('a');
    ///
    /// assert_eq!(field.pad_with, 'a');
    /// ```
    pub fn pad_with(mut self, val: char) -> Self {
        self.pad_with = val;
        self
    }

    /// Sets the justification to use for this field. Left will align to the left and Right to the
    /// right.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::{Field, Justify};
    ///
    /// let field = Field::default().justify(Justify::Right);
    ///
    /// assert_eq!(field.justify, Justify::Right);
    /// ```
    pub fn justify<T: Into<Justify>>(mut self, val: T) -> Self {
        self.justify = val.into();
        self
    }
}

/// The type of line break between each record that should be inserted or skipped while reading.
#[derive(Debug, Clone, PartialEq)]
pub enum LineBreak {
    /// No linebreak
    None,
    /// Break lines with \n
    Newline,
    /// Break lines with \r\n
    CRLF,
}

impl LineBreak {
    /// The width in bytes of the given line break.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::LineBreak;
    ///
    /// let no_linebreak = LineBreak::None;
    /// let newline_linebreak = LineBreak::Newline;
    /// let crlf_linebreak = LineBreak::CRLF;
    ///
    /// assert_eq!(no_linebreak.byte_width(), 0);
    /// assert_eq!(newline_linebreak.byte_width(), 1);
    /// assert_eq!(crlf_linebreak.byte_width(), 2);
    /// ```
    pub fn byte_width(&self) -> usize {
        match self {
            LineBreak::None => 0,
            LineBreak::Newline => 1,
            LineBreak::CRLF => 2,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn line_break_byte_width() {
        assert_eq!(LineBreak::None.byte_width(), 0);
        assert_eq!(LineBreak::Newline.byte_width(), 1);
        assert_eq!(LineBreak::CRLF.byte_width(), 2);
    }

    #[test]
    fn field_building() {
        let field = Field::default()
            .range(0..10)
            .name(Some("foo"))
            .pad_with('a')
            .justify(Justify::Right);

        assert_eq!(field.range, 0..10);
        assert_eq!(field.name.unwrap(), "foo");
        assert_eq!(field.pad_with, 'a');
        assert_eq!(field.justify, Justify::Right);
    }

    #[test]
    fn field_width() {
        let field = Field::default().range(5..23);

        assert_eq!(field.width(), 18);
    }
}

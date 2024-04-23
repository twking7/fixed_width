/*!
The `fixed_width` crate is designed to facilitate easy reading and writing of fixed width files with
[serde](https://serde.rs/) support.
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
[`FieldSet`](enum.FieldSet.html)
definitions for your data up front so the (de)serialization code can work.

Several errors may occur while using the library. These are defined in the
[`Error`](enum.Error.html)
type.

# Installing

Start by adding the dependency to your project in `Cargo.toml`:

```toml
fixed_width = "0.5"
```

Then in the root of your project:

```
use fixed_width;
```

There is also the `fixed_width_derive` crate that provides a struct attribute syntax to ease deriving
field definitions for your types. It is optional, but if you wish to use it you can add it to your
project like so in your `Cargo.toml`:

```toml
fixed_width = "0.5"
fixed_width_derive = "0.5"
```

# Usage

Reading a `String`:

```rust
use fixed_width::Reader;
use std::result;

let mut reader = Reader::from_string("record1record2").width(7);

let records: Vec<String> = reader
    .string_reader()
    .filter_map(result::Result::ok)
    .collect();
```

Reading a `String` into a `Vec` of user defined structs:

```rust
use serde_derive::Deserialize;
use serde;
use fixed_width::{Reader, FixedWidth, FieldSet};
use std::result;

#[derive(Deserialize)]
struct Person {
    pub name: String,
    pub age: usize,
}

impl FixedWidth for Person {
    fn fields() -> FieldSet {
        FieldSet::Seq(vec![
            FieldSet::new_field(0..6),
            FieldSet::new_field(6..9),
        ])
    }
}

let mut reader = Reader::from_string("foobar 25barfoo 35").width(9);
let records: Vec<Person> = reader
    .byte_reader()
    .filter_map(result::Result::ok)
    .map(|bytes| fixed_width::from_bytes(&bytes).unwrap())
    .collect();
```
!*/
#![crate_name = "fixed_width"]
#![deny(missing_docs)]

pub use crate::de::{
    deserialize, from_bytes, from_bytes_with_fields, from_str, from_str_with_fields,
    DeserializeError, Deserializer,
};
pub use crate::{
    error::Error,
    reader::{ByteReader, Reader, StringReader},
    ser::{to_bytes, to_string, to_writer, to_writer_with_fields, SerializeError, Serializer},
    writer::{AsByteSlice, Writer},
};
use std::{ops::Range, result};

mod de;
mod error;
mod macros;
mod reader;
mod ser;
mod writer;

/// Convenience type for `Result` types pertaining to this library.
pub type Result<T> = result::Result<T, error::Error>;

/// Defines fixed width field definitions for a type.
pub trait FixedWidth {
    /// Returns field definitaions
    fn fields() -> FieldSet;
}

/// Justification of a fixed width field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Justify {
    /// Justify the field to the left in the record.
    Left,
    /// Justify the field to the right in the record.
    Right,
}

impl<T: AsRef<str>> From<T> for Justify {
    fn from(s: T) -> Self {
        match s.as_ref().to_lowercase().trim() {
            "right" => Justify::Right,
            "left" => Justify::Left,
            _ => panic!("Justify must be 'left' or 'right'"),
        }
    }
}

/// Defines a field in a fixed width record. There can be 1 or more fields in a fixed width record.
#[derive(Debug, Clone)]
pub struct FieldConfig {
    /// Name of the field.
    name: Option<String>,
    /// Byte range of the field.
    range: Range<usize>,
    /// The character to use for padding the field.
    pad_with: char,
    /// The justification (Left or Right) of the field.
    justify: Justify,
}

impl Default for FieldConfig {
    fn default() -> Self {
        Self {
            name: None,
            range: 0..0,
            pad_with: ' ',
            justify: Justify::Left,
        }
    }
}

impl FieldConfig {
    ///  Create a new field.
    ///
    /// ```rust
    /// use fixed_width::FieldConfig;
    ///
    /// let field = FieldConfig::new(0..1);
    /// ```
    pub fn new(range: Range<usize>) -> Self {
        FieldConfig {
            range,
            ..Default::default()
        }
    }

    fn width(&self) -> usize {
        self.range.end - self.range.start
    }
}

/// Field structure definition.
#[derive(Debug, Clone)]
pub enum FieldSet {
    /// For single Field
    Item(FieldConfig),
    /// For Sequence of Fields
    Seq(Vec<FieldSet>),
}

impl FieldSet {
    ///  Create a new field.
    ///
    /// ```rust
    /// use fixed_width::FieldSet;
    ///
    /// let field = FieldSet::new_field(0..1);
    /// ```
    pub fn new_field(range: std::ops::Range<usize>) -> Self {
        Self::Item(FieldConfig {
            range,
            ..Default::default()
        })
    }

    /// Sets the name of this field. Mainly used when deserializing into a HashMap to derive the keys.
    /// (This method is not valid on `FieldSet::Seq` and cause panic)
    ///
    /// ```rust
    /// use fixed_width::FieldSet;
    ///
    /// let fields = FieldSet::Seq(vec![
    ///     FieldSet::new_field(0..1).name("foo"),
    ///     FieldSet::Seq(vec![
    ///         FieldSet::new_field(0..2).name("bar"), FieldSet::new_field(0..3).name("baz")
    ///     ])
    /// ]);
    /// ```
    pub fn name<T: Into<String>>(mut self, val: T) -> Self {
        match &mut self {
            Self::Item(conf) => {
                conf.name = Some(val.into());
                self
            }
            _ => panic!("Setting name on FieldSet::Seq is not feasible."),
        }
    }

    /// Sets the character to use as padding the value of this field to its byte width.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::FieldSet;
    ///
    /// let field = FieldSet::new_field(0..1).pad_with('x');
    /// let fields = FieldSet::Seq(vec![
    ///     FieldSet::new_field(0..1),
    ///     FieldSet::Seq(vec![FieldSet::new_field(0..2), FieldSet::new_field(0..3)]),
    /// ])
    /// .pad_with('x');
    /// ```
    pub fn pad_with(mut self, val: char) -> Self {
        match self {
            Self::Item(ref mut config) => {
                config.pad_with = val;
                self
            }
            Self::Seq(seq) => Self::Seq(seq.into_iter().map(|fs| fs.pad_with(val)).collect()),
        }
    }

    /// Sets the justification to use fields. Left will align to the left and Right to the
    /// right.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::{FieldSet, Justify};
    ///
    /// let field = FieldSet::new_field(0..1).justify(Justify::Right);
    /// let fields = FieldSet::Seq(vec![
    ///     FieldSet::new_field(0..1),
    ///     FieldSet::Seq(vec![FieldSet::new_field(0..2), FieldSet::new_field(0..3)]),
    /// ])
    /// .justify(Justify::Right);
    /// ```
    pub fn justify<T: Into<Justify>>(mut self, val: T) -> Self {
        let val = val.into();
        match self {
            Self::Item(ref mut config) => {
                config.justify = val;
                self
            }
            Self::Seq(seq) => Self::Seq(seq.into_iter().map(|fs| fs.justify(val)).collect()),
        }
    }

    /// Append `FieldSet` with the given item.
    ///
    /// ### Example
    /// ```rust
    /// use fixed_width::FieldSet;
    ///
    /// // Suppose field defined as:
    /// let append_fields_1 = FieldSet::new_field(0..1).append(FieldSet::new_field(1..2));
    /// let append_fields_2 = FieldSet::new_field(0..1).append(
    ///     FieldSet::Seq(vec![
    ///         FieldSet::new_field(1..2),
    ///         FieldSet::new_field(2..3),
    ///     ])
    /// );
    ///
    /// // Are identical to:
    /// let fields_1 = FieldSet::Seq(vec![
    ///     FieldSet::new_field(0..1),
    ///     FieldSet::new_field(1..2),
    /// ]);
    /// let fields_2 = FieldSet::Seq(vec![
    ///     FieldSet::new_field(0..1),
    ///     FieldSet::Seq(vec![
    ///         FieldSet::new_field(1..2),
    ///         FieldSet::new_field(2..3),
    ///     ]),
    /// ]);
    ///
    /// # assert_eq!(
    /// #     format!("{:?}", append_fields_1),
    /// #     format!("{:?}", FieldSet::Seq(vec![
    /// #         FieldSet::new_field(0..1),
    /// #         FieldSet::new_field(1..2),
    /// #     ]))
    /// # );
    /// # assert_eq!(
    /// #     format!("{:?}", append_fields_2),
    /// #     format!("{:?}", FieldSet::Seq(vec![
    /// #         FieldSet::new_field(0..1),
    /// #         FieldSet::Seq(vec![
    /// #             FieldSet::new_field(1..2),
    /// #             FieldSet::new_field(2..3),
    /// #         ]),
    /// #     ])),
    /// # );
    /// ```
    pub fn append(self, item: Self) -> Self {
        match self {
            Self::Item(_) => Self::Seq(vec![self, item]),
            Self::Seq(mut seq) => {
                seq.append(&mut vec![item]);
                Self::Seq(seq)
            }
        }
    }

    /// Extend `FieldSet` with the given item.
    ///
    /// ### Example
    /// ```rust
    /// use fixed_width::FieldSet;
    ///
    /// // Suppose field defined as:
    /// let extend_fields_1 = FieldSet::new_field(0..1).extend(FieldSet::new_field(1..2));
    /// let extend_fields_2 = FieldSet::new_field(0..1).extend(
    ///     FieldSet::Seq(vec![
    ///         FieldSet::new_field(1..2),
    ///         FieldSet::new_field(2..3),
    ///     ])
    /// );
    ///
    /// // Are identical to:
    /// let fields_1 = FieldSet::Seq(vec![
    ///     FieldSet::new_field(0..1),
    ///     FieldSet::new_field(1..2),
    /// ]);
    /// let fields_2 = FieldSet::Seq(vec![
    ///     FieldSet::new_field(0..1),
    ///     FieldSet::new_field(1..2),
    ///     FieldSet::new_field(2..3),
    /// ]);
    ///
    /// # assert_eq!(
    /// #     format!("{:?}", extend_fields_1),
    /// #     format!("{:?}", FieldSet::Seq(vec![
    /// #         FieldSet::new_field(0..1),
    /// #         FieldSet::new_field(1..2),
    /// #     ]))
    /// # );
    /// # assert_eq!(
    /// #     format!("{:?}", extend_fields_2),
    /// #     format!("{:?}", FieldSet::Seq(vec![
    /// #         FieldSet::new_field(0..1),
    /// #         FieldSet::new_field(1..2),
    /// #         FieldSet::new_field(2..3),
    /// #     ])),
    /// # );
    /// ```
    pub fn extend(self, item: Self) -> Self {
        match self {
            Self::Item(_) => match item {
                Self::Item(_) => self.append(item),
                Self::Seq(_) => Self::Seq(vec![self]).extend(item),
            },
            Self::Seq(mut seq) => {
                seq.extend(item);
                Self::Seq(seq)
            }
        }
    }

    /// Converts `FieldSet` into flatten `Vec<FieldConfig>`.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::{FieldConfig, FieldSet};
    ///
    /// let fields = FieldSet::Seq(vec![
    ///     FieldSet::Seq(vec![FieldSet::new_field(0..1), FieldSet::new_field(1..2)]),
    ///     FieldSet::new_field(2..3)
    /// ]);
    /// let flatten_fields = vec![
    ///     FieldConfig::new(0..1), FieldConfig::new(1..2), FieldConfig::new(2..3)
    /// ];
    ///
    /// assert_eq!(format!("{:?}", fields.flatten()), format!("{:?}", flatten_fields));
    /// ```
    pub fn flatten(self) -> Vec<FieldConfig> {
        let mut flatten = vec![];
        let mut stack = vec![vec![self]];

        while !stack.is_empty() {
            let last = stack.last_mut().unwrap();
            if last.is_empty() {
                stack.pop();
            } else {
                let field = last.drain(..1).next().unwrap();
                match field {
                    FieldSet::Item(conf) => flatten.push(conf),
                    FieldSet::Seq(seq) => stack.push(seq.to_vec()),
                }
            }
        }

        flatten
    }
}

impl IntoIterator for FieldSet {
    type Item = FieldSet;
    type IntoIter = std::vec::IntoIter<FieldSet>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            field @ FieldSet::Item(_) => vec![field].into_iter(),
            FieldSet::Seq(seq) => seq.into_iter(),
        }
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
    fn fieldset_name() {
        let field = FieldSet::new_field(0..0).name("foo");
        let field = field.flatten().pop().unwrap();
        assert_eq!(field.name.as_ref().unwrap(), "foo");
    }

    #[test]
    #[should_panic]
    fn failed_on_fieldset_name() {
        FieldSet::Seq(vec![]).name("foo");
    }

    #[test]
    fn fieldset_pad_with() {
        let fields = FieldSet::Seq(vec![
            FieldSet::new_field(0..1),
            FieldSet::Seq(vec![FieldSet::new_field(0..2), FieldSet::new_field(0..3)]),
        ])
        .pad_with('a');

        for field in fields.flatten() {
            assert_eq!(field.pad_with, 'a')
        }
    }

    #[test]
    fn fieldset_justify() {
        let fields = FieldSet::Seq(vec![
            FieldSet::new_field(0..1),
            FieldSet::Seq(vec![FieldSet::new_field(0..2), FieldSet::new_field(0..3)]),
        ])
        .justify(Justify::Right);

        for field in fields.flatten() {
            assert_eq!(field.justify, Justify::Right)
        }
    }

    #[test]
    fn fieldset_justify_str() {
        let fields = FieldSet::Seq(vec![
            FieldSet::new_field(0..1),
            FieldSet::Seq(vec![FieldSet::new_field(0..2), FieldSet::new_field(0..3)]),
        ])
        .justify("right");

        for field in fields.flatten() {
            assert_eq!(field.justify, Justify::Right)
        }
    }

    #[test]
    #[should_panic]
    fn fieldset_justify_panic() {
        let _ = FieldSet::Seq(vec![
            FieldSet::new_field(0..1),
            FieldSet::Seq(vec![FieldSet::new_field(0..2), FieldSet::new_field(0..3)]),
        ])
        .justify("foo");
    }

    #[test]
    fn field_building() {
        let field = FieldSet::new_field(0..10)
            .name("foo")
            .pad_with('a')
            .justify(Justify::Right);
        let field = field.flatten().pop().unwrap();

        assert_eq!(field.range, 0..10);
        assert_eq!(field.name.as_ref().unwrap(), "foo");
        assert_eq!(field.pad_with, 'a');
        assert_eq!(field.justify, Justify::Right);
    }
}

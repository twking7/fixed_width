use crate::{error::Error, writer::Writer, FieldConfig, FieldSet, FixedWidth, Justify, Result};
use serde::ser::{self, Error as SerError, Serialize};
use std::{error::Error as StdError, fmt, io, iter, vec};

/// Serializes the given type that implements `FixedWidth` and `Serialize` to a `String`.
///
/// ### Example
///
/// ```rust
/// use serde_derive::Serialize;
/// use serde;
/// use fixed_width::{FieldSet, FixedWidth};
///
/// #[derive(Serialize)]
/// struct Record {
///     pub name: String,
///     pub room: usize,
/// }
///
/// impl FixedWidth for Record {
///     fn fields() -> FieldSet {
///         FieldSet::Seq(vec![
///             FieldSet::new_field(0..4),
///             FieldSet::new_field(4..8),
///         ])
///     }
/// }
///
/// let record = Record { name: "Carl".to_string(), room: 1234 };
/// let s = fixed_width::to_string(&record).unwrap();
///
/// assert_eq!(s, "Carl1234");
/// ```
pub fn to_string<T: FixedWidth + Serialize>(record: &T) -> Result<String> {
    let mut w = Writer::from_memory();
    to_writer(&mut w, record)?;
    Ok(w.into())
}

/// Serializes the given type that implements `FixedWidth` and `Serialize` to a `String`.
///
/// ### Example
///
/// ```rust
/// use serde_derive::Serialize;
/// use serde;
/// use fixed_width::{FixedWidth, FieldSet};
///
/// #[derive(Serialize)]
/// struct Record {
///     pub name: String,
///     pub room: usize,
/// }
///
/// impl FixedWidth for Record {
///     fn fields() -> FieldSet {
///         FieldSet::Seq(vec![
///             FieldSet::new_field(0..4),
///             FieldSet::new_field(4..8),
///         ])
///     }
/// }
///
/// let record = Record { name: "Carl".to_string(), room: 1234 };
/// let s = fixed_width::to_bytes(&record).unwrap();
///
/// assert_eq!(&s, b"Carl1234");
/// ```
pub fn to_bytes<T: FixedWidth + Serialize>(record: &T) -> Result<Vec<u8>> {
    let mut w = Writer::from_memory();
    to_writer(&mut w, record)?;
    Ok(w.into())
}

/// Serializes a type that implements `FixedWidth` to the given writer. Similar to
/// `to_writer_with_fields`, but this function uses the fields defined in the trait implementation.
///
/// ### Example
///
/// ```rust
/// use serde_derive::Serialize;
/// use serde;
/// use fixed_width::{FixedWidth, Writer, FieldSet};
///
/// #[derive(Serialize)]
/// struct Person {
///     pub name: String,
///     pub age: usize,
/// }
///
/// impl FixedWidth for Person {
///     fn fields() -> FieldSet {
///         FieldSet::Seq(vec![
///             FieldSet::new_field(0..8),
///             FieldSet::new_field(8..10),
///         ])
///     }
/// }
///
/// let mut w = Writer::from_memory();
///
/// let person = Person {
///     name: "coolname".to_string(),
///     age: 25,
/// };
///
/// fixed_width::to_writer(&mut w, &person).unwrap();
///
/// let s: String = w.into();
/// assert_eq!("coolname25", s);
/// ```
pub fn to_writer<'w, T, W>(wrtr: &'w mut W, val: &T) -> Result<()>
where
    T: FixedWidth + Serialize,
    W: 'w + io::Write,
{
    to_writer_with_fields(wrtr, val, T::fields())
}

/// Serializes data to the given writer using the provided `Field`s.
///
/// ### Example
///
/// ```rust
/// use fixed_width::{FieldSet, Writer, to_writer_with_fields};
///
/// let fields = FieldSet::Seq(vec![
///     FieldSet::new_field(0..4),
///     FieldSet::new_field(4..8),
/// ]);
/// let mut w = Writer::from_memory();
/// let data = vec!["1234", "abcd"];
///
/// to_writer_with_fields(&mut w, &data, fields).unwrap();
///
/// let s: String = w.into();
/// assert_eq!("1234abcd", s);
/// ```
pub fn to_writer_with_fields<'w, T, W>(wrtr: &'w mut W, val: &T, fields: FieldSet) -> Result<()>
where
    T: Serialize,
    W: 'w + io::Write,
{
    let mut ser = Serializer::new(wrtr, fields);
    val.serialize(&mut ser)
}

/// Errors that occur during serialization.
#[derive(Debug)]
pub enum SerializeError {
    /// General error message as a `String`.
    Message(String),
    /// The desired type is unsupported by this serializer.
    Unsupported(String),
    /// The number of `Field`s given were less than the number of values to be serialized.
    UnexpectedEndOfFields,
}

impl fmt::Display for SerializeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SerializeError::Message(ref e) => write!(f, "{}", e),
            SerializeError::Unsupported(ref e) => write!(f, "{}", e),
            SerializeError::UnexpectedEndOfFields => write!(f, "Unexpected End of Fields"),
        }
    }
}

impl StdError for SerializeError {
    fn cause(&self) -> Option<&dyn StdError> {
        None
    }
}

impl SerError for Error {
    fn custom<T: fmt::Display>(msg: T) -> Error {
        Error::from(SerializeError::Message(msg.to_string()))
    }
}

/// A serializer for fixed width data. Writes to the given Writer using the provided field
/// definitions to determine how to serialize data into records.
pub struct Serializer<'w, W: 'w + io::Write> {
    fields: iter::Peekable<vec::IntoIter<FieldConfig>>,
    wrtr: &'w mut W,
}

impl<'w, W: 'w + io::Write> Serializer<'w, W> {
    /// Creates a new Serializer from a Writer and a set of field definitions.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use serde;
    /// use fixed_width::{FieldSet, Serializer, Writer};
    /// use serde::Serialize;
    ///
    /// let fields = FieldSet::Seq(vec![
    ///     FieldSet::new_field(0..4).name("letters"),
    ///     FieldSet::new_field(4..8).name("numbers"),
    /// ]);
    ///
    /// let mut writer = Writer::from_memory();
    /// let mut record = vec!["abcd", "1234"];
    ///
    /// {
    ///     let mut ser = Serializer::new(&mut writer, fields);
    ///     record.serialize(&mut ser);
    /// }
    ///
    /// let s: String = writer.into();
    /// assert_eq!("abcd1234", s);
    /// ```
    pub fn new(wrtr: &'w mut W, fields: FieldSet) -> Self {
        Self {
            fields: fields.flatten().into_iter().peekable(),
            wrtr,
        }
    }

    fn next_field(&mut self) -> Result<FieldConfig> {
        match self.fields.next() {
            Some(f) => Ok(f),
            None => Err(Error::from(SerializeError::UnexpectedEndOfFields)),
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.wrtr.write_all(bytes)?;
        Ok(())
    }
}

macro_rules! serialize_with_str {
    ($ser_fn:ident, $int_ty:ty) => {
        fn $ser_fn(self, val: $int_ty) -> Result<Self::Ok> {
            self.serialize_str(&val.to_string())
        }
    };
}

impl<'a, 'w, W: io::Write> ser::Serializer for &'a mut Serializer<'w, W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    serialize_with_str!(serialize_u8, u8);
    serialize_with_str!(serialize_i8, i8);
    serialize_with_str!(serialize_u16, u16);
    serialize_with_str!(serialize_i16, i16);
    serialize_with_str!(serialize_u32, u32);
    serialize_with_str!(serialize_i32, i32);
    serialize_with_str!(serialize_u64, u64);
    serialize_with_str!(serialize_i64, i64);
    serialize_with_str!(serialize_f32, f32);
    serialize_with_str!(serialize_f64, f64);
    serialize_with_str!(serialize_char, char);

    fn serialize_bool(self, val: bool) -> Result<Self::Ok> {
        self.serialize_str(&(val as u8).to_string())
    }

    fn serialize_str(self, val: &str) -> Result<Self::Ok> {
        let bytes = val.as_bytes();
        self.serialize_bytes(bytes)
    }

    fn serialize_bytes(self, val: &[u8]) -> Result<Self::Ok> {
        let bytes = pad(val, &self.next_field()?);
        self.write_bytes(&bytes)
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.serialize_bytes(&[])
    }

    fn serialize_some<T: ?Sized + Serialize>(self, val: &T) -> Result<Self::Ok> {
        val.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        None::<()>.serialize(self)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        val: &T,
    ) -> Result<Self::Ok> {
        val.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        val: &T,
    ) -> Result<Self::Ok> {
        val.serialize(&mut *self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        variant.serialize(&mut *self)?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(SerializeError::Unsupported("serialize_map".to_string()).into())
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        variant.serialize(&mut *self)?;
        Ok(self)
    }
}

impl<'a, 'w, W: io::Write> ser::SerializeSeq for &'a mut Serializer<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, W: io::Write> ser::SerializeTuple for &'a mut Serializer<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, W: io::Write> ser::SerializeTupleStruct for &'a mut Serializer<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, W: io::Write> ser::SerializeTupleVariant for &'a mut Serializer<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, W: io::Write> ser::SerializeMap for &'a mut Serializer<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, _key: &T) -> Result<()> {
        unreachable!()
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, _value: &T) -> Result<()> {
        unreachable!()
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, W: io::Write> ser::SerializeStruct for &'a mut Serializer<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, W: io::Write> ser::SerializeStructVariant for &'a mut Serializer<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

#[inline]
fn pad(bytes: &[u8], field: &FieldConfig) -> Vec<u8> {
    let width = field.width();
    let pad = field.pad_with as u8;
    let mut v = bytes.to_vec();

    if v.len() > width {
        v.resize(width, pad);
    } else {
        for _ in 0..(width - v.len()) {
            match field.justify {
                Justify::Left => v.push(pad),
                _ => v.insert(0, pad),
            }
        }
    }

    v
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{FieldSet, FixedWidth, Writer};
    use serde_bytes::ByteBuf;
    use serde_derive::Serialize;
    use std::collections::HashMap;

    #[test]
    fn bool_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::new_field(0..1);
        to_writer_with_fields(&mut wrtr, &true, fields.clone()).unwrap();
        to_writer_with_fields(&mut wrtr, &false, fields.clone()).unwrap();
        let s: String = wrtr.into();

        assert_eq!(s, "10");
    }

    #[test]
    fn int_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::new_field(0..4);

        to_writer_with_fields(&mut wrtr, &123_u8, fields.clone()).unwrap();
        to_writer_with_fields(&mut wrtr, &-123_i8, fields.clone()).unwrap();
        to_writer_with_fields(&mut wrtr, &123_u16, fields.clone()).unwrap();
        to_writer_with_fields(&mut wrtr, &-123_i16, fields.clone()).unwrap();
        to_writer_with_fields(&mut wrtr, &123_u32, fields.clone()).unwrap();
        to_writer_with_fields(&mut wrtr, &-123_i32, fields.clone()).unwrap();
        to_writer_with_fields(&mut wrtr, &123_u64, fields.clone()).unwrap();
        to_writer_with_fields(&mut wrtr, &-123_i64, fields.clone()).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, "123 -123123 -123123 -123123 -123");
    }

    #[test]
    fn float_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::new_field(0..4);

        to_writer_with_fields(&mut wrtr, &(12.3 as f32), fields.clone()).unwrap();
        to_writer_with_fields(&mut wrtr, &(-2.3 as f32), fields.clone()).unwrap();
        to_writer_with_fields(&mut wrtr, &(24.6 as f64), fields.clone()).unwrap();
        to_writer_with_fields(&mut wrtr, &(-2.6 as f64), fields.clone()).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, "12.3-2.324.6-2.6");
    }

    #[test]
    fn str_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::new_field(0..4);

        let st = "foo".to_string();
        to_writer_with_fields(&mut wrtr, &st, fields).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, "foo ");
    }

    #[test]
    fn bytes_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::new_field(0..4);

        let bytes = ByteBuf::from(b"foo".to_vec());
        to_writer_with_fields(&mut wrtr, &bytes, fields).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, "foo ");
    }

    #[test]
    fn none_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::new_field(0..4);

        let none: Option<usize> = None;
        to_writer_with_fields(&mut wrtr, &none, fields).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, "    ");
    }

    #[test]
    fn some_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::new_field(0..4);

        to_writer_with_fields(&mut wrtr, &Some(" foo"), fields).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, " foo");
    }

    #[test]
    fn unit_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::new_field(0..4);

        to_writer_with_fields(&mut wrtr, &(), fields).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, "    ");
    }

    #[derive(Debug, Serialize)]
    struct Unit;

    #[test]
    fn unit_struct_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::new_field(0..4);

        to_writer_with_fields(&mut wrtr, &Unit, fields).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, "    ");
    }

    #[derive(Debug, Serialize)]
    struct Newtype(usize);

    #[test]
    fn newtype_struct_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::new_field(0..4);

        to_writer_with_fields(&mut wrtr, &Newtype(123), fields).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, "123 ");
    }

    #[test]
    fn seq_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::Seq(vec![FieldSet::new_field(0..4), FieldSet::new_field(0..3)]);

        to_writer_with_fields(&mut wrtr, &[111, 222], fields).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, "111 222");
    }

    #[test]
    fn tuple_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::Seq(vec![FieldSet::new_field(0..4), FieldSet::new_field(0..3)]);

        to_writer_with_fields(&mut wrtr, &(111, 222), fields).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, "111 222");
    }

    #[derive(Debug, Serialize)]
    struct Tuple(usize, usize);

    #[test]
    fn tuple_struct_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::Seq(vec![FieldSet::new_field(0..4), FieldSet::new_field(0..3)]);

        to_writer_with_fields(&mut wrtr, &Tuple(111, 222), fields).unwrap();

        let s: String = wrtr.into();
        assert_eq!(s, "111 222");
    }

    #[test]
    fn map_ser() {
        let mut wrtr = Writer::from_memory();
        let fields = FieldSet::Seq(vec![FieldSet::new_field(0..4), FieldSet::new_field(0..3)]);

        let mut h = HashMap::new();
        h.insert("foo", 123);
        h.insert("bar", 456);

        let res = to_writer_with_fields(&mut wrtr, &h, fields);

        match res {
            Ok(_) => assert!(false, "should not be Ok"),
            Err(Error::SerializeError(SerializeError::Unsupported(_))) => assert!(true),
            Err(_) => assert!(false, "should be an unsupported error"),
        };
    }

    #[derive(Debug, Serialize)]
    struct Test1 {
        a: usize,
        b: String,
        c: f64,
        d: Option<usize>,
    }

    impl FixedWidth for Test1 {
        fn fields() -> FieldSet {
            FieldSet::Seq(vec![
                FieldSet::new_field(0..3),
                FieldSet::new_field(3..6),
                FieldSet::new_field(6..10),
                FieldSet::new_field(10..13),
            ])
        }
    }

    #[test]
    fn struct_ser() {
        let test = Test1 {
            a: 123,
            b: "abc".to_string(),
            c: 9876.0,
            d: Some(12),
        };

        let mut w = Writer::from_memory();
        to_writer(&mut w, &test).unwrap();
        let s: String = w.into();

        assert_eq!(s, "123abc987612 ");
    }

    #[test]
    fn pad_left_justified() {
        let inputs = vec!["123456789".as_bytes(), "12345".as_bytes(), "123".as_bytes()];
        let field = &FieldSet::new_field(0..5)
            .justify(Justify::Left)
            .pad_with('T')
            .flatten()[0];

        let expected = vec!["12345".as_bytes(), "12345".as_bytes(), "123TT".as_bytes()];

        for (i, input) in inputs.iter().enumerate() {
            let padded = pad(input, field);
            assert_eq!(padded, expected[i].to_vec());
        }
    }

    #[test]
    fn pad_right_justified() {
        let inputs = vec!["123456789".as_bytes(), "12345".as_bytes(), "123".as_bytes()];
        let field = &FieldSet::new_field(0..5)
            .justify(Justify::Right)
            .pad_with('T')
            .flatten()[0];

        let expected = vec!["12345".as_bytes(), "12345".as_bytes(), "TT123".as_bytes()];

        for (i, input) in inputs.iter().enumerate() {
            let padded = pad(input, field);
            println!("{:?}", padded);
            assert_eq!(padded, expected[i].to_vec());
        }
    }

    #[test]
    fn to_string_ser() {
        let test = Test1 {
            a: 123,
            b: "abc".to_string(),
            c: 9876.0,
            d: Some(12),
        };

        let s = to_string(&test).unwrap();
        assert_eq!(s, "123abc987612 ");
    }

    #[test]
    fn to_bytes_ser() {
        let test = Test1 {
            a: 123,
            b: "abc".to_string(),
            c: 9876.0,
            d: Some(12),
        };

        let b = to_bytes(&test).unwrap();
        assert_eq!(b, b"123abc987612 ".to_vec());
    }

    #[derive(Serialize)]
    struct Test2 {
        a: Test1,
        b: Test1,
    }

    impl FixedWidth for Test2 {
        fn fields() -> FieldSet {
            FieldSet::Seq(vec![
                FieldSet::Seq(vec![
                    FieldSet::new_field(0..3),
                    FieldSet::new_field(3..6),
                    FieldSet::new_field(6..10),
                    FieldSet::new_field(10..13),
                ]),
                FieldSet::Seq(vec![
                    FieldSet::new_field(13..16),
                    FieldSet::new_field(16..19),
                    FieldSet::new_field(19..23),
                    FieldSet::new_field(23..26),
                ]),
            ])
        }
    }

    #[test]
    fn nested_struct() {
        let test = Test2 {
            a: Test1 {
                a: 123,
                b: "abc".to_string(),
                c: 9876.0,
                d: Some(12),
            },
            b: Test1 {
                a: 321,
                b: "cba".to_string(),
                c: 6789.0,
                d: Some(21),
            },
        };

        let s = to_string(&test).unwrap();
        assert_eq!(s, "123abc987612 321cba678921 ".to_string());
    }
}

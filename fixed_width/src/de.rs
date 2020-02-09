use crate::{error, Field, FixedWidth};
use serde::{
    self,
    de::{self, Deserialize, Error, IntoDeserializer, Visitor},
};
use std::{convert, error::Error as StdError, fmt, iter, num, result::Result, str, vec};

/// Deserializes a `&str` into the given type that implements `FixedWidth` and `Deserialize`.
///
/// ### Example
///
/// ```rust
/// use serde_derive::Deserialize;
/// use serde;
/// use fixed_width::{Field, FixedWidth};
///
/// #[derive(Deserialize)]
/// struct Record {
///     pub name: String,
///     pub room: usize,
/// }
///
/// impl FixedWidth for Record {
///     fn fields() -> Vec<Field> {
///         vec![
///             Field::default().range(0..4),
///             Field::default().range(4..8),
///         ]
///     }
/// }
///
/// fn main() {
///     let s = "Carl1234";
///     let record: Record = fixed_width::from_str(&s).unwrap();
///
///     assert_eq!(record.name, "Carl");
///     assert_eq!(record.room, 1234);
/// }
/// ```
pub fn from_str<'de, T>(s: &'de str) -> Result<T, error::Error>
where
    T: FixedWidth + Deserialize<'de>,
{
    from_str_with_fields(s, T::fields())
}

/// Deserializes a `&[u8]` into the given type that implements `FixedWidth` and `Deserialize`.
///
/// ### Example
///
/// ```rust
/// use serde_derive::Deserialize;
/// use serde;
/// use fixed_width::{Field, FixedWidth};
///
/// #[derive(Deserialize)]
/// struct Record {
///     pub name: String,
///     pub room: usize,
/// }
///
/// impl FixedWidth for Record {
///     fn fields() -> Vec<Field> {
///         vec![
///             Field::default().range(0..4),
///             Field::default().range(4..8),
///         ]
///     }
/// }
///
/// fn main() {
///     let b = b"Carl1234";
///     let record: Record = fixed_width::from_bytes(b).unwrap();
///
///     assert_eq!(record.name, "Carl");
///     assert_eq!(record.room, 1234);
/// }
/// ```
pub fn from_bytes<'de, T>(b: &'de [u8]) -> Result<T, error::Error>
where
    T: FixedWidth + Deserialize<'de>,
{
    from_bytes_with_fields(b, T::fields())
}

/// Deserializes `&str` data to the given writer using the provided `Field`s.
///
/// ### Example
///
/// ```rust
/// use std::collections::HashMap;
/// use fixed_width::{Field, from_str_with_fields};
///
/// let fields = vec![
///     Field::default().range(0..4).name(Some("numbers")),
///     Field::default().range(4..8).name(Some("letters")),
/// ];
/// let mut s = "1234abcd";
///
/// let h: HashMap<String, String> = from_str_with_fields(s, fields).unwrap();
/// assert_eq!(h.get("numbers").unwrap(), "1234");
/// assert_eq!(h.get("letters").unwrap(), "abcd");
/// ```
pub fn from_str_with_fields<'de, T>(s: &'de str, fields: Vec<Field>) -> Result<T, error::Error>
where
    T: Deserialize<'de>,
{
    from_bytes_with_fields(s.as_bytes(), fields)
}

/// Deserializes `&[u8]` data to the given writer using the provided `Field`s.
///
/// ### Example
///
/// ```rust
/// use std::collections::HashMap;
/// use fixed_width::{Field, from_bytes_with_fields};
///
/// let fields = vec![
///     Field::default().range(0..4).name(Some("numbers")),
///     Field::default().range(4..8).name(Some("letters")),
/// ];
/// let mut bytes = b"1234abcd";
///
/// let h: HashMap<String, String> = from_bytes_with_fields(bytes, fields).unwrap();
/// assert_eq!(h.get("numbers").unwrap(), "1234");
/// assert_eq!(h.get("letters").unwrap(), "abcd");
/// ```
pub fn from_bytes_with_fields<'de, T>(
    bytes: &'de [u8],
    fields: Vec<Field>,
) -> Result<T, error::Error>
where
    T: Deserialize<'de>,
{
    let mut de = Deserializer::new(bytes, fields);
    T::deserialize(&mut de).map_err(convert::Into::into)
}

/// Errors that occur during deserialization.
#[derive(Debug)]
pub enum DeserializeError {
    /// General error message as a `String`.
    Message(String),
    /// The desired type is unsupported by this deserializer.
    Unsupported(String),
    /// The number of `Field`s given were less than the number of values to be deserialized.
    UnexpectedEndOfRecord,
    /// The bytes given were not valid UTF-8.
    InvalidUtf8(str::Utf8Error),
    /// A boolean value could not be parsed for this field.
    ParseBoolError(str::ParseBoolError),
    /// An integer value could not be parsed for this field.
    ParseIntError(num::ParseIntError),
    /// A float value could not be parsed for this field.
    ParseFloatError(num::ParseFloatError),
}

impl serde::de::Error for DeserializeError {
    fn custom<T: fmt::Display>(msg: T) -> DeserializeError {
        DeserializeError::Message(msg.to_string())
    }
}

impl StdError for DeserializeError {
    fn cause(&self) -> Option<&dyn StdError> {
        match self {
            DeserializeError::Message(_e) => None,
            DeserializeError::Unsupported(_e) => None,
            DeserializeError::UnexpectedEndOfRecord => None,
            DeserializeError::InvalidUtf8(e) => Some(e),
            DeserializeError::ParseBoolError(e) => Some(e),
            DeserializeError::ParseIntError(e) => Some(e),
            DeserializeError::ParseFloatError(e) => Some(e),
        }
    }
}

impl fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeserializeError::Message(ref e) => write!(f, "{}", e),
            DeserializeError::Unsupported(ref e) => write!(f, "{}", e),
            DeserializeError::UnexpectedEndOfRecord => {
                write!(f, "byte length of record was less than defined length")
            }
            DeserializeError::InvalidUtf8(ref e) => write!(f, "{}", e),
            DeserializeError::ParseBoolError(ref e) => write!(f, "{}", e),
            DeserializeError::ParseIntError(ref e) => write!(f, "{}", e),
            DeserializeError::ParseFloatError(ref e) => write!(f, "{}", e),
        }
    }
}

impl From<str::Utf8Error> for DeserializeError {
    fn from(e: str::Utf8Error) -> Self {
        DeserializeError::InvalidUtf8(e)
    }
}

impl From<str::ParseBoolError> for DeserializeError {
    fn from(e: str::ParseBoolError) -> Self {
        DeserializeError::ParseBoolError(e)
    }
}

impl From<num::ParseIntError> for DeserializeError {
    fn from(e: num::ParseIntError) -> Self {
        DeserializeError::ParseIntError(e)
    }
}

impl From<num::ParseFloatError> for DeserializeError {
    fn from(e: num::ParseFloatError) -> Self {
        DeserializeError::ParseFloatError(e)
    }
}

/// A deserialized for fixed width data. Reads from the given bytes using the provided field
/// definitions to determine how many bytes to read for each deserialized value.
pub struct Deserializer<'r> {
    fields: iter::Peekable<vec::IntoIter<Field>>,
    input: &'r [u8],
}

impl<'r, 'de> Deserializer<'r> {
    /// Creates a new Deserializer from the given bytes and field definitions.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use serde;
    /// use fixed_width::{Deserializer, Field};
    /// use serde::Deserialize;
    /// use std::collections::HashMap;
    ///
    /// fn main() {
    ///     let input = b"1234abcd99";
    ///     let fields = vec![
    ///         Field::default().range(0..4).name(Some("numbers")),
    ///         Field::default().range(4..8).name(Some("letters")),
    ///         Field::default().range(8..10),
    ///     ];
    ///
    ///     let mut de = Deserializer::new(input, fields);
    ///     let h: HashMap<String, String> = HashMap::deserialize(&mut de).unwrap();
    ///
    ///     assert_eq!(h.get("numbers").unwrap(), "1234");
    ///     assert_eq!(h.get("letters").unwrap(), "abcd");
    ///     // If no name is supplied, the byte range is used as the key instead.
    ///     assert_eq!(h.get("8..10").unwrap(), "99");
    /// }
    /// ```
    pub fn new(input: &'r [u8], fields: Vec<Field>) -> Self {
        Self {
            fields: fields.into_iter().peekable(),
            input,
        }
    }

    /// Gets a reference to the underlying input bytes.
    ///
    /// ### Example
    ///
    /// ```rust
    /// use fixed_width::{Deserializer, Field, Reader};
    ///
    /// let fields = vec![Field::default().range(0..3)];
    /// let de = Deserializer::new(b"foobar", fields);
    ///
    /// assert_eq!(de.get_ref(), b"foobar");
    /// ```
    pub fn get_ref(&self) -> &[u8] {
        self.input
    }

    fn peek_field(&mut self) -> Option<&Field> {
        self.fields.peek()
    }

    fn skip_field(&mut self) {
        self.fields.next();
    }

    fn peek_bytes(&mut self) -> Result<&'r [u8], DeserializeError> {
        let field = match self.fields.peek() {
            Some(field) => field,
            None => return Err(DeserializeError::UnexpectedEndOfRecord),
        };

        match self.input.get(field.range.clone()) {
            Some(ref bytes) => Ok(bytes),
            None => Err(DeserializeError::UnexpectedEndOfRecord),
        }
    }

    fn next_bytes(&mut self) -> Result<&'r [u8], DeserializeError> {
        let field = match self.fields.next() {
            Some(field) => field,
            None => return Err(DeserializeError::UnexpectedEndOfRecord),
        };

        match self.input.get(field.range.clone()) {
            Some(ref bytes) => Ok(bytes),
            None => Err(DeserializeError::UnexpectedEndOfRecord),
        }
    }

    fn peek_str(&mut self) -> Result<&'r str, DeserializeError> {
        Ok(str::from_utf8(self.peek_bytes()?)?.trim())
    }

    fn next_str(&mut self) -> Result<&'r str, DeserializeError> {
        Ok(str::from_utf8(self.next_bytes()?)?.trim())
    }

    fn done(&mut self) -> bool {
        self.fields.peek().is_none()
    }
}

macro_rules! deserialize_int {
    ($de_fn:ident, $visit_fn:ident) => {
        fn $de_fn<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            let i = self.next_str()?
                .parse()
                .map_err(DeserializeError::ParseIntError)?;

            visitor.$visit_fn(i)
        }
    }
}

impl<'a, 'de: 'a> serde::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = DeserializeError;

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let s = self.next_str()?;
        if s.len() > 1 {
            Err(DeserializeError::Message(format!(
                "expected bool field to be 1 byte, got {}",
                s.len()
            )))
        } else {
            let c = s.chars().next().unwrap_or('0');
            if c == '0' {
                visitor.visit_bool(false)
            } else {
                visitor.visit_bool(true)
            }
        }
    }

    deserialize_int!(deserialize_i8, visit_i8);
    deserialize_int!(deserialize_i16, visit_i16);
    deserialize_int!(deserialize_i32, visit_i32);
    deserialize_int!(deserialize_i64, visit_i64);
    deserialize_int!(deserialize_u8, visit_u8);
    deserialize_int!(deserialize_u16, visit_u16);
    deserialize_int!(deserialize_u32, visit_u32);
    deserialize_int!(deserialize_u64, visit_u64);

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let f = self
            .next_str()?
            .parse()
            .map_err(DeserializeError::ParseFloatError)?;

        visitor.visit_f32(f)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let f = self
            .next_str()?
            .parse()
            .map_err(DeserializeError::ParseFloatError)?;

        visitor.visit_f64(f)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.next_str().and_then(|s| visitor.visit_borrowed_str(s))
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.next_str().and_then(|s| visitor.visit_borrowed_str(&s))
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let s = self.next_str()?;
        if s.len() > 1 {
            Err(DeserializeError::Message(format!(
                "expected bool field to be 1 byte, got {}",
                s.len()
            )))
        } else {
            let c = s.chars().next().unwrap_or(' ');
            visitor.visit_char(c)
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.next_bytes()
            .and_then(|b| visitor.visit_borrowed_bytes(b))
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.next_bytes()
            .and_then(|b| visitor.visit_byte_buf(b.to_vec()))
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.peek_str()?.is_empty() {
            self.skip_field();
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.skip_field();
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.skip_field();
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(self)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_map(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_enum(self)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_any(visitor)
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let s = self.next_str()?;

        if s.len() == 1 {
            if s == "1" {
                visitor.visit_bool(true)
            } else if s == "0" {
                visitor.visit_bool(false)
            } else {
                let c = s.chars().next().unwrap_or(' ');
                visitor.visit_char(c)
            }
        } else if let Ok(n) = s.parse::<i64>() {
            visitor.visit_i64(n)
        } else if let Ok(n) = s.parse::<f64>() {
            visitor.visit_f64(n)
        } else {
            visitor.visit_str(&s)
        }
    }
}

impl<'a, 'de: 'a> de::SeqAccess<'de> for &'a mut Deserializer<'de> {
    type Error = DeserializeError;

    fn next_element_seed<S: de::DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, Self::Error> {
        if self.done() {
            Ok(None)
        } else {
            seed.deserialize(&mut **self).map(Some)
        }
    }
}

impl<'a, 'de: 'a> de::MapAccess<'de> for &'a mut Deserializer<'de> {
    type Error = DeserializeError;

    fn next_key_seed<S: de::DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, Self::Error> {
        if self.done() {
            Ok(None)
        } else {
            let name = match self.peek_field() {
                Some(f) => f
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("{}..{}", f.range.start, f.range.end)),
                None => return Err(DeserializeError::UnexpectedEndOfRecord),
            };
            seed.deserialize(name.into_deserializer()).map(Some)
        }
    }

    fn next_value_seed<S: de::DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<S::Value, Self::Error> {
        seed.deserialize(&mut **self)
    }
}

impl<'a, 'de: 'a> de::EnumAccess<'de> for &'a mut Deserializer<'de> {
    type Error = DeserializeError;
    type Variant = Self;

    fn variant_seed<S: de::DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> Result<(S::Value, Self::Variant), Self::Error> {
        let name = match self.peek_field() {
            Some(field) => match field.name {
                Some(ref name) => name.clone(),
                None => {
                    return Err(DeserializeError::Message(format!(
                        "no name for field with range {}..{}",
                        field.range.start, field.range.end
                    )))
                }
            },
            None => return Err(DeserializeError::UnexpectedEndOfRecord),
        };
        seed.deserialize(name.into_deserializer())
            .map(|v| (v, self))
    }
}

impl<'a, 'de: 'a> de::VariantAccess<'de> for &'a mut Deserializer<'de> {
    type Error = DeserializeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(
        self,
        _seed: T,
    ) -> Result<T::Value, Self::Error> {
        Err(DeserializeError::invalid_type(
            de::Unexpected::UnitVariant,
            &"newtype variant",
        ))
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(DeserializeError::invalid_type(
            de::Unexpected::UnitVariant,
            &"tuple variant",
        ))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(DeserializeError::invalid_type(
            de::Unexpected::UnitVariant,
            &"struct variant",
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Field, FixedWidth};
    use serde::Deserialize;
    use serde_bytes::ByteBuf;
    use serde_derive::Deserialize;
    use std::collections::HashMap;

    #[test]
    fn bool_de() {
        let fields = vec![Field::default().range(0..1)];
        let t: bool = from_bytes_with_fields(b"1", fields.clone()).unwrap();
        let f: bool = from_bytes_with_fields(b"0", fields.clone()).unwrap();

        assert!(t);
        assert!(!f);
    }

    #[test]
    fn int_de() {
        let fields = vec![Field::default().range(0..4)];

        let uint8: u8 = from_bytes_with_fields(b"0123", fields.clone()).unwrap();
        let iint8: i8 = from_bytes_with_fields(b"-123", fields.clone()).unwrap();
        assert_eq!(uint8, 123);
        assert_eq!(iint8, -123);

        let uint16: u16 = from_bytes_with_fields(b"0123", fields.clone()).unwrap();
        let iint16: i16 = from_bytes_with_fields(b"-123", fields.clone()).unwrap();
        assert_eq!(uint16, 123);
        assert_eq!(iint16, -123);

        let uint32: u32 = from_bytes_with_fields(b"0123", fields.clone()).unwrap();
        let iint32: i32 = from_bytes_with_fields(b"-123", fields.clone()).unwrap();
        assert_eq!(uint32, 123);
        assert_eq!(iint32, -123);

        let uint64: u64 = from_bytes_with_fields(b"0123", fields.clone()).unwrap();
        let iint64: i64 = from_bytes_with_fields(b"-123", fields.clone()).unwrap();
        assert_eq!(uint64, 123);
        assert_eq!(iint64, -123);
    }

    #[test]
    fn float_de() {
        let fields = vec![Field::default().range(0..6)];

        let pos_f32: f32 = from_bytes_with_fields(b"0123.1", fields.clone()).unwrap();
        let neg_f32: f32 = from_bytes_with_fields(b"-123.1", fields.clone()).unwrap();
        assert_eq!(pos_f32, 123.1);
        assert_eq!(neg_f32, -123.1);

        let pos_f64: f64 = from_bytes_with_fields(b"0123.1", fields.clone()).unwrap();
        let neg_f64: f64 = from_bytes_with_fields(b"-123.1", fields.clone()).unwrap();
        assert_eq!(pos_f64, 123.1);
        assert_eq!(neg_f64, -123.1);
    }

    #[test]
    fn str_de() {
        let fields = vec![Field::default().range(0..6)];
        let s: &str = from_bytes_with_fields(b"foobar", fields).unwrap();
        assert_eq!(s, "foobar");
    }

    #[test]
    fn string_de() {
        let fields = vec![Field::default().range(0..6)];
        let s: String = from_bytes_with_fields(b"foobar", fields).unwrap();
        assert_eq!(s, "foobar");
    }

    #[test]
    fn char_de() {
        let fields = vec![Field::default().range(0..1)];
        let s: char = from_bytes_with_fields(b"f", fields).unwrap();
        assert_eq!(s, 'f');
    }

    #[test]
    fn bytes_de() {
        let fields = vec![Field::default().range(0..6)];
        let s: Vec<u8> = from_bytes_with_fields::<ByteBuf>(b"foobar", fields)
            .unwrap()
            .into_vec();
        assert_eq!(s, b"foobar".to_vec());
    }

    #[test]
    fn byte_buf_de() {
        let fields = vec![Field::default().range(0..6)];
        let s: &[u8] = from_bytes_with_fields(b"foobar", fields).unwrap();
        assert_eq!(s, b"foobar");
    }

    #[test]
    fn option_de() {
        let fields = vec![Field::default().range(0..1)];
        let c: Option<char> = from_bytes_with_fields(b"c", fields).unwrap();
        assert_eq!(c, Some('c'));

        let fields = vec![Field::default().range(0..1)];
        let c: Option<char> = from_bytes_with_fields(b" ", fields).unwrap();
        assert_eq!(c, None);
    }

    #[test]
    fn unit_de() {
        let fields = vec![Field::default().range(0..1)];
        let u: () = from_bytes_with_fields(b"c", fields).unwrap();
        assert_eq!(u, ());
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct Unit;

    #[test]
    fn unit_struct_de() {
        let fields = vec![Field::default().range(0..3)];
        let unit: Unit = from_bytes_with_fields(b"123", fields).unwrap();
        assert_eq!(unit, Unit);
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct Newtype(usize);

    #[test]
    fn newtype_struct_de() {
        let fields = vec![Field::default().range(0..3)];
        let nt: Newtype = from_bytes_with_fields(b"123", fields).unwrap();
        assert_eq!(nt, Newtype(123));
    }

    #[test]
    fn seq_de() {
        let fields = vec![Field::default().range(0..3), Field::default().range(3..6)];
        let v: Vec<usize> = from_bytes_with_fields(b"111222", fields).unwrap();
        assert_eq!(v, vec![111, 222]);
    }

    #[derive(Debug, Deserialize)]
    struct Test1 {
        a: usize,
        b: String,
        c: f64,
        d: Option<usize>,
    }

    impl FixedWidth for Test1 {
        fn fields() -> Vec<Field> {
            vec![
                Field::default().range(0..3).name(Some("a")),
                Field::default().range(3..6).name(Some("b")),
                Field::default().range(6..10),
                Field::default().range(10..13).name(Some("d")),
            ]
        }
    }

    #[test]
    fn struct_de() {
        let input = b"123abc9876 12";
        let test: Test1 = from_bytes(input).unwrap();

        assert_eq!(test.a, 123);
        assert_eq!(test.b, "abc");
        assert_eq!(test.c, 9876.0);
        assert_eq!(test.d, Some(12));
    }

    #[test]
    fn tuple_de() {
        let fields = vec![Field::default().range(0..3), Field::default().range(3..6)];
        let t: (usize, usize) = from_bytes_with_fields(b"111222", fields).unwrap();
        assert_eq!(t, (111, 222));
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct Tuple(usize, usize);

    #[test]
    fn tuple_struct_de() {
        let fields = vec![Field::default().range(0..3), Field::default().range(3..6)];
        let t: Tuple = from_bytes_with_fields(b"111222", fields).unwrap();
        assert_eq!(t, Tuple(111, 222));
    }

    #[test]
    fn hashmap_de() {
        let input = b"123abc9876 12";
        let mut de = Deserializer::new(input, Test1::fields());

        let test: HashMap<String, String> = HashMap::deserialize(&mut de).unwrap();

        assert_eq!(test.get("a").unwrap(), "123");
        assert_eq!(test.get("b").unwrap(), "abc");
        assert_eq!(test.get("6..10").unwrap(), "9876");
        assert_eq!(test.get("d").unwrap(), "12");
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(untagged)]
    enum UntaggedEnum {
        Int(usize),
    }

    #[test]
    fn untagged_enum_de() {
        let fields = vec![Field::default().range(0..3).name(Some("Int"))];
        let e: UntaggedEnum = from_bytes_with_fields(b"111", fields).unwrap();
        assert_eq!(e, UntaggedEnum::Int(111));
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct TaggedEnum {
        a: UntaggedEnum,
    }

    #[test]
    fn tagged_enum_de() {
        let fields = vec![Field::default().range(0..3).name(Some("a"))];
        let e: TaggedEnum = from_bytes_with_fields(b"111", fields).unwrap();
        assert_eq!(
            e,
            TaggedEnum {
                a: UntaggedEnum::Int(111)
            }
        );
    }

    #[test]
    fn from_str_de() {
        let s = "123abc9876 12";
        let test: Test1 = from_str(s).unwrap();

        assert_eq!(test.a, 123);
        assert_eq!(test.b, "abc");
        assert_eq!(test.c, 9876.0);
        assert_eq!(test.d, Some(12));
    }

    #[test]
    fn from_bytes_de() {
        let b = b"123abc9876 12";
        let test: Test1 = from_bytes(b).unwrap();

        assert_eq!(test.a, 123);
        assert_eq!(test.b, "abc");
        assert_eq!(test.c, 9876.0);
        assert_eq!(test.d, Some(12));
    }

    #[test]
    fn test_from_str_with_fields() {
        let fields = vec![Field::default().range(0..3).name(Some("a"))];
        let e: TaggedEnum = from_str_with_fields("111", fields).unwrap();
        assert_eq!(
            e,
            TaggedEnum {
                a: UntaggedEnum::Int(111)
            }
        );
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct TestChar {
        a: char,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct TestBool {
        a: bool,
    }

    #[test]
    fn test_does_not_panic_for_empty_char() {
        let fields = vec![Field::default().range(0..1)];
        let tc: TestChar = from_bytes_with_fields(b"  ", fields).unwrap();

        assert_eq!(tc.a, ' ');
    }

    #[test]
    fn test_does_not_panic_for_empty_bool() {
        let fields = vec![Field::default().range(0..1)];
        let tc: TestBool = from_bytes_with_fields(b"  ", fields).unwrap();

        assert_eq!(tc.a, false);
    }
}

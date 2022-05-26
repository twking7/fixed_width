use fixed_width::{DeserializeError, Deserializer, FixedWidth, Reader, Serializer};
use fixed_width_derive::FixedWidth;
use serde::{Deserialize, Serialize};
use serde_derive::{Deserialize, Serialize};
use std::result;

#[derive(FixedWidth, Serialize, Deserialize)]
struct Stuff {
    #[fixed_width(range = "0..6")]
    pub stuff1: String,
    #[fixed_width(range = "6..12", pad_with = "0")]
    pub stuff2: String,
    #[fixed_width(range = "12..15", pad_with = "0")]
    pub stuff3: usize,
    #[fixed_width(range = "15..19")]
    pub stuff4: usize,
    #[fixed_width(range = "21..27")]
    pub stuff5: String,
    #[fixed_width(range = "27..31", justify = "right")]
    pub stuff6: String,
}

#[derive(FixedWidth, Serialize, Deserialize)]
struct Optionals {
    #[fixed_width(range = "0..4")]
    pub stuff1: Option<String>,
    #[fixed_width(range = "4..10")]
    pub stuff2: Option<String>,
    #[fixed_width(range = "10..15")]
    pub stuff3: Option<usize>,
}

#[derive(FixedWidth, Deserialize)]
#[allow(dead_code)]
struct Record1 {
    #[fixed_width(range = "0..1")]
    pub record_type: usize,
    #[fixed_width(range = "1..5")]
    pub state: String,
}

#[derive(FixedWidth, Deserialize)]
#[allow(dead_code)]
struct Record2 {
    #[fixed_width(range = "0..1")]
    pub record_type: usize,
    #[fixed_width(range = "1..5")]
    pub name: String,
}

#[derive(FixedWidth, Deserialize)]
#[allow(dead_code)]
struct SkippedStuff {
    #[fixed_width(range = "0..6")]
    pub stuff1: String,
    #[fixed_width(range = "6..12", pad_with = "0")]
    pub stuff2: String,
    #[fixed_width(range = "12..15", pad_with = "0")]
    pub stuff3: usize,
    #[serde(skip)]
    pub skipped: i64,
    #[fixed_width(range = "15..19")]
    pub stuff4: usize,
    #[fixed_width(range = "21..27")]
    pub stuff5: String,
    #[fixed_width(range = "27..31", justify = "right")]
    pub stuff6: String,
}

fn field_def_fields() -> fixed_width::FieldSet {
    fixed_width::FieldSet::Seq(vec![
        fixed_width::FieldSet::new_field(0..3),
        fixed_width::FieldSet::new_field(3..9),
    ])
}

#[derive(FixedWidth, Deserialize)]
#[fixed_width(field_def = "field_def_fields")]
#[allow(dead_code)]
struct ByFieldDef {
    pub id: usize,
    pub name: String,
}

#[test]
fn test_serialize() {
    let stuff = Stuff {
        stuff1: "foo".to_string(),
        stuff2: "bar".to_string(),
        stuff3: 234,
        stuff4: 9,
        stuff5: "foobar".to_string(),
        stuff6: "123".to_string(),
    };

    let mut w = fixed_width::Writer::from_memory();
    {
        let mut ser = Serializer::new(&mut w, Stuff::fields());
        stuff.serialize(&mut ser).unwrap();
    }

    assert_eq!("foo   bar0002349   foobar 123", Into::<String>::into(w));
}

#[test]
fn test_deserialize() {
    let fr = "   foo000bar234   9  foobar123 ".as_bytes();
    let mut de = Deserializer::new(fr, Stuff::fields());
    let stuff = Stuff::deserialize(&mut de).unwrap();

    assert_eq!(stuff.stuff1, "foo");
    assert_eq!(stuff.stuff2, "000bar");
    assert_eq!(stuff.stuff3, 234);
    assert_eq!(stuff.stuff4, 9);
    assert_eq!(stuff.stuff5, "foobar");
    assert_eq!(stuff.stuff6, "123");
}

#[test]
fn test_deserialize_multiple() {
    let fr = "   foo000bar234   9  foobar321    foo000bar234   9  foobar123 ".as_bytes();

    let mut rdr = Reader::from_bytes(fr).width(31);

    for record in rdr.byte_reader().filter_map(result::Result::ok) {
        let stuff: Stuff = fixed_width::from_bytes(&record).unwrap();
        assert_eq!(stuff.stuff1, "foo");
        assert_eq!(stuff.stuff2, "000bar");
    }
}

#[test]
fn test_from_fixed_record_when_input_is_too_small() {
    let fr = "   foo000bar234   9".as_bytes();
    let mut de = Deserializer::new(fr, Stuff::fields());
    let err = Stuff::deserialize(&mut de);

    match err {
        Ok(_) => assert!(false, "expected Err, got Ok"),
        Err(DeserializeError::UnexpectedEndOfRecord) => assert!(true),
        Err(e) => assert!(false, "expected InvalidRecordError, got {}", e),
    }
}

#[test]
fn test_serialize_optionals() {
    let optionals = Optionals {
        stuff1: None,
        stuff2: Some("foo".to_string()),
        stuff3: Some(23),
    };

    let mut w = fixed_width::Writer::from_memory();
    {
        let mut ser = Serializer::new(&mut w, Optionals::fields());
        optionals.serialize(&mut ser).unwrap();
    }

    assert_eq!("    foo   23   ", Into::<String>::into(w));
}

#[test]
fn test_deserialize_optionals() {
    let fr = "    foo   23   ".as_bytes();
    let mut de = Deserializer::new(fr, Optionals::fields());
    let optionals = Optionals::deserialize(&mut de).unwrap();

    assert_eq!(optionals.stuff1, None);
    assert_eq!(optionals.stuff2, Some("foo".to_string()));
    assert_eq!(optionals.stuff3, Some(23));
}

#[test]
fn test_multiple_record_types() {
    let data = "0OHIO1 BOB";

    let mut reader = Reader::from_string(data).width(5);
    let mut rec1 = false;
    let mut rec2 = false;

    while let Some(Ok(bytes)) = reader.next_record() {
        match bytes.get(0) {
            Some(b'0') => {
                let Record1 { state, .. } = fixed_width::from_bytes(bytes).unwrap();
                rec1 = true;
                assert_eq!(state, "OHIO");
            }
            Some(b'1') => {
                let Record2 { name, .. } = fixed_width::from_bytes(bytes).unwrap();
                rec2 = true;
                assert_eq!(name, "BOB");
            }
            Some(_) => assert!(false, "unexpected record type"),
            None => assert!(false, "unexpected None"),
        }
    }

    assert!(rec1 && rec2);
}

#[test]
fn test_deserialize_with_skipped_fields() {
    let fr = "   foo000bar234   9  foobar123 ".as_bytes();
    let mut de = Deserializer::new(fr, SkippedStuff::fields());
    let stuff = SkippedStuff::deserialize(&mut de).unwrap();

    assert_eq!(stuff.stuff1, "foo");
    assert_eq!(stuff.stuff2, "000bar");
    assert_eq!(stuff.stuff3, 234);
    assert_eq!(stuff.stuff4, 9);
    assert_eq!(stuff.stuff5, "foobar");
    assert_eq!(stuff.stuff6, "123");
}

#[test]
fn test_specify_fields_by_field_def() {
    let record = "999foobar";
    let data: ByFieldDef = fixed_width::from_str(record).unwrap();

    assert_eq!(data.id, 999);
    assert_eq!(data.name, "foobar");
}

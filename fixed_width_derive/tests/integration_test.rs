#[macro_use]
extern crate fixed_width_derive;
#[macro_use]
extern crate serde_derive;
extern crate fixed_width;
extern crate serde;

use fixed_width::{DeserializeError, Deserializer, FixedWidth, Serializer};
use serde::{Deserialize, Serialize};

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
    #[fixed_width(range = "21..27", default = "foobar")]
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

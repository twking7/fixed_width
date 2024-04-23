use fixed_width::{LineBreak, Reader, Writer};
use std::{
    fs::{self, File},
    io::Write,
    result,
};

#[test]
fn read_from_file() {
    let mut rdr = Reader::from_file("./tests/data/sample_file.txt")
        .unwrap()
        .width(16);

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
fn read_from_file_with_newlines() {
    let mut rdr = Reader::from_file("./tests/data/sample_file_newlines.txt")
        .unwrap()
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
fn write_to_file() {
    let path = "./tests/data/sample_write.txt";
    let records = [
        "1111222233334444".to_string(),
        "1111222233334444".to_string(),
        "1111222233334444".to_string(),
    ];

    let f = File::create(path).unwrap();

    let mut wrtr = Writer::from_writer(f);
    wrtr.write_iter(records.iter()).unwrap();
    wrtr.flush().unwrap();

    let expected = "111122223333444411112222333344441111222233334444";
    let s = fs::read_to_string(path).unwrap();

    fs::remove_file(path).unwrap();
    assert_eq!(expected, s);
}

#[test]
fn write_to_file_with_newlines() {
    let path = "./tests/data/sample_write_newlines.txt";
    let records = [
        "1111222233334444".to_string(),
        "1111222233334444".to_string(),
        "1111222233334444".to_string(),
    ];

    let f = File::create(path).unwrap();

    let mut wrtr = Writer::from_writer(f).linebreak(LineBreak::Newline);
    wrtr.write_iter(records.iter()).unwrap();
    wrtr.flush().unwrap();

    let expected = "1111222233334444\n1111222233334444\n1111222233334444";
    let s = fs::read_to_string(path).unwrap();

    fs::remove_file(path).unwrap();
    assert_eq!(expected, s);
}

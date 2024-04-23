# Fixed Width &emsp; ![Build Status] [![Latest Version]][crates.io] [![Documents]][docs.rs]

[Build Status]: https://github.com/twking7/fixed_width/actions/workflows/rust.yml/badge.svg
[Latest Version]: https://img.shields.io/badge/crates.io-0.6.0-blue.svg
[crates.io]: https://crates.io/crates/fixed_width
[Documents]: https://img.shields.io/docsrs/fixed_width/latest
[docs.rs]: https://docs.rs/fixed_width

The `fixed_width` crate is designed to facilitate easy reading and writing of fixed width files
with [serde](https://serde.rs/) support.

Fixed width in this case means that each line of the file is the same number of bytes.
It supports unicode, but only if the unicode fits in the designated byte size of the record.
It also provides a few useful abstractions to ease serializing and deserializing data into and out
of fixed width files.

## Usage

Add as a dependency:

```toml
[dependencies]
fixed_width = "0.6"

# Optionally, if you are running Rust version 1.30.0 or above and want to derive fixed width field definitions:
fixed_width_derive = "0.6"
```

in the root of your crate:

```rust
use fixed_width;

// and if you are using fixed_width_derive:
use fixed_width_derive::FixedWidth;
```

## Example

Read fixed width data from a `&str`:

```rust
use fixed_width::{FixedWidth, Reader};

let data = "1234554321";
let mut reader = Reader::from_string(data).width(5);

for record in reader.string_reader().filter_map(std::result::Result::ok) {
    // Prints "12345" and then "54321"
    println!("{}", record);
}
```

Read data from a `&str` using serde, using `fixed_width_derive`:

```rust
use fixed_width::{FixedWidth, Reader};
use fixed_width_derive::FixedWidth;
use serde_derive::Deserialize;

// It is not necessary to use `fixed_width_derive`, you can manually implement the `FixedWidth` trait.
#[derive(FixedWidth, Deserialize)]
struct Record {
    #[fixed_width(range = "0..5")]
    pub n: usize,
}

let data = "1234554321";

let mut reader = Reader::from_string(data).width(5);

for bytes in reader.byte_reader().filter_map(std::result::Result::ok) {
    // You must specify the type for deserialization
    let record: Record = fixed_width::from_bytes(&bytes).unwrap();

    // Prints "12345" and then "54321"
    println!("{}", record.n);
}
```

Read data where there are different record types in the file:

```rust
use fixed_width::{FixedWidth, Reader};
use fixed_width_derive::FixedWidth;
use serde_derive::Deserialize;

#[derive(FixedWidth, Deserialize)]
struct Record1 {
    #[fixed_width(range = "0..1")]
    pub record_type: usize,
    #[fixed_width(range = "1..5")]
    pub state: String,
}

#[derive(FixedWidth, Deserialize)]
struct Record2 {
    #[fixed_width(range = "0..1")]
    pub record_type: usize,
    #[fixed_width(range = "1..5")]
    pub name: String,
}

let data = "0OHIO1 BOB";

let mut reader = Reader::from_string(data).width(5);

while let Some(Ok(bytes)) = reader.next_record() {
    match bytes.get(0) {
        Some(b'0') => {
            let Record1 { state, .. } = fixed_width::from_bytes(bytes).unwrap();
            assert_eq!(state, "OHIO");
        }
        Some(b'1') => {
            let Record2 { name, .. } = fixed_width::from_bytes(bytes).unwrap();
            assert_eq!(name, "BOB");
        }
        Some(_) => {}
        None => {}
    }
}
```

Read data from a file:

```rust
use fixed_width::Reader;
use std::fs;

// from a file path on disk
let filepath = "/path/to/file.txt";
let mut reader = Reader::from_file(filepath)?.width(5);

// from a file handle
let file = fs::File::open(filepath)?;
let mut reader = Reader::from_reader(file).width(5);
```

## License

Licensed under MIT.

## Contributing

Issues and pull requests are welcome, please add tests and documentation (if necessary) for any changes you submit.

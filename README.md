# Fixed Width &emsp; [![Build Status]][travis] [![Latest Version]][crates.io]

[Build Status]: https://travis-ci.org/twking7/fixed_width.svg?branch=master
[travis]: https://travis-ci.org/twking7/fixed_width
[Latest Version]: https://img.shields.io/badge/crates.io-0.4.0-blue.svg
[crates.io]: https://docs.rs/fixed_width

A fixed width data reader and writer library. Also supports [Serde](https://github.com/serde-rs/serde).

Fixed width in this case means that each line of the file is the same number of bytes. It supports unicode, but only if the unicode fits in the designated byte size of the record.

## Documentation

https://docs.rs/fixed_width

## Usage

Add as a dependency:

```toml
[dependencies]
fixed_width = "0.4"

# Optionally, if you are running Rust version 1.30.0 or above and want to derive fixed width field definitions:
fixed_width_derive = "0.4"
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
use fixed_width::{FixedWidth, Field, Reader};
use std::result;

fn main() {
  let data = "1234554321";
  let mut reader = Reader::from_string(data).width(5);

  for record in reader.string_reader().filter_map(result::Result::ok) {
    // Prints "12345" and then "54321"
    println!("{}", record);
  }
}
```

Read data from a `&str` using serde, using `fixed_width_derive`:

```rust
use serde::Deserialize;
use serde_derive::Deserialize;
use fixed_width::{FixedWidth, Field, Reader};
use fixed_width_derive::FixedWidth;
use std::result;

// It is not necessary to use `fixed_width_derive`, you can manually implement the `FixedWidth` trait.
#[derive(FixedWidth, Deserialize)]
struct Record {
  #[fixed_width(range = "0..5")]
  pub n: usize,
}

fn main() {
  let data = "1234554321";

  let mut reader = Reader::from_string(data).width(5);

  for bytes in reader.byte_reader().filter_map(result::Result::ok) {
    // You must specify the type for deserialization
    let record: Record = fixed_width::from_bytes(&bytes).unwrap();

    // Prints "12345" and then "54321"
    println!("{}", record.n);
  }
}
```

Read data where there are different record types in the file:

```rust
use serde::Deserialize;
use serde_derive::Deserialize;

use fixed_width::{FixedWidth, Field, Reader, from_bytes};
use fixed_width_derive::FixedWidth;
use std::result;

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

fn main() {
  let data = "0OHIO1 BOB";

  let mut reader = Reader::from_string(data).width(5);

  while let Some(Ok(bytes)) = reader.next_record() {
    match bytes.get(0) {
      Some(b'0') => {
        let Record1 { state, .. } = from_bytes(bytes).unwrap();
        assert_eq!(state, "OHIO");
      },
      Some(b'1') => {
        let Record2 { name, .. } = from_bytes(bytes).unwrap();
        assert_eq!(name, "BOB");
      },
      Some(_) => {},
      None => {},
    }
  }
}

```

## License

Licensed under MIT.

## Contributing

Issues and pull requests are welcome, please add tests and documentation (if necessary) for any changes you submit.

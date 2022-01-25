# linkstore

linkstore is a library that allows you to define global variables in your final compiled binary that can be modified post-compilation.

linkstore currently supports ELF and PE executable formats and can be used with both statically and dynamically linked libraries.

# Supported types

Currently, linkstore can serialize and deserialize numbers (excluding `usize` and `isize`), `bool` and fixed-length arrays out of the box.

For anything else, you'll need to implement your own deserialization from fixed-length byte arrays.

# Usage

## Defining & using linkstore globals

```rust
#[macro_use] extern crate linkstore;

linkstore! {
    pub static LINKSTORE_TEST: u64 = 0xDEADBEEF;
    pub static LINKSTORE_YEAH: u32 = 0xDEADBEEF;
    pub static LINKSTORE_BYTES: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
    pub static LINKSTORE_SHORTS: [u16; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
    pub static LINKSTORE_BIG: u128 = 0xDEADBEEF;
}

fn main() {
    unsafe {
        println!("LINKSTORE_TEST = {:x}", LINKSTORE_TEST::get());
        println!("LINKSTORE_YEAH = {:x}", LINKSTORE_YEAH::get());
        println!("LINKSTORE_BYTES = {:?}", LINKSTORE_BYTES::get());
        println!("LINKSTORE_SHORTS = {:?}", LINKSTORE_SHORTS::get());
        println!("LINKSTORE_BIG = {:b}", LINKSTORE_BIG::get());
    }
}
```

## Manipulating linkstore globals after compilation

Once your binary has been built, you can use linkstore to modify the values.

```rust
fn main() {
    // You can use `linkstore::open_binary` to open a binary file from the filesystem.
    let mut binary: std::fs::File = linkstore::open_binary("C:\\Windows\\system32\\kernel32.dll").unwrap();

    // Alternatively, you can work directly on a memory buffer or memory-mapped file using a `std::io::Cursor`
    let mut binary: Vec<u8> = std::fs::read("C:\\Windows\\system32\\kernel32.dll").unwrap();
    let mut binary: std::io::Cursor<&mut [u8]> = std::io::Cursor::new(&mut binary);

    let mut embedder = linkstore::Embedder::new(&mut binary).unwrap();

    embedder.embed("LINKSTORE_TEST", &69_u64).unwrap();
    embedder.embed("LINKSTORE_YEAH", &420_u32).unwrap();
    embedder.embed("LINKSTORE_BYTES", &[1_u8, 2, 3, 4]).unwrap();
    embedder.embed("LINKSTORE_SHORTS", &[1_u16, 2, 3, 4]).unwrap();
    embedder.embed("LINKSTORE_BIG", &(u128::MAX / 2)).unwrap();

    embedder.finish().unwrap();
}
```

# TODO

* MacOS binaries support
* MacO + fat binaries support
* When specialization is stabilized, implement a ton of specialization and potentially extra serialization/deserialization support
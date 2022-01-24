# linkstore

linkstore is a library that allows you to define global variables in your final compiled binary that can be modified post-compilation.

linkstore currently supports ELF and PE executable formats and can be used with both statically and dynamically linked libraries.

# Usage

## Defining linkstore globals

First, you must define the globals you want to linkstore.

Only simple, memory-contigious, "plain old data" types can be serialized and deserialized by default. You can try and define your own, but remember that you cannot make any heap allocations or use pointers. Normal Rust `const` and `static` rules apply.

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
    println!("LINKSTORE_TEST = {:x}", LINKSTORE_TEST::get());
    println!("LINKSTORE_YEAH = {:x}", LINKSTORE_YEAH::get());
    println!("LINKSTORE_BYTES = {:?}", LINKSTORE_BYTES::get());
    println!("LINKSTORE_SHORTS = {:?}", LINKSTORE_SHORTS::get());
    println!("LINKSTORE_BIG = {:b}", LINKSTORE_BIG::get());
}
```

## Manipulating linkstore globals after compilation

Once your binary has been built, you can use linkstore to modify the values.

```rust
fn main() {
    let mut binary = linkstore::open_binary("C:\\Windows\\system32\\kernel32.dll").unwrap();
    let mut embedder = Embedder::new(&mut binary).unwrap();

    embedder.embed("LINKSTORE_TEST", &69_u64).unwrap();
    embedder.embed("LINKSTORE_YEAH", &420_u32).unwrap();
    embedder.embed("LINKSTORE_BYTES", &[1_u8, 2, 3, 4]).unwrap();
    embedder.embed("LINKSTORE_SHORTS", &[1_u16, 2, 3, 4]).unwrap();
    embedder.embed("LINKSTORE_BIG", &(u128::MAX / 2)).unwrap();

    embedder.finish().unwrap();
}
```
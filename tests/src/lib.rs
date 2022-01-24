mod common;

#[repr(C)]
pub struct LinkStoreTest {
	a: u64,
	b: u32,
	c: [u8; 4],
	d: u64,
	e: u64
}

#[no_mangle]
pub extern "C" fn linkstore_test() -> LinkStoreTest {
	let (d, e) = {
		let bytes = common::LINKSTORE_BIG::get().to_le_bytes();
		let (d, e) = bytes.split_at(bytes.len() / 2);
		(
			u64::from_le_bytes(d.try_into().unwrap()),
			u64::from_le_bytes(e.try_into().unwrap())
		)
	};
	LinkStoreTest {
		a: *common::LINKSTORE_TEST::get(),
		b: *common::LINKSTORE_YEAH::get(),
		c: *common::LINKSTORE_BYTES::get(),
		d, e
	}
}
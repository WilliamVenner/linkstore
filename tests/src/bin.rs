mod common;

fn main() {
	unsafe {
		let a = *common::LINKSTORE_TEST::get();
		let b = *common::LINKSTORE_YEAH::get();
		let c = a.checked_add(b as u64).unwrap();
		assert_eq!(c, 69 + 420);
	}
	std::process::exit(123);
}

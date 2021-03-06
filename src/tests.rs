use crate::*;
use std::process::Command;

#[repr(C)]
#[derive(PartialEq, Eq, Debug)]
pub struct LinkStoreTest {
	a: u64,
	b: u32,
	c: [u8; 4],
	d: u64,
	e: u64,
}
impl LinkStoreTest {
	fn test() -> Self {
		let (d, e) = {
			let bytes = (u128::MAX / 2).to_le_bytes();
			let (d, e) = bytes.split_at(bytes.len() / 2);
			(u64::from_le_bytes(d.try_into().unwrap()), u64::from_le_bytes(e.try_into().unwrap()))
		};
		Self {
			a: 69,
			b: 420,
			c: [1, 2, 3, 4],
			d,
			e,
		}
	}
}

fn build(target: &str) {
	assert!(Command::new("cargo")
		.args(&[
			"clean",
			"--manifest-path",
			"tests/Cargo.toml",
			"--target-dir",
			"tests/target",
			"--target",
			target
		])
		.status()
		.unwrap()
		.success());
	assert!(Command::new("cargo")
		.args(&[
			"build",
			"--profile",
			"linkstore-test-release",
			"--examples",
			"--target",
			target,
			"--manifest-path",
			"tests/Cargo.toml",
			"--target-dir",
			"tests/target"
		])
		.status()
		.unwrap()
		.success());
}

unsafe fn first_pass<'a, IO: BinaryHandle<'a>>(embedder: &mut Embedder<'a, IO>) {
	assert_eq!(embedder.read::<u64>("LINKSTORE_TEST").unwrap().next(), Some(0xDEADBEEF_u64));
	assert_eq!(embedder.read::<u32>("LINKSTORE_YEAH").unwrap().next(), Some(0xDEADBEEF_u32));
	assert!(matches!(
		embedder.try_read::<[u8; 4]>("LINKSTORE_BYTES").unwrap().next(),
		Some(Ok([0xDE, 0xAD, 0xBE, 0xEF]))
	));
	assert!(matches!(
		embedder.try_read::<[u16; 4]>("LINKSTORE_SHORTS").unwrap().next(),
		Some(Ok([0xDE, 0xAD, 0xBE, 0xEF]))
	));
	assert_eq!(embedder.read::<u128>("LINKSTORE_BIG").unwrap().next(), Some(0xDEADBEEF_u128));
	embedder.embed("LINKSTORE_TEST", &69_u64).unwrap();
	embedder.embed("LINKSTORE_YEAH", &420_u32).unwrap();
	embedder.embed("LINKSTORE_BYTES", &[1_u8, 2, 3, 4]).unwrap();
	embedder.embed("LINKSTORE_SHORTS", &[1_u16, 2, 3, 4]).unwrap();
	embedder.embed("LINKSTORE_BIG", &(u128::MAX / 2)).unwrap();
}

unsafe fn second_pass<'a, IO: BinaryHandle<'a>>(embedder: &mut Embedder<'a, IO>) {
	assert_eq!(embedder.read::<u64>("LINKSTORE_TEST").unwrap().next(), Some(69_u64));
	assert_eq!(embedder.read::<u32>("LINKSTORE_YEAH").unwrap().next(), Some(420_u32));
	assert!(matches!(
		embedder.try_read::<[u8; 4]>("LINKSTORE_BYTES").unwrap().next(),
		Some(Ok([1, 2, 3, 4]))
	));
	assert!(matches!(
		embedder.try_read::<[u16; 4]>("LINKSTORE_SHORTS").unwrap().next(),
		Some(Ok([1, 2, 3, 4]))
	));
	assert_eq!(embedder.read::<u128>("LINKSTORE_BIG").unwrap().next(), Some(u128::MAX / 2));
}

fn test_executable(path: &str, lib: bool, open: bool) {
	#[cfg(target_os = "linux")]
	assert!(Command::new("strip").arg(path).status().unwrap().success());

	{
		let mut binary = crate::open_binary(path).unwrap();
		let mut embedder = Embedder::new(&mut binary).unwrap();
		unsafe { first_pass(&mut embedder) };
		embedder.finish().unwrap();
	}

	{
		let mut binary = crate::open_binary(path).unwrap();
		let mut embedder = Embedder::new(&mut binary).unwrap();
		unsafe { second_pass(&mut embedder) };
	}

	if open {
		if lib {
			unsafe {
				let lib = libloading::Library::new(path).unwrap();
				let f: extern "C" fn() -> LinkStoreTest = *lib.get(b"linkstore_test\0").unwrap();
				assert_eq!(f(), LinkStoreTest::test());
			}
		} else {
			let output = Command::new(path).output().unwrap();
			if output.status.code() != Some(123) {
				panic!(
					"Code: {:?} != {:?}\n{}\n{}",
					output.status.code(),
					Some(123),
					String::from_utf8_lossy(&output.stdout),
					String::from_utf8_lossy(&output.stderr)
				);
			}
		}
	}
}

#[test]
#[cfg(target_os = "windows")]
fn linkstore() {
	{
		build("i686-pc-windows-msvc");
		test_executable(
			"tests/target/i686-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_bin.exe",
			false,
			true,
		);
		//test_executable("tests/target/i686-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_staticlib.lib", true, false);
		#[cfg(target_pointer_width = "32")]
		{
			test_executable(
				"tests/target/i686-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_dylib.dll",
				true,
				true,
			);
			test_executable(
				"tests/target/i686-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_cdylib.dll",
				true,
				true,
			);
		}
		#[cfg(not(target_pointer_width = "32"))]
		{
			test_executable(
				"tests/target/i686-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_dylib.dll",
				true,
				false,
			);
			test_executable(
				"tests/target/i686-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_cdylib.dll",
				true,
				false,
			);
		}
	}

	{
		build("x86_64-pc-windows-msvc");
		test_executable(
			"tests/target/x86_64-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_bin.exe",
			false,
			true,
		);
		//test_executable("tests/target/x86_64-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_staticlib.lib", true, false);
		#[cfg(target_pointer_width = "64")]
		{
			test_executable(
				"tests/target/x86_64-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_dylib.dll",
				true,
				true,
			);
			test_executable(
				"tests/target/x86_64-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_cdylib.dll",
				true,
				true,
			);
		}
		#[cfg(not(target_pointer_width = "64"))]
		{
			test_executable(
				"tests/target/x86_64-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_dylib.dll",
				true,
				false,
			);
			test_executable(
				"tests/target/x86_64-pc-windows-msvc/linkstore-test-release/examples/linkstore_tests_cdylib.dll",
				true,
				false,
			);
		}
	}
}

#[test]
#[cfg(target_os = "linux")]
fn linkstore() {
	{
		build("i686-unknown-linux-gnu");
		test_executable(
			"tests/target/i686-unknown-linux-gnu/linkstore-test-release/examples/linkstore_tests_bin",
			false,
			true,
		);
		test_executable(
			"tests/target/i686-unknown-linux-gnu/linkstore-test-release/examples/liblinkstore_tests_staticlib.a",
			true,
			false,
		);
		#[cfg(target_pointer_width = "32")]
		{
			test_executable(
				"tests/target/i686-unknown-linux-gnu/linkstore-test-release/examples/liblinkstore_tests_dylib.so",
				true,
				true,
			);
			test_executable(
				"tests/target/i686-unknown-linux-gnu/linkstore-test-release/examples/liblinkstore_tests_cdylib.so",
				true,
				true,
			);
		}
		#[cfg(not(target_pointer_width = "32"))]
		{
			test_executable(
				"tests/target/i686-unknown-linux-gnu/linkstore-test-release/examples/liblinkstore_tests_dylib.so",
				true,
				false,
			);
			test_executable(
				"tests/target/i686-unknown-linux-gnu/linkstore-test-release/examples/liblinkstore_tests_cdylib.so",
				true,
				false,
			);
		}
	}

	{
		build("x86_64-unknown-linux-gnu");
		test_executable(
			"tests/target/x86_64-unknown-linux-gnu/linkstore-test-release/examples/linkstore_tests_bin",
			false,
			true,
		);
		test_executable(
			"tests/target/x86_64-unknown-linux-gnu/linkstore-test-release/examples/liblinkstore_tests_staticlib.a",
			true,
			false,
		);
		#[cfg(target_pointer_width = "64")]
		{
			test_executable(
				"tests/target/x86_64-unknown-linux-gnu/linkstore-test-release/examples/liblinkstore_tests_dylib.so",
				true,
				true,
			);
			test_executable(
				"tests/target/x86_64-unknown-linux-gnu/linkstore-test-release/examples/liblinkstore_tests_cdylib.so",
				true,
				true,
			);
		}
		#[cfg(not(target_pointer_width = "64"))]
		{
			test_executable(
				"tests/target/x86_64-unknown-linux-gnu/linkstore-test-release/examples/liblinkstore_tests_dylib.so",
				true,
				false,
			);
			test_executable(
				"tests/target/x86_64-unknown-linux-gnu/linkstore-test-release/examples/liblinkstore_tests_cdylib.so",
				true,
				false,
			);
		}
	}
}

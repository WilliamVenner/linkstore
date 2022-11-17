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
	println!("Testing {path:?} (library: {lib})");

	#[cfg(unix)] {
		#[cfg(not(target_os = "macos"))]
		assert!(Command::new("strip").arg(path).status().unwrap().success());

		#[cfg(target_os = "macos")] {
			let mut strip = Command::new("strip");

			if lib {
				// default `strip` is too aggressive for dylibs on macOS
				strip.arg("-x");
			}

			assert!(strip.arg(path).status().unwrap().success());
		}
	}

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

	#[cfg(target_os = "macos")] {
		// We need to resign the binary to be able to run it
		assert!(Command::new("codesign")
			.args(&["--force", "--sign", "-", path])
			.status()
			.unwrap()
			.success());
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
					"Code: {:?} != {:?}{}\n{}\n{}",
					output.status.code(),
					Some(123),
					{
						#[cfg(unix)] {
							use std::os::unix::process::ExitStatusExt;
							format!("\nSignal: {:?}\nStopped Signal: {:?}", output.status.signal(), output.status.stopped_signal())
						}
						#[cfg(not(unix))] {
							""
						}
					},
					String::from_utf8_lossy(&output.stdout),
					String::from_utf8_lossy(&output.stderr)
				);
			}
		}
	}
}

macro_rules! generate_target_tests {
	{$({
		target_os = $target_os:literal,
		target_arch = $target_arch:literal,
		target_pointer_width = $target_pointer_width:literal,
		target_triple = $target_triple:literal,

		format_executable: $format_executable:literal,
		format_dylib: $format_dylib:literal,
		format_staticlib: $format_staticlib:literal,
	}),*} => {
		$(
			#[test]
			#[cfg(all(target_os = $target_os, target_arch = $target_arch, target_pointer_width = $target_pointer_width))]
			fn linkstore() {
				build($target_triple);
				test_executable(
					format!(concat!("tests/target/", $target_triple, "/linkstore-test-release/examples/", $format_executable), "linkstore_tests_bin").as_str(),
					false,
					true
				);
				test_executable(
					format!(concat!("tests/target/", $target_triple, "/linkstore-test-release/examples/", $format_staticlib), "linkstore_tests_staticlib").as_str(),
					true,
					false
				);
				test_executable(
					format!(concat!("tests/target/", $target_triple, "/linkstore-test-release/examples/", $format_dylib), "linkstore_tests_dylib").as_str(),
					true,
					true
				);
				test_executable(
					format!(concat!("tests/target/", $target_triple, "/linkstore-test-release/examples/", $format_dylib), "linkstore_tests_cdylib").as_str(),
					true,
					true
				);
			}
		)*
	};
}
generate_target_tests! {
	// x86_64
	{
		target_os = "windows",
		target_arch = "x86_64",
		target_pointer_width = "64",
		target_triple = "x86_64-pc-windows-msvc",

		format_executable: "{}.exe",
		format_dylib: "{}.dll",
		format_staticlib: "{}.lib",
	},
	{
		target_os = "linux",
		target_arch = "x86_64",
		target_pointer_width = "64",
		target_triple = "x86_64-unknown-linux-gnu",

		format_executable: "{}",
		format_dylib: "lib{}.so",
		format_staticlib: "lib{}.a",
	},
	{
		target_os = "macos",
		target_arch = "x86_64",
		target_pointer_width = "64",
		target_triple = "x86_64-apple-darwin",

		format_executable: "{}",
		format_dylib: "lib{}.dylib",
		format_staticlib: "lib{}.a",
	},

	// aarm64
	{
		target_os = "windows",
		target_arch = "aarch64",
		target_pointer_width = "64",
		target_triple = "aarch64-pc-windows-msvc",

		format_executable: "{}.exe",
		format_dylib: "{}.dll",
		format_staticlib: "{}.lib",
	},
	{
		target_os = "linux",
		target_arch = "aarch64",
		target_pointer_width = "64",
		target_triple = "aarch64-unknown-linux-gnu",

		format_executable: "{}",
		format_dylib: "lib{}.so",
		format_staticlib: "lib{}.a",
	},
	{
		target_os = "macos",
		target_arch = "aarch64",
		target_pointer_width = "64",
		target_triple = "aarch64-apple-darwin",

		format_executable: "{}",
		format_dylib: "lib{}.dylib",
		format_staticlib: "lib{}.a",
	}
}

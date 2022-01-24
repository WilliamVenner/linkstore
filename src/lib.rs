pub use goblin;

mod util;

mod embed;
pub use embed::Embedder;

pub mod read;
pub mod write;

pub const LINK_SECTION: &'static [u8] = b".lnkstre";

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("{0}")]
	Goblin(#[from] goblin::error::Error),

	#[error("Size of linkstore ({0} bytes) does not match size of value ({1} bytes)")]
	MismatchedSize(u64, usize),

	#[error("Linkstore contains no key with name {0}")]
	NotPresent(String),

	#[error("Binary does not contain a Linkstore section")]
	NoLinkstore,

	#[error("Unknown binary format")]
	Unrecognised,

	#[error("Format of .lnkstore section is corrupt or unsupported")]
	DecodingError,

	#[error("I/O error: {0}")]
	IoError(#[from] std::io::Error)
}

pub fn parse_binary<'a>(bytes: &'a [u8]) -> Result<goblin::Object<'a>, Error> {
	Ok(goblin::Object::parse(bytes)?)
}

pub fn open_binary<P: AsRef<std::path::Path>>(path: P) -> Result<std::fs::File, Error> {
	Ok(std::fs::OpenOptions::new()
		.truncate(false)
		.write(true)
		.read(true)
		.open(path)?)
}

#[macro_export]
macro_rules! linkstore {
	{$($vis:vis static $name:ident: $ty:ty = $init:expr;)+} => {$(
		#[allow(non_snake_case)]
		$vis mod $name {
			use core::mem::size_of;

			const NAME_LEN: usize = stringify!($name).len();

			#[repr(C)]
			struct AlignedLinkStore($ty);

			#[repr(C, packed)]
			struct LinkStoreContainer {
				name: [u8; NAME_LEN + 1],
				size: [u8; size_of::<usize>()],
				padding: [u8; size_of::<usize>()],
				value: AlignedLinkStore,
			}

			#[link_section = ".lnkstre"]
			#[used]
			static $name: LinkStoreContainer = LinkStoreContainer {
				name: {
					let mut static_bytes = [0u8; NAME_LEN + 1];
					let bytes = stringify!($name).as_bytes();
					let mut i = 0;
					while i < NAME_LEN {
						static_bytes[i] = bytes[i];
						i += 1;
					}
					static_bytes
				},

				size: (size_of::<$ty>()).to_le_bytes(),
				padding: (size_of::<LinkStoreContainer>() - (size_of::<usize>() * 2) - (NAME_LEN + 1) - size_of::<$ty>()).to_le_bytes(),
				value: AlignedLinkStore($init)
			};

			pub fn get() -> &'static $ty {
				&$name.value.0
			}
		}
	)+};
}

linkstore! {
	pub static LOLOLO: u64 = 0xDEADBEEF;
	pub static LOLOLO2: u64 = 0xDEADBEEF;
}

#[no_mangle]
pub extern "C" fn gmod13_open(_ptr: usize) -> i32 {
	println!("{}", LOLOLO::get());
	println!("{}", LOLOLO2::get());
	0
}

fn main() {
	println!("{}", LOLOLO::get());
	println!("{}", LOLOLO2::get());
}
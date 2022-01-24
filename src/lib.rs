#[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
compile_error!("Unsupported pointer width");

pub use goblin;

mod util;

mod embed;
pub use embed::Embedder;

mod read;
mod write;
pub use read::*;
pub use write::BinaryEmbeddable;

#[cfg(test)]
mod tests;

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

	#[error("Format of .lnkstore section is corrupt, unsupported or a bug may be present")]
	DecodingError,

	#[error("Format of .lnkstore section is corrupt, unsupported or a bug may be present")]
	NameDecodingError,

	#[error("Format of .lnkstore section is corrupt, unsupported or a bug may be present (expected {0}, got {1} for magic marker)")]
	MagicDecodingError(u8, u8),

	#[error("Format of .lnkstore section is corrupt, unsupported or a bug may be present (unexpected EOF)")]
	UnexpectedEof,

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

#[doc(hidden)]
pub const MAGIC: u8 = 234;

#[macro_export]
macro_rules! linkstore {
	{$($vis:vis static $name:ident: $ty:ty = $init:expr;)+} => {$(
		#[allow(non_snake_case)]
		$vis mod $name {
			use core::mem::{size_of, align_of};

			const NAME_LEN: usize = stringify!($name).len();

			#[repr(C)]
			struct LinkStoreContainer<T: $crate::BinaryEmbeddable> {
				name: [u8; NAME_LEN + 1 + 1],
				size: [u8; size_of::<usize>()],
				padding: [u8; size_of::<usize>()],
				value: $crate::VolatileWrapper<T>
			}

			#[link_section = ".lnkstre"]
			#[used]
			static $name: LinkStoreContainer<$ty> = LinkStoreContainer {
				name: {
					let mut static_bytes = [0u8; NAME_LEN + 1 + 1];
					static_bytes[0] = $crate::MAGIC;

					let bytes = stringify!($name).as_bytes();
					let mut i = 0;
					while i < NAME_LEN {
						static_bytes[i + 1] = bytes[i];
						i += 1;
					}

					static_bytes
				},

				size: size_of::<$ty>().to_le_bytes(),
				padding: (size_of::<LinkStoreContainer<$ty>>() - (NAME_LEN + 1 + 1) - (size_of::<usize>() * 2) - size_of::<$ty>()).to_le_bytes(),

				value: $crate::VolatileWrapper::new($init)
			};

			pub fn get() -> &'static $ty {
				debug_assert_eq!(std::mem::align_of::<LinkStoreContainer<$ty>>(), align_of::<$ty>(), "Alignment error");
				$name.value.get()
			}
		}
	)+};
}

#[doc(hidden)]
#[repr(transparent)]
pub struct VolatileWrapper<T>(core::cell::UnsafeCell<T>);
impl<T> VolatileWrapper<T> {
	#[doc(hidden)]
	pub const fn new(val: T) -> Self {
		Self(core::cell::UnsafeCell::new(val))
	}

	#[doc(hidden)]
	pub fn get(&'static self) -> &'static T {
		unsafe {
			std::mem::forget(std::ptr::read_volatile(self.0.get()));
			&*self.0.get()
		}
	}
}
unsafe impl<T> Sync for VolatileWrapper<T> {}
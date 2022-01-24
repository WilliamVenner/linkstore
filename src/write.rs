use std::borrow::Cow;

use crate::{read::{DecodeBinaryEmbeddable, TryDecodeBinaryEmbeddable}, embed::{EmbeddableBytes, EmbeddedBytes}};

#[derive(Debug)]
pub(crate) struct PendingEmbed<'a> {
	pub(crate) offset: u64,
	pub(crate) size: u64,
	pub(crate) little_endian: bool,
	pub(crate) bytes: EmbeddedBytes<'a>,
}

pub trait BinaryEmbeddable {
	fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes;
	fn as_be_bytes<'a>(&'a self) -> EmbeddableBytes {
		self.as_le_bytes()
	}
}

macro_rules! infallible_decode {
	(impl DecodeBinaryEmbeddable for $ty:ty {$($tt:tt)+}) => {
		impl DecodeBinaryEmbeddable for $ty {$($tt)+}
		impl TryDecodeBinaryEmbeddable for $ty {
			type Error = core::convert::Infallible;

			fn try_from_le_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
				Ok(<$ty as DecodeBinaryEmbeddable>::from_le_bytes(bytes))
			}
		}
	};
}

impl BinaryEmbeddable for bool {
	fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes {
		EmbeddableBytes::Small { size: 1, bytes: [*self as u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] }
	}
}
infallible_decode!(impl DecodeBinaryEmbeddable for bool {
	fn from_le_bytes(bytes: &[u8]) -> Self {
		bytes[0] != 0
	}
});

impl BinaryEmbeddable for Vec<u8> {
	fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes {
		EmbeddableBytes::Large(Cow::Borrowed(self))
	}
}
infallible_decode!(impl DecodeBinaryEmbeddable for Vec<u8> {
	fn from_le_bytes(bytes: &[u8]) -> Self {
		bytes.to_vec()
	}
});

impl BinaryEmbeddable for String {
	fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes {
		EmbeddableBytes::Large(Cow::Borrowed(self.as_bytes()))
	}
}
impl DecodeBinaryEmbeddable for String {
	fn from_le_bytes(bytes: &[u8]) -> Self {
		String::from_utf8_lossy(bytes).into_owned()
	}
}
impl TryDecodeBinaryEmbeddable for String {
	type Error = core::str::Utf8Error;

	fn try_from_le_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		core::str::from_utf8(bytes).map(|str| str.to_string())
	}
}

impl BinaryEmbeddable for [u8] {
	fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes {
		EmbeddableBytes::Large(Cow::Borrowed(self))
	}
}
impl BinaryEmbeddable for str {
	fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes {
		EmbeddableBytes::Large(Cow::Borrowed(self.as_bytes()))
	}
}
macro_rules! impl_numbers {
	($($ty:ty),+) => {$(
		impl BinaryEmbeddable for $ty {
			fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes {
				self.to_le_bytes().into()
			}
			fn as_be_bytes<'a>(&'a self) -> EmbeddableBytes {
				self.to_be_bytes().into()
			}
		}
		infallible_decode!(impl DecodeBinaryEmbeddable for $ty {
			fn from_le_bytes(bytes: &[u8]) -> Self {
				<$ty>::from_le_bytes(bytes.try_into().unwrap())
			}
		});
	)+}
}
impl_numbers!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
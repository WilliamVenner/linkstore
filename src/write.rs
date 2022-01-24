use std::borrow::Cow;
use crate::{embed::{EmbeddableBytes, EmbeddedBytes}, Embedder, Error};

impl<'a> Embedder<'a> {
	pub fn embed<T: BinaryEmbeddable>(&mut self, name: &'a str, value: &'a T) -> Result<&mut Self, Error> {
		let embeds = self.embeds.get_mut(name).ok_or_else(|| Error::NotPresent(name.to_string()))?;

		for embed in embeds.as_mut() {
			if embed.size != core::mem::size_of::<T>() as u64 {
				return Err(Error::MismatchedSize(embed.size, core::mem::size_of::<T>()));
			}

			embed.bytes = EmbeddedBytes::Set(if embed.little_endian {
				value.as_le_bytes()
			} else {
				value.as_be_bytes()
			});
		}

		Ok(self)
	}
}

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

impl BinaryEmbeddable for bool {
	fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes {
		EmbeddableBytes::Small { size: 1, bytes: [*self as u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] }
	}
}

impl BinaryEmbeddable for Vec<u8> {
	fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes {
		EmbeddableBytes::Large(Cow::Borrowed(self))
	}
}

impl BinaryEmbeddable for String {
	fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes {
		EmbeddableBytes::Large(Cow::Borrowed(self.as_bytes()))
	}
}

impl<const N: usize> BinaryEmbeddable for [u8; N] {
	fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes {
		if N > 16 {
			EmbeddableBytes::Large(Cow::Borrowed(self))
		} else {
			let mut bytes = [0u8; 16];
			bytes[0..N].copy_from_slice(self);
			EmbeddableBytes::Small { size: N as u8, bytes }
		}
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
	)+}
}
impl_numbers!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);
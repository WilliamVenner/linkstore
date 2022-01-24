use crate::{embed::EmbeddedBytes, Embedder, Error};
use std::borrow::Cow;

impl<'a> Embedder<'a> {
	pub fn embed<T: BinaryEmbeddable>(&mut self, name: &'a str, value: &'a T) -> Result<&mut Self, Error> {
		let embeds = self.embeds.get_mut(name).ok_or_else(|| Error::NotPresent(name.to_string()))?;

		for embed in embeds.as_mut() {
			if embed.size != core::mem::size_of::<T>() as u64 {
				return Err(Error::MismatchedSize(embed.size, core::mem::size_of::<T>()));
			}

			embed.bytes = EmbeddedBytes::Set(if embed.little_endian { value.as_le_bytes() } else { value.as_be_bytes() });
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
	fn as_le_bytes<'a>(&'a self) -> Cow<'a, [u8]>;
	fn as_be_bytes<'a>(&'a self) -> Cow<'a, [u8]> {
		self.as_le_bytes()
	}
}

impl BinaryEmbeddable for bool {
	fn as_le_bytes<'a>(&'a self) -> Cow<'a, [u8]> {
		if *self {
			Cow::Borrowed(&[1])
		} else {
			Cow::Borrowed(&[0])
		}
	}
}

impl<T: BinaryEmbeddable, const N: usize> BinaryEmbeddable for [T; N] {
	fn as_le_bytes<'a>(&'a self) -> Cow<'a, [u8]> {
		let mut bytes = Vec::with_capacity(self.len() * core::mem::size_of::<T>());
		for elem in self {
			bytes.extend_from_slice(elem.as_le_bytes().as_ref());
		}
		Cow::Owned(bytes)
	}

	fn as_be_bytes<'a>(&'a self) -> Cow<'a, [u8]> {
		let mut bytes = Vec::with_capacity(self.len() * core::mem::size_of::<T>());
		for elem in self {
			bytes.extend_from_slice(elem.as_be_bytes().as_ref());
		}
		Cow::Owned(bytes)
	}
}

macro_rules! impl_numbers {
	($($ty:ty),+) => {$(
		impl BinaryEmbeddable for $ty {
			fn as_le_bytes<'a>(&'a self) -> Cow<'a, [u8]> {
				self.to_le_bytes().to_vec().into()
			}
			fn as_be_bytes<'a>(&'a self) -> Cow<'a, [u8]> {
				self.to_be_bytes().to_vec().into()
			}
		}
	)+}
}
impl_numbers!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

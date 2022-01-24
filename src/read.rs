use core::marker::PhantomData;
use crate::write::PendingEmbed;

pub trait DecodeBinaryEmbeddable: Sized {
	fn from_le_bytes(bytes: &[u8]) -> Self;
	fn from_be_bytes(bytes: &[u8]) -> Self {
		Self::from_le_bytes(bytes)
	}
}
pub trait TryDecodeBinaryEmbeddable: Sized {
	type Error;
	fn try_from_le_bytes(bytes: &[u8]) -> Result<Self, Self::Error>;
	fn try_from_be_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		Self::try_from_le_bytes(bytes)
	}
}

pub struct TryEmbeddedValueIterator<'a, T> where T: TryDecodeBinaryEmbeddable {
	embeds: &'a [PendingEmbed<'a>],
	idx: usize,
	_phantom: PhantomData<T>
}
impl<'a, T> TryEmbeddedValueIterator<'a, T> where T: TryDecodeBinaryEmbeddable {
	pub(crate) fn new(embeds: &'a [PendingEmbed<'a>]) -> Self {
		Self {
			embeds,
			idx: 0,
			_phantom: Default::default()
		}
	}
}
impl<'a, T: 'a> Iterator for TryEmbeddedValueIterator<'a, T> where T: TryDecodeBinaryEmbeddable {
	type Item = Result<T, <T as TryDecodeBinaryEmbeddable>::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		let embed = match self.embeds.get(self.idx) {
			Some(embed) => embed,
			None => return None
		};
		self.idx += 1;

		if embed.little_endian {
			Some(TryDecodeBinaryEmbeddable::try_from_le_bytes(embed.bytes.as_ref()))
		} else {
			Some(TryDecodeBinaryEmbeddable::try_from_be_bytes(embed.bytes.as_ref()))
		}
	}
}

pub struct EmbeddedValueIterator<'a, T> where T: DecodeBinaryEmbeddable {
	embeds: &'a [PendingEmbed<'a>],
	idx: usize,
	_phantom: PhantomData<T>
}
impl<'a, T> EmbeddedValueIterator<'a, T> where T: DecodeBinaryEmbeddable {
	pub(crate) fn new(embeds: &'a [PendingEmbed<'a>]) -> Self {
		Self {
			embeds,
			idx: 0,
			_phantom: Default::default()
		}
	}
}
impl<'a, T: 'a> Iterator for EmbeddedValueIterator<'a, T> where T: DecodeBinaryEmbeddable {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		let embed = match self.embeds.get(self.idx) {
			Some(embed) => embed,
			None => return None
		};
		self.idx += 1;

		if embed.little_endian {
			Some(DecodeBinaryEmbeddable::from_le_bytes(embed.bytes.as_ref()))
		} else {
			Some(DecodeBinaryEmbeddable::from_be_bytes(embed.bytes.as_ref()))
		}
	}
}
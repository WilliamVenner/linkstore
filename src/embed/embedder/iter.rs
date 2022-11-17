use crate::embed::{
	decode::{DecodeLinkstore, TryDecodeLinkstore},
	embedder::Linkstore,
};
use std::marker::PhantomData;

pub struct TryEmbeddedValueIterator<'a, T>
where
	T: TryDecodeLinkstore,
{
	embeds: &'a [Linkstore<'a>],
	idx: usize,
	_phantom: PhantomData<T>,
}
impl<'a, T> TryEmbeddedValueIterator<'a, T>
where
	T: TryDecodeLinkstore,
{
	pub(crate) fn new(embeds: &'a [Linkstore<'a>]) -> Self {
		Self {
			embeds,
			idx: 0,
			_phantom: Default::default(),
		}
	}
}
impl<'a, T: 'a> Iterator for TryEmbeddedValueIterator<'a, T>
where
	T: TryDecodeLinkstore,
{
	type Item = Result<T, <T as TryDecodeLinkstore>::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		let embed = match self.embeds.get(self.idx) {
			Some(embed) => embed,
			None => return None,
		};
		self.idx += 1;

		Some(TryDecodeLinkstore::try_from_le_bytes(embed.bytes.as_ref()))
	}
}

pub struct EmbeddedValueIterator<'a, T>
where
	T: DecodeLinkstore,
{
	embeds: &'a [Linkstore<'a>],
	idx: usize,
	_phantom: PhantomData<T>,
}
impl<'a, T> EmbeddedValueIterator<'a, T>
where
	T: DecodeLinkstore,
{
	pub(crate) fn new(embeds: &'a [Linkstore<'a>]) -> Self {
		Self {
			embeds,
			idx: 0,
			_phantom: Default::default(),
		}
	}
}
impl<'a, T: 'a> Iterator for EmbeddedValueIterator<'a, T>
where
	T: DecodeLinkstore,
{
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		let embed = match self.embeds.get(self.idx) {
			Some(embed) => embed,
			None => return None,
		};
		self.idx += 1;

		Some(DecodeLinkstore::from_le_bytes(embed.bytes.as_ref()))
	}
}

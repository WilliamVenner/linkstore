use crate::{
	write::{BinaryEmbeddable, PendingEmbed},
	Embedder, Error,
};
use core::marker::PhantomData;
use core::mem::MaybeUninit;

impl<'a> Embedder<'a> {
	pub fn try_read<T: BinaryEmbeddable + TryDecodeBinaryEmbeddable>(&mut self, name: &str) -> Result<TryEmbeddedValueIterator<'_, T>, Error> {
		let embeds = self.embeds.get(name).ok_or_else(|| Error::NotPresent(name.to_string()))?.as_ref();
		if embeds.len() > 0 && embeds[0].size != core::mem::size_of::<T>() as u64 {
			return Err(Error::MismatchedSize(embeds[0].size, core::mem::size_of::<T>()));
		}
		Ok(TryEmbeddedValueIterator::new(embeds))
	}

	pub fn read<T: BinaryEmbeddable + DecodeBinaryEmbeddable>(&mut self, name: &str) -> Result<EmbeddedValueIterator<'_, T>, Error> {
		let embeds = self.embeds.get(name).ok_or_else(|| Error::NotPresent(name.to_string()))?.as_ref();
		if embeds.len() > 0 && embeds[0].size != core::mem::size_of::<T>() as u64 {
			return Err(Error::MismatchedSize(embeds[0].size, core::mem::size_of::<T>()));
		}
		Ok(EmbeddedValueIterator::new(embeds))
	}
}

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

#[derive(thiserror::Error)]
pub enum TryDecodeBinaryEmbeddableArrayError<E> {
	#[error("array had {0} elements, expected {1}")]
	MismatchedElementCount(usize, usize),

	#[error("{0} bytes cannot be decoded into elements consisting of {1} bytes each")]
	MismatchedBytesCount(usize, usize),

	#[error("{0}")]
	Other(#[from] E),
}
impl<T: TryDecodeBinaryEmbeddable, const N: usize> TryDecodeBinaryEmbeddable for [T; N] {
	type Error = TryDecodeBinaryEmbeddableArrayError<<T as TryDecodeBinaryEmbeddable>::Error>;

	fn try_from_le_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		if bytes.len() % core::mem::size_of::<T>() != 0 {
			return Err(TryDecodeBinaryEmbeddableArrayError::MismatchedBytesCount(
				bytes.len(),
				core::mem::size_of::<T>(),
			));
		}
		if bytes.len() < N * core::mem::size_of::<T>() {
			return Err(TryDecodeBinaryEmbeddableArrayError::MismatchedElementCount(
				bytes.len() / core::mem::size_of::<T>(),
				N,
			));
		}

		let mut result = unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() };
		for (i, chunk) in bytes.chunks(core::mem::size_of::<T>()).enumerate() {
			unsafe { *result[i].as_mut_ptr() = T::try_from_le_bytes(chunk)? };
		}
		Ok(result.map(|elem| unsafe { elem.assume_init() }))
	}
}

infallible_decode!(impl DecodeBinaryEmbeddable for bool {
	fn from_le_bytes(bytes: &[u8]) -> Self {
		bytes[0] != 0
	}
});

macro_rules! impl_numbers {
	($($ty:ty),+) => {$(
		infallible_decode!(impl DecodeBinaryEmbeddable for $ty {
			fn from_le_bytes(bytes: &[u8]) -> Self {
				debug_assert_eq!(bytes.len(), core::mem::size_of::<$ty>());
				<$ty>::from_le_bytes(bytes.try_into().unwrap())
			}
		});
	)+}
}
impl_numbers!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

pub struct TryEmbeddedValueIterator<'a, T>
where
	T: TryDecodeBinaryEmbeddable,
{
	embeds: &'a [PendingEmbed<'a>],
	idx: usize,
	_phantom: PhantomData<T>,
}
impl<'a, T> TryEmbeddedValueIterator<'a, T>
where
	T: TryDecodeBinaryEmbeddable,
{
	pub(crate) fn new(embeds: &'a [PendingEmbed<'a>]) -> Self {
		Self {
			embeds,
			idx: 0,
			_phantom: Default::default(),
		}
	}
}
impl<'a, T: 'a> Iterator for TryEmbeddedValueIterator<'a, T>
where
	T: TryDecodeBinaryEmbeddable,
{
	type Item = Result<T, <T as TryDecodeBinaryEmbeddable>::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		let embed = match self.embeds.get(self.idx) {
			Some(embed) => embed,
			None => return None,
		};
		self.idx += 1;

		if embed.little_endian {
			Some(TryDecodeBinaryEmbeddable::try_from_le_bytes(embed.bytes.as_ref()))
		} else {
			Some(TryDecodeBinaryEmbeddable::try_from_be_bytes(embed.bytes.as_ref()))
		}
	}
}

pub struct EmbeddedValueIterator<'a, T>
where
	T: DecodeBinaryEmbeddable,
{
	embeds: &'a [PendingEmbed<'a>],
	idx: usize,
	_phantom: PhantomData<T>,
}
impl<'a, T> EmbeddedValueIterator<'a, T>
where
	T: DecodeBinaryEmbeddable,
{
	pub(crate) fn new(embeds: &'a [PendingEmbed<'a>]) -> Self {
		Self {
			embeds,
			idx: 0,
			_phantom: Default::default(),
		}
	}
}
impl<'a, T: 'a> Iterator for EmbeddedValueIterator<'a, T>
where
	T: DecodeBinaryEmbeddable,
{
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		let embed = match self.embeds.get(self.idx) {
			Some(embed) => embed,
			None => return None,
		};
		self.idx += 1;

		if embed.little_endian {
			Some(DecodeBinaryEmbeddable::from_le_bytes(embed.bytes.as_ref()))
		} else {
			Some(DecodeBinaryEmbeddable::from_be_bytes(embed.bytes.as_ref()))
		}
	}
}

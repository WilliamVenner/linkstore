use core::mem::MaybeUninit;

/// Implemented for types that can be decoded from a linkstore.
///
/// ## Safety
///
/// Implementing this trait is extremely unsafe. The bytes will be effectively [`core::mem::transmute`]d into the type in the compiled binary, so the bytes must be valid and in the correct endianness if applicable.
pub unsafe trait DecodeLinkstore: Sized + TryDecodeLinkstore {
	fn from_le_bytes(bytes: &[u8]) -> Self;
	fn from_be_bytes(bytes: &[u8]) -> Self {
		Self::from_le_bytes(bytes)
	}
}

/// Implemented for types that can be decoded from a linkstore, but may be fallible.
///
/// Some types have validity constraints that must be checked. For example, arrays must contain a fixed number of elements.
///
/// ## Safety
///
/// Implementing this trait is extremely unsafe. The bytes will be effectively [`core::mem::transmute`]d into the type in the compiled binary, so the bytes must be valid and in the correct endianness if applicable.
pub unsafe trait TryDecodeLinkstore: Sized {
	type Error;
	fn try_from_le_bytes(bytes: &[u8]) -> Result<Self, Self::Error>;
	fn try_from_be_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		Self::try_from_le_bytes(bytes)
	}
}

/// Automatically implements an `core::convert::Infallible` implementation of `TryDecodeLinkstore` during implementation of `DecodeLinkstore`
macro_rules! infallible_decode {
	(unsafe impl DecodeLinkstore for $ty:ty {$($tt:tt)+}) => {
		unsafe impl DecodeLinkstore for $ty {$($tt)+}
		unsafe impl TryDecodeLinkstore for $ty {
			type Error = core::convert::Infallible;

			fn try_from_le_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
				Ok(<$ty as DecodeLinkstore>::from_le_bytes(bytes))
			}
		}
	};
}

/// Errors that can occur when decoding an array stored in a linkstore
#[derive(thiserror::Error)]
pub enum TryDecodeLinkstoreArrayError<E> {
	#[error("array had {0} elements, expected {1}")]
	MismatchedElementCount(usize, usize),

	#[error("{0} bytes cannot be decoded into elements consisting of {1} bytes each")]
	MismatchedBytesCount(usize, usize),

	#[error("{0}")]
	Other(#[from] E),
}
unsafe impl<T: TryDecodeLinkstore, const N: usize> TryDecodeLinkstore for [T; N] {
	type Error = TryDecodeLinkstoreArrayError<<T as TryDecodeLinkstore>::Error>;

	fn try_from_le_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
		if bytes.len() % core::mem::size_of::<T>() != 0 {
			return Err(TryDecodeLinkstoreArrayError::MismatchedBytesCount(bytes.len(), core::mem::size_of::<T>()));
		}
		if bytes.len() < N * core::mem::size_of::<T>() {
			return Err(TryDecodeLinkstoreArrayError::MismatchedElementCount(
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

infallible_decode!(
	unsafe impl DecodeLinkstore for bool {
		fn from_le_bytes(bytes: &[u8]) -> Self {
			bytes[0] != 0
		}
	}
);

macro_rules! impl_numbers {
	($($ty:ty),+) => {$(
		infallible_decode!(unsafe impl DecodeLinkstore for $ty {
			fn from_le_bytes(bytes: &[u8]) -> Self {
				debug_assert_eq!(bytes.len(), core::mem::size_of::<$ty>());
				<$ty>::from_le_bytes(bytes.try_into().unwrap())
			}
		});
	)+}
}
impl_numbers!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

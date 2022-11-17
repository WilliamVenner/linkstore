use std::borrow::Cow;

/// A magic byte we use to mark the beginning of a linkstore in the link section.
pub const MAGIC: u8 = 234;

/// Implemented for types that can be encoded into a linkstore.
///
/// ## Safety
///
/// Implementing this trait is extremely unsafe. The bytes will be effectively [`core::mem::transmute`]d into the type in the compiled binary, so the bytes must be valid and normalized to little-endian if applicable.
pub unsafe trait EncodeLinkstore {
	fn as_le_bytes(&self) -> Cow<'_, [u8]>;
}

unsafe impl EncodeLinkstore for bool {
	fn as_le_bytes(&self) -> Cow<'_, [u8]> {
		if *self {
			Cow::Borrowed(&[1])
		} else {
			Cow::Borrowed(&[0])
		}
	}
}

unsafe impl<T: EncodeLinkstore, const N: usize> EncodeLinkstore for [T; N] {
	fn as_le_bytes(&self) -> Cow<'_, [u8]> {
		let mut bytes = Vec::with_capacity(self.len() * core::mem::size_of::<T>());
		for elem in self {
			bytes.extend_from_slice(elem.as_le_bytes().as_ref());
		}
		Cow::Owned(bytes)
	}
}

macro_rules! impl_numbers {
	($($ty:ty),+) => {$(
		unsafe impl EncodeLinkstore for $ty {
			fn as_le_bytes(&self) -> Cow<'_, [u8]> {
				self.to_le_bytes().to_vec().into()
			}
		}
	)+}
}
impl_numbers!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

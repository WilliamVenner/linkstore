#[doc(hidden)]
pub mod private {
	use crate::TryDecodeLinkstore;
	use core::{cell::UnsafeCell, mem::size_of};

	pub use crate::embed::encode::MAGIC;

	#[repr(transparent)]
	pub struct VolatileWrapper<T: Sized>(UnsafeCell<T>);
	impl<T: Sized> VolatileWrapper<T> {
		#[doc(hidden)]
		pub const fn new(val: T) -> Self {
			Self(UnsafeCell::new(val))
		}

		#[doc(hidden)]
		pub fn get(&'static self) -> &'static T {
			unsafe {
				core::mem::forget(core::ptr::read_volatile(self.0.get()));
				&*self.0.get()
			}
		}
	}
	unsafe impl<T: TryDecodeLinkstore + Sized> Sync for VolatileWrapper<T> {}

	pub const fn calc_padding<Container, T>(name: &'static str) -> usize
	where
		Container: Sized,
		T: Sized,
	{
		size_of::<Container>() - (name.len() + 1 + 1) - (size_of::<usize>() * 2) - size_of::<T>()
	}
}

/// Defines linkstores in the current binary.
///
/// ```no_run
/// #[macro_use] extern crate linkstore;
///
/// linkstore! {
///     pub static LINKSTORE_TEST: u64 = 0xDEADBEEF;
///     pub static LINKSTORE_YEAH: u32 = 0xDEADBEEF;
///     pub static LINKSTORE_BYTES: [u8; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
///     pub static LINKSTORE_SHORTS: [u16; 4] = [0xDE, 0xAD, 0xBE, 0xEF];
///     pub static LINKSTORE_BIG: u128 = 0xDEADBEEF;
/// }
///
/// fn main() {
///     unsafe {
///         println!("LINKSTORE_TEST = {:x}", LINKSTORE_TEST::get());
///         println!("LINKSTORE_YEAH = {:x}", LINKSTORE_YEAH::get());
///         println!("LINKSTORE_BYTES = {:?}", LINKSTORE_BYTES::get());
///         println!("LINKSTORE_SHORTS = {:?}", LINKSTORE_SHORTS::get());
///         println!("LINKSTORE_BIG = {:b}", LINKSTORE_BIG::get());
///     }
/// }
/// ```
#[macro_export]
macro_rules! linkstore {
	{$($vis:vis static $name:ident: $ty:ty = $init:expr;)+} => {$(
		#[allow(non_snake_case)]
		$vis mod $name {
			use ::core::mem::{size_of, align_of};
			use $crate::__private::*;

			const NAME: &'static str = stringify!($name);

			#[repr(C)]
			pub struct LinkStoreContainer<T: $crate::EncodeLinkstore> {
				name: [u8; NAME.len() + 1 + 1],
				size: [u8; size_of::<usize>()],
				padding: [u8; size_of::<usize>()],
				pub value: VolatileWrapper<T>
			}

			#[link_section = ".lnkstre"]
			#[used]
			static $name: LinkStoreContainer<$ty> = LinkStoreContainer {
				name: {
					let mut static_bytes = [0u8; NAME.len() + 1 + 1];
					static_bytes[0] = MAGIC;

					let bytes = NAME.as_bytes();
					let mut i = 0;
					while i < NAME.len() {
						static_bytes[i + 1] = bytes[i];
						i += 1;
					}

					static_bytes
				},

				size: size_of::<$ty>().to_le_bytes(),
				padding: calc_padding::<LinkStoreContainer<$ty>, $ty>(NAME).to_le_bytes(),

				value: VolatileWrapper::new($init)
			};

			/// Gets a the value contained in the linkstore.
			///
			/// ## Safety
			///
			/// This function is unsafe because malformed, corrupted or otherwise invalid data in the binary or unsound decoding implementations may cause undefined behavior.
			pub unsafe fn get() -> &'static $ty {
				debug_assert_eq!(align_of::<LinkStoreContainer<$ty>>(), align_of::<$ty>(), "Alignment error - this is a bug with linkstore!");
				$name.value.get()
			}
		}
	)+};
}

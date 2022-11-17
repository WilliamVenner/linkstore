use super::{
	decode::{DecodeLinkstore, TryDecodeLinkstore},
	encode::{EncodeLinkstore, MAGIC},
	io::BinaryHandle,
};
use crate::Error;
use std::{
	borrow::Cow,
	collections::{hash_map::Entry, HashMap},
	io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom},
};

mod formats;

mod util;
use util::MaybeScalar;

mod iter;
use iter::{EmbeddedValueIterator, TryEmbeddedValueIterator};

/// Opens a binary file in read and write mode without truncation.
///
/// The returned [`std::fs::File`] is suitable for use with [`Embedder`]
pub fn open_binary<P: AsRef<std::path::Path>>(path: P) -> Result<std::fs::File, Error> {
	Ok(std::fs::OpenOptions::new().truncate(false).write(true).read(true).open(path)?)
}

#[derive(Debug)]
pub(crate) struct Linkstore<'a> {
	pub(crate) offset: u64,
	pub(crate) size: u64,
	pub(crate) little_endian: bool,
	pub(crate) bytes: LinkstoreBytes<'a>,
}

pub(crate) type Linkstores<'a> = HashMap<String, MaybeScalar<Linkstore<'a>>>;

#[derive(Debug)]
pub(crate) enum LinkstoreBytes<'a> {
	Unchanged(Cow<'a, [u8]>),
	Set(Cow<'a, [u8]>),
}
impl AsRef<[u8]> for LinkstoreBytes<'_> {
	#[inline(always)]
	fn as_ref(&self) -> &[u8] {
		match self {
			Self::Unchanged(bytes) => bytes.as_ref(),
			Self::Set(bytes) => bytes.as_ref(),
		}
	}
}

#[ouroboros::self_referencing]
pub struct OwnedObject<'a, IO>
where
	IO: BinaryHandle<'a>,
{
	handle: &'a mut IO,

	#[borrows(handle)]
	#[covariant]
	bytes: Cow<'this, [u8]>,

	#[borrows(bytes)]
	#[covariant]
	object: goblin::Object<'this>,
}

#[must_use]
/// The `Embedder` allows you to read and manipulate linkstores in a binary executable.
///
/// ## Example
///
/// ```no_run
/// // You can use linkstore::open_binary` to open a binary file from the filesystem.
/// let mut binary: std::fs::File = linkstore::open_binary("C:\\Windows\\system32\\kernel32.dll").unwrap();
///
/// // Alternatively, you can work directly on a memory buffer or memory-mapped file using a `std::io::Cursor`
/// let mut binary: Vec<u8> = std::fs::read("C:\\Windows\\system32\\kernel32.dll").unwrap();
/// let mut binary: std::io::Cursor<&mut [u8]> = std::io::Cursor::new(&mut binary);
///
/// let mut embedder = linkstore::Embedder::new(&mut binary).unwrap();
///
/// embedder.embed("LINKSTORE_TEST", &69_u64).unwrap();
/// embedder.embed("LINKSTORE_YEAH", &420_u32).unwrap();
/// embedder.embed("LINKSTORE_BYTES", &[1_u8, 2, 3, 4]).unwrap();
/// embedder.embed("LINKSTORE_SHORTS", &[1_u16, 2, 3, 4]).unwrap();
/// embedder.embed("LINKSTORE_BIG", &(u128::MAX / 2)).unwrap();
///
/// embedder.finish().unwrap();
/// ```
pub struct Embedder<'a, IO>
where
	IO: BinaryHandle<'a>,
{
	object: OwnedObject<'a, IO>,
	pub(crate) embeds: Linkstores<'a>,
}
impl<'a, IO> Embedder<'a, IO>
where
	IO: BinaryHandle<'a>,
{
	/// Creates a new [`Embedder`] for a binary executable.
	///
	/// The handle must implement [`BinaryHandle`](crate::BinaryHandle)!
	pub fn new(handle: &'a mut IO) -> Result<Embedder<'a, IO>, Error> {
		let bytes = handle.get_memory()?;

		let object = OwnedObjectTryBuilder {
			handle,
			bytes_builder: |handle| Ok(bytes.map(Cow::Owned).or_else(|| handle.get_memory_ref().map(Cow::Borrowed)).unwrap()),
			object_builder: |bytes| Ok::<_, Error>(goblin::Object::parse(bytes.as_ref())?),
		}
		.try_build()?;

		let mut embedder = Embedder {
			object,
			embeds: Linkstores::default(),
		};

		embedder.discover_linkstores()?;

		Ok(embedder)
	}

	fn discover_linkstores(&mut self) -> Result<(), Error> {
		let object = self.object.borrow_object();
		let bytes = self.object.borrow_bytes();

		let mut handle = BufReader::with_capacity(256, Cursor::new(bytes.as_ref()));

		formats::discover_linkstores::<IO>(bytes.as_ref(), object, &mut handle, &mut self.embeds, 0)
	}

	fn decode_section(
		embeds: &mut Linkstores,
		handle: &mut BufReader<Cursor<&[u8]>>,
		header_offset: u64,
		header_size: u64,
		is_64: bool,
		little_endian: bool,
		fat_offset: u64,
	) -> Result<(), Error> {
		use core::mem::size_of;

		handle.seek(SeekFrom::Start(header_offset as _))?;

		let mut header_size = header_size as usize;

		// 1 magic byte
		// 1 nul byte
		// 2 usizes
		// the rest is variable length
		let minimum_header_size = 1 + 1 + ((if is_64 { size_of::<u64>() } else { size_of::<u32>() }) * 2);

		macro_rules! read_type {
			($ty:ty) => {{
				let mut buf = [0u8; size_of::<$ty>()];
				handle.read_exact(&mut buf)?;
				<$ty>::from_le_bytes(buf)
			}};
		}

		macro_rules! move_header_cursor {
			($amount:expr) => {
				header_size = match header_size.checked_sub($amount) {
					Some(header_size) => header_size,
					None => return Err(Error::UnexpectedEof),
				}
			};
		}

		while header_size >= minimum_header_size {
			match handle.read_until(MAGIC, &mut Vec::new()).map_err(|err| (err.kind(), err)) {
				Ok(n) => {
					header_size = header_size.saturating_sub(n);
					if header_size < minimum_header_size {
						break;
					}
				}
				Err((std::io::ErrorKind::UnexpectedEof, _)) => break,
				Err((_, err)) => return Err(Error::IoError(err)),
			}

			let name = {
				let mut name = Vec::with_capacity(256);
				move_header_cursor!(handle.read_until(0, &mut name)?);

				if name.is_empty() {
					return Err(Error::NameDecodingError);
				}

				String::from_utf8_lossy(&name[0..name.len() - 1]).into_owned()
			};

			let size = {
				if is_64 {
					move_header_cursor!(size_of::<u64>());
					read_type!(u64)
				} else {
					move_header_cursor!(size_of::<u32>());
					read_type!(u32) as u64
				}
			};

			let padding = {
				if is_64 {
					move_header_cursor!(size_of::<u64>());
					read_type!(u64)
				} else {
					move_header_cursor!(size_of::<u32>());
					read_type!(u32) as u64
				}
			};

			move_header_cursor!(padding as usize);
			move_header_cursor!(size as usize);

			handle.seek(SeekFrom::Current(padding as _))?;

			let (offset, bytes) = {
				let offset = handle.seek(SeekFrom::Current(0))?;

				let bytes = {
					let mut bytes = vec![0u8; size as usize];
					handle.read_exact(&mut bytes)?;
					bytes.into()
				};

				(offset, bytes)
			};

			let embed = Linkstore {
				offset: offset + fat_offset,
				size,
				little_endian,
				bytes: LinkstoreBytes::Unchanged(bytes),
			};

			match embeds.entry(name) {
				Entry::Occupied(mut o) => {
					o.get_mut().as_vec().push(embed);
				}
				Entry::Vacant(v) => {
					v.insert(MaybeScalar::Scalar(embed));
				}
			}
		}

		Ok(())
	}

	/// Attempt to fallibly decode & read a value from a linkstore in this binary.
	///
	/// ## Safety
	///
	/// This function is unsafe because malformed, corrupted or otherwise invalid data in the binary or unsound decoding implementations may cause undefined behavior.
	pub unsafe fn try_read<T: EncodeLinkstore + TryDecodeLinkstore>(&mut self, name: &str) -> Result<TryEmbeddedValueIterator<'_, T>, Error> {
		let embeds = self.embeds.get(name).ok_or_else(|| Error::NotPresent(name.to_string()))?.as_ref();
		if !embeds.is_empty() && embeds[0].size != core::mem::size_of::<T>() as u64 {
			return Err(Error::MismatchedSize(embeds[0].size, core::mem::size_of::<T>()));
		}
		Ok(TryEmbeddedValueIterator::new(embeds))
	}

	/// Decode & read a value from a linkstore in this binary.
	///
	/// ## Safety
	///
	/// This function is unsafe because malformed, corrupted or otherwise invalid data in the binary or unsound decoding implementations may cause undefined behavior.
	pub unsafe fn read<T: EncodeLinkstore + DecodeLinkstore>(&mut self, name: &str) -> Result<EmbeddedValueIterator<'_, T>, Error> {
		let embeds = self.embeds.get(name).ok_or_else(|| Error::NotPresent(name.to_string()))?.as_ref();
		if !embeds.is_empty() && embeds[0].size != core::mem::size_of::<T>() as u64 {
			return Err(Error::MismatchedSize(embeds[0].size, core::mem::size_of::<T>()));
		}
		Ok(EmbeddedValueIterator::new(embeds))
	}

	/// Register a linkstore to be embedded.
	///
	/// ## macOS Binaries
	///
	/// Because macOS binaries (namely Mach-O) are signed, patching them invalidates the signature; the signature is formed partially from the contents of the binary itself.
	///
	/// It's out of the scope of `linkstore` to remedy this, so please remember to resign your binaries after embedding linkstores. Most macOS machines will refuse to run binaries with missing or invalid signatures.
	pub fn embed<T: EncodeLinkstore>(&mut self, name: &'a str, value: &'a T) -> Result<&mut Self, Error> {
		let embeds = self.embeds.get_mut(name).ok_or_else(|| Error::NotPresent(name.to_string()))?;

		for embed in embeds.as_mut() {
			if embed.size != core::mem::size_of::<T>() as u64 {
				return Err(Error::MismatchedSize(embed.size, core::mem::size_of::<T>()));
			}

			let bytes = if embed.little_endian { value.as_le_bytes() } else { value.as_be_bytes() };
			if bytes.len() != core::mem::size_of::<T>() {
				return Err(Error::MismatchedSize(bytes.len() as u64, core::mem::size_of::<T>()));
			}

			embed.bytes = LinkstoreBytes::Set(bytes);
		}

		Ok(self)
	}

	/// Consume the Embedder and write the linkstores to the file or memory buffer.
	pub fn finish(self) -> Result<(), Error> {
		let handle = self.object.into_heads().handle;

		for (_, embeds) in self.embeds {
			let mut write_embed = |embed: Linkstore| -> Result<(), Error> {
				if let LinkstoreBytes::Set(bytes) = embed.bytes {
					handle.seek(SeekFrom::Start(embed.offset))?;
					handle.write_all(bytes.as_ref())?;
				}
				Ok(())
			};

			match embeds {
				MaybeScalar::Scalar(scalar) => write_embed(scalar)?,
				MaybeScalar::Vec(vec) => vec.into_iter().try_for_each(write_embed)?,
			}
		}

		Ok(())
	}
}

use std::{io::{Write, BufReader, Seek, SeekFrom, Read, Cursor, BufRead}, collections::{HashMap, hash_map::Entry}, borrow::Cow};
use crate::{write::*, Error, util::{MaybeScalar, MaybeOwnedBytes}};

pub(crate) type Embeds<'a> = HashMap<String, MaybeScalar<PendingEmbed<'a>>>;

mod ar;
mod pe;
mod elf;
mod coff;
mod macho;

fn discover_linkstores_impl<'a>(bytes: &'a [u8], object: &'a goblin::Object, handle: &mut BufReader<Cursor<&[u8]>>, embeds: &mut Embeds, ar_offset: u64) -> Result<(), Error> {
	match object {
		goblin::Object::Elf(elf) => elf::discover_linkstores(embeds, handle, elf, ar_offset),
		goblin::Object::PE(pe) => pe::discover_linkstores(embeds, handle, pe, ar_offset),
		goblin::Object::Archive(ar) => ar::discover_linkstores(embeds, handle, ar, ar_offset),

		goblin::Object::Mach(goblin::mach::Mach::Binary(macho)) => macho::discover_linkstores(embeds, handle, macho, ar_offset),
		goblin::Object::Mach(goblin::mach::Mach::Fat(fat)) => macho::discover_linkstores_multiarch(embeds, handle, fat),

		goblin::Object::Unknown(_) => {
			if ar_offset != 0 {
				if let Ok(coff) = goblin::pe::Coff::parse(bytes) {
					return coff::discover_linkstores(embeds, handle, &coff, ar_offset);
				}
			}
			Err(Error::Unrecognised)
		}
	}
}

fn filter_map_linkstore_section<'a, T>(name: &'a [u8], section: &'a T) -> Option<&'a T> {
	if name.starts_with(super::LINK_SECTION) {
		Some(section)
	} else {
		None
	}
}

#[derive(Debug)]
pub(crate) enum EmbeddedBytes<'a> {
	Unchanged(Cow<'a, [u8]>),
	Set(Cow<'a, [u8]>),
}
impl AsRef<[u8]> for EmbeddedBytes<'_> {
	#[inline(always)]
	fn as_ref(&self) -> &[u8] {
		match self {
			Self::Unchanged(bytes) => bytes.as_ref(),
			Self::Set(bytes) => bytes.as_ref(),
		}
	}
}

pub enum BinaryHandle<'a> {
	File(&'a mut std::fs::File),
	Memory(Cursor<&'a mut [u8]>)
}
impl BinaryHandle<'_> {
	pub fn len(&mut self) -> Option<u64> {
		match self {
			Self::File(f) => {
				f.metadata().ok().map(|metadata| metadata.len()).or_else(|| {
					let current_pos = f.stream_position().ok()?;
					let len = f.seek(SeekFrom::End(0)).ok()?;
					f.seek(SeekFrom::Start(current_pos)).ok()?;
					Some(len)
				})
			},
			Self::Memory(m) => Some(m.get_ref().len() as u64)
		}
	}
}
impl Write for BinaryHandle<'_> {
	#[inline(always)]
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		match self {
			Self::File(f) => f.write(buf),
			Self::Memory(m) => m.write(buf)
		}
	}

	#[inline(always)]
	fn flush(&mut self) -> std::io::Result<()> {
		match self {
			Self::File(f) => f.flush(),
			Self::Memory(m) => m.flush()
		}
	}
}
impl Seek for BinaryHandle<'_> {
	#[inline(always)]
	fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
		match self {
			Self::File(f) => f.seek(pos),
			Self::Memory(m) => m.seek(pos)
		}
	}
}
impl<'a> Into<BinaryHandle<'a>> for &'a mut std::fs::File {
	#[inline(always)]
	fn into(self) -> BinaryHandle<'a> {
		BinaryHandle::File(self)
	}
}
impl<'a> Into<BinaryHandle<'a>> for &'a mut Vec<u8> {
	#[inline(always)]
	fn into(self) -> BinaryHandle<'a> {
		BinaryHandle::Memory(Cursor::new(self))
	}
}

#[ouroboros::self_referencing]
pub struct OwnedObject<'a> {
	handle: BinaryHandle<'a>,

	#[borrows(handle)]
	#[covariant]
	bytes: MaybeOwnedBytes<'this>,

	#[borrows(bytes)]
	#[covariant]
	object: goblin::Object<'this>
}

#[must_use]
pub struct Embedder<'a> {
	object: OwnedObject<'a>,
	pub(crate) embeds: Embeds<'a>
}
impl<'a> Embedder<'a> {
	pub fn new<H: Into<BinaryHandle<'a>>>(handle: H) -> Result<Embedder<'a>, Error> {
		let mut handle = handle.into();
		let len = handle.len();

		let bytes = if let BinaryHandle::File(f) = &mut handle {
			let pos = f.stream_position()?;
			f.seek(SeekFrom::Start(0))?;

			let mut buf = if let Some(len) = len {
				Vec::with_capacity(len as usize)
			} else {
				Vec::new()
			};

			f.read_to_end(&mut buf)?;
			f.seek(SeekFrom::Start(pos))?;

			Some(MaybeOwnedBytes::Owned(buf))
		} else {
			None
		};

		let object = OwnedObjectTryBuilder {
			handle,
			bytes_builder: |handle| Ok(match &handle {
				BinaryHandle::Memory(m) => MaybeOwnedBytes::Borrowed(&**m.get_ref()),
				BinaryHandle::File(_) => bytes.unwrap()
			}),
			object_builder: |bytes| {
				super::parse_binary(bytes.as_ref())
			}
		}.try_build()?;

		let mut embedder = Embedder {
			object,
			embeds: Embeds::default()
		};

		embedder.discover_linkstores()?;

		Ok(embedder)
	}

	fn discover_linkstores(&mut self) -> Result<(), Error> {
		let object = self.object.borrow_object();
		let bytes = self.object.borrow_bytes();

		let mut handle = BufReader::with_capacity(256, Cursor::new(bytes.as_ref()));

		discover_linkstores_impl(
			bytes.as_ref(),
			object,
			&mut handle,
			&mut self.embeds,
			0
		)
	}

	fn decode_section(
		embeds: &mut Embeds, handle: &mut BufReader<Cursor<&'a [u8]>>,
		header_offset: u64, header_size: u64,
		is_64: bool, little_endian: bool,
		fat_offset: u64
	) -> Result<(), Error> {
		use core::mem::size_of;

		handle.seek(SeekFrom::Start(header_offset as _))?;

		let mut header_size = header_size as usize;

		// 1 magic byte
		// 1 nul byte
		// 2 usizes
		// the rest is variable length
		let minimum_header_size = 1 + 1 + ((if is_64 {
			size_of::<u64>()
		} else {
			size_of::<u32>()
		}) * 2);

		macro_rules! read_type {
			($ty:ty) => {{
				let mut buf = [0u8; size_of::<$ty>()];
				handle.read_exact(&mut buf)?;
				<$ty>::from_le_bytes(buf)
			}}
		}

		macro_rules! move_header_cursor {
			($amount:expr) => {
				header_size = match header_size.checked_sub($amount) {
					Some(header_size) => header_size,
					None => return Err(Error::UnexpectedEof)
				}
			}
		}

		while header_size >= minimum_header_size {
			match handle.read_until(crate::MAGIC, &mut Vec::new()).map_err(|err| (err.kind(), err)) {
				Ok(n) => {
					header_size = header_size.saturating_sub(n);
					if header_size < minimum_header_size {
						break;
					}
				},
				Err((std::io::ErrorKind::UnexpectedEof, _)) => break,
				Err((_, err)) => return Err(Error::IoError(err))
			}

			let name = {
				let mut name = Vec::with_capacity(256);
				move_header_cursor!(handle.read_until(0, &mut name)?);

				if name.is_empty() {
					return Err(Error::NameDecodingError);
				}

				String::from_utf8_lossy(&name[0..name.len()-1]).into_owned()
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

			let embed = PendingEmbed {
				offset: offset + fat_offset,
				size,
				little_endian,
				bytes: EmbeddedBytes::Unchanged(bytes)
			};

			match embeds.entry(name) {
				Entry::Occupied(mut o) => { o.get_mut().as_vec().push(embed); },
				Entry::Vacant(v) => { v.insert(MaybeScalar::Scalar(embed)); },
			}
		}

		Ok(())
	}

	pub fn finish(self) -> Result<(), Error> {
		let mut handle = self.object.into_heads().handle;

		for (_, embeds) in self.embeds {
			for embed in embeds {
				if let EmbeddedBytes::Set(bytes) = embed.bytes {
					handle.seek(SeekFrom::Start(embed.offset))?;
					handle.write_all(bytes.as_ref())?;
				}
			}
		}

		Ok(())
	}
}
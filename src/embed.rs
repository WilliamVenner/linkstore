use std::{io::{Write, BufRead, BufReader, Seek, SeekFrom, Read, Cursor}, collections::{hash_map::Entry, HashMap}, borrow::Cow};
use crate::{write::*, read::*, Error, util::{MaybeScalar, MaybeOwnedBytes}};

type Embeds<'a> = HashMap<String, MaybeScalar<PendingEmbed<'a>>>;

mod ar;
mod elf;
mod macho;
mod pe;

fn filter_map_linkstore_section<'a, T>(name: &'a [u8], section: &'a T) -> Option<&'a T> {
	if name.starts_with(super::LINK_SECTION) {
		Some(section)
	} else {
		None
	}
}

#[derive(Debug, Clone)]
pub enum EmbeddableBytes<'a> {
	Small { size: u8, bytes: [u8; 16] },
	Large(Cow<'a, [u8]>)
}
impl AsRef<[u8]> for EmbeddableBytes<'_> {
	fn as_ref(&self) -> &[u8] {
		match self {
			Self::Small { size, bytes } => &bytes[0..*size as usize],
			Self::Large(bytes) => bytes.as_ref()
		}
	}
}
impl From<Vec<u8>> for EmbeddableBytes<'_> {
	fn from(bytes: Vec<u8>) -> Self {
		EmbeddableBytes::Large(Cow::Owned(bytes))
	}
}
macro_rules! embeddable_bytes {
	($($n:literal),+) => {$(
		impl From<[u8; $n]> for EmbeddableBytes<'_> {
			fn from(from: [u8; $n]) -> Self {
				let mut bytes = [0u8; 16];
				bytes[0..$n].copy_from_slice(&from);

				EmbeddableBytes::Small { size: $n, bytes }
			}
		}
		impl BinaryEmbeddable for [u8; $n] {
			fn as_le_bytes<'a>(&'a self) -> EmbeddableBytes {
				let mut bytes = [0u8; 16];
				bytes[0..$n].copy_from_slice(self);

				EmbeddableBytes::Small { size: $n, bytes }
			}
		}
	)+}
}
embeddable_bytes!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);

#[derive(Debug)]
pub(crate) enum EmbeddedBytes<'a> {
	Unchanged(EmbeddableBytes<'a>),
	Set(EmbeddableBytes<'a>),
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
	embeds: Embeds<'a>
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

		match object {
			goblin::Object::Elf(elf) => elf::discover_linkstores(&mut self.embeds, &mut handle, elf, 0),
			goblin::Object::PE(pe) => pe::discover_linkstores(&mut self.embeds, &mut handle, pe),
			goblin::Object::Mach(goblin::mach::Mach::Binary(macho)) => macho::discover_linkstores(&mut self.embeds, &mut handle, macho, 0),
			goblin::Object::Mach(goblin::mach::Mach::Fat(fat)) => macho::discover_linkstores_multiarch(&mut self.embeds, &mut handle, fat),
			goblin::Object::Archive(ar) => ar::discover_linkstores(&mut self.embeds, &mut handle, ar),
			goblin::Object::Unknown(_) => Err(Error::Unrecognised)
		}

	}

	fn decode_section(
		embeds: &mut Embeds, handle: &mut BufReader<Cursor<&'a [u8]>>,
		header_offset: u64, header_size: u64,
		is_64: bool, little_endian: bool,
		fat_offset: u64
	) -> Result<(), Error> {
		let minimum_header_size = (if is_64 {
			core::mem::size_of::<u64>()
		} else {
			core::mem::size_of::<u32>()
		}) * 3;

		handle.seek(SeekFrom::Start(header_offset))?;

		let mut header_size = header_size as usize;
		while header_size > minimum_header_size {
			let mut name = Vec::with_capacity(256);
			handle.read_until(0, &mut name)?;

			if name[name.len() - 1] == 0 {
				name.pop();
			}

			if name.is_empty() {
				return Err(Error::DecodingError);
			}

			header_size -= name.len() + 1;

			let name = String::from_utf8_lossy(&name).into_owned();
			let size = if is_64 {
				let mut buf = [0u8; core::mem::size_of::<u64>()];
				header_size -= buf.len();
				handle.read_exact(&mut buf)?;
				u64::from_le_bytes(buf)
			} else {
				let mut buf = [0u8; core::mem::size_of::<u32>()];
				header_size -= buf.len();
				handle.read_exact(&mut buf)?;
				u32::from_le_bytes(buf) as u64
			};

			let padding = if is_64 {
				let mut buf = [0u8; core::mem::size_of::<u64>()];
				header_size -= buf.len();
				handle.read_exact(&mut buf)?;
				u64::from_le_bytes(buf)
			} else {
				let mut buf = [0u8; core::mem::size_of::<u32>()];
				header_size -= buf.len();
				handle.read_exact(&mut buf)?;
				u32::from_le_bytes(buf) as u64
			};

			header_size -= (size + padding) as usize;

			let offset = handle.seek(SeekFrom::Current(padding as _))?;

			let bytes = if size > 16 {
				let mut bytes = vec![0u8; size as usize];
				handle.read_exact(&mut bytes)?;
				EmbeddableBytes::Large(bytes.into())
			} else {
				let mut bytes = [0u8; 16];
				handle.read_exact(&mut bytes[..size as usize])?;
				EmbeddableBytes::Small { size: size as u8, bytes }
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

	pub fn try_read<T: BinaryEmbeddable + TryDecodeBinaryEmbeddable>(&mut self, name: &str) -> Result<TryEmbeddedValueIterator<'_, T>, Error> {
		Ok(TryEmbeddedValueIterator::new(self.embeds.get(name).ok_or_else(|| Error::NotPresent(name.to_string()))?.as_ref()))
	}

	pub fn read<T: BinaryEmbeddable + DecodeBinaryEmbeddable>(&mut self, name: &str) -> Result<EmbeddedValueIterator<'_, T>, Error> {
		Ok(EmbeddedValueIterator::new(self.embeds.get(name).ok_or_else(|| Error::NotPresent(name.to_string()))?.as_ref()))
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

#[test]
fn dump_linkstore() {
	let mut binary = crate::open_binary("target/debug/liblinkstore.a").unwrap();
	let mut embedder = Embedder::new(&mut binary).unwrap();
	println!("{:?}", embedder.read::<u64>("LOLOLO").unwrap().next());
	println!("{:?}", embedder.read::<u64>("LOLOLO2").unwrap().next());
	embedder.embed("LOLOLO", &69_u64).unwrap();
	embedder.embed("LOLOLO2", &420_u64).unwrap();
	embedder.finish().unwrap();
}
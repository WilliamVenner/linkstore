use std::{io::{Seek, SeekFrom, Read, Write, Cursor}, fs::File};

use crate::Error;

#[doc(hidden)]
pub trait BinaryHandle<'a>: Read + Write + Seek {
	fn get_memory_ref(&self) -> Option<&[u8]>;
	fn get_memory(&mut self) -> Result<Option<Vec<u8>>, Error>;
	fn size_hint(&mut self) -> Option<u64>;
}
impl<'a> BinaryHandle<'a> for Cursor<&'a mut [u8]> {
	#[inline]
	fn get_memory_ref(&self) -> Option<&[u8]> {
		Some(&**self.get_ref())
	}

	#[inline]
	fn get_memory(&mut self) -> Result<Option<Vec<u8>>, Error> {
		Ok(None)
	}

	#[inline]
	fn size_hint(&mut self) -> Option<u64> {
		Some(self.get_ref().len() as _)
	}
}
impl<'a> BinaryHandle<'a> for File {
	#[inline]
	fn get_memory_ref(&self) -> Option<&[u8]> {
		None
	}

	fn get_memory(&mut self) -> Result<Option<Vec<u8>>, Error> {
		let pos = self.stream_position()?;
		self.seek(SeekFrom::Start(0))?;

		let mut buf = if let Some(len) = self.size_hint() {
			Vec::with_capacity(len as usize)
		} else {
			Vec::new()
		};

		self.read_to_end(&mut buf)?;
		self.seek(SeekFrom::Start(pos))?;

		Ok(Some(buf))
	}

	fn size_hint(&mut self) -> Option<u64> {
		self.metadata().ok().map(|metadata| metadata.len()).or_else(|| {
			let current_pos = self.stream_position().ok()?;
			let len = self.seek(SeekFrom::End(0)).ok()?;
			self.seek(SeekFrom::Start(current_pos)).ok()?;
			Some(len)
		})
	}
}
use super::*;
pub(super) fn discover_linkstores<'a>(all_embeds: &mut Embeds, handle: &mut BufReader<Cursor<&'a [u8]>>, ar: &goblin::archive::Archive) -> Result<(), Error> {
	let handle = handle.get_mut().get_mut();

	let mut i = 0;
	while i < ar.len() {
		let bin = ar.get_at(i).ok_or(Error::DecodingError)?;

		let offset = bin.offset;
		let bin = &handle[offset as usize..][..bin.size() as usize];

		elf::discover_linkstores(
			all_embeds,
			&mut BufReader::new(Cursor::new(bin)),
			&goblin::elf::Elf::parse(bin)?,
			offset
		)?;

		i += 1;
	}

	Ok(())
}
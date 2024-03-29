use super::*;

pub(super) fn discover_linkstores<'a, IO: BinaryHandle<'a> + 'a>(
	all_embeds: &mut Linkstores,
	handle: &mut BufReader<Cursor<&[u8]>>,
	ar: &goblin::archive::Archive,
	ar_offset: u64,
) -> Result<(), Error> {
	let handle = handle.get_mut().get_mut();

	let mut i = 0;
	while i < ar.len() {
		let bin = ar.get_at(i).ok_or(Error::DecodingError)?;

		let offset = ar_offset + bin.offset;
		let bin = &handle[offset as usize..][..bin.size() as usize];

		super::discover_linkstores::<IO>(
			bin,
			&goblin::Object::parse(bin)?,
			&mut BufReader::new(Cursor::new(bin)),
			all_embeds,
			offset,
		)?;

		i += 1;
	}

	Ok(())
}

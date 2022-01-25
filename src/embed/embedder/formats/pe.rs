use super::*;
pub(super) fn discover_linkstores<'a, IO: BinaryHandle<'a> + 'a>(
	embeds: &mut Linkstores,
	handle: &mut BufReader<Cursor<&[u8]>>,
	pe: &goblin::pe::PE,
	ar_offset: u64,
) -> Result<(), Error> {
	for header in pe
		.sections
		.iter()
		.filter_map(|section| filter_map_linkstore_section(&section.name, section))
	{
		Embedder::<IO>::decode_section(
			embeds,
			handle,
			header.pointer_to_raw_data as _,
			header.virtual_size as _,
			pe.is_64,
			true,
			ar_offset,
		)?;
	}
	Ok(())
}

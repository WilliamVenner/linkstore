use super::*;
pub(super) fn discover_linkstores<'a>(embeds: &mut Embeds, handle: &mut BufReader<Cursor<&'a [u8]>>, pe: &goblin::pe::PE) -> Result<(), Error> {
	for header in pe.sections.iter().filter_map(|section| {
		filter_map_linkstore_section(
			&section.name,
			section
		)
	}) {
		Embedder::decode_section(
			embeds, handle,
			header.pointer_to_raw_data as _, header.virtual_size as _,
			pe.is_64, true, 0
		)?;
	}
	Ok(())
}
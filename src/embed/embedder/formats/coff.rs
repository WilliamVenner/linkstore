use super::*;

const IMAGE_FILE_32BIT_MACHINE: u16 = 0x0100;

pub(super) fn discover_linkstores<'a, IO: BinaryHandle<'a> + 'a>(
	embeds: &mut Linkstores,
	handle: &mut BufReader<Cursor<&[u8]>>,
	coff: &goblin::pe::Coff,
	ar_offset: u64,
) -> Result<(), Error> {
	for header in coff
		.sections
		.iter()
		.filter_map(|section| filter_map_linkstore_section(&section.name, section))
	{
		Embedder::<IO>::decode_section(
			embeds,
			handle,
			header.pointer_to_raw_data as _,
			header.virtual_size as _,
			dbg!(coff.header.characteristics & IMAGE_FILE_32BIT_MACHINE != 0),
			true,
			ar_offset,
		)?;
	}
	Ok(())
}

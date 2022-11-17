use super::*;

pub(super) fn discover_linkstores<'a, IO: BinaryHandle<'a> + 'a>(
	embeds: &mut Linkstores,
	handle: &mut BufReader<Cursor<&[u8]>>,
	elf: &goblin::elf::Elf,
	ar_offset: u64,
) -> Result<(), Error> {
	for header in elf
		.section_headers
		.iter()
		.filter_map(|section| filter_map_linkstore_section(elf.shdr_strtab.get_at(section.sh_name)?.as_bytes(), section))
	{
		Embedder::<IO>::decode_section(embeds, handle, header.sh_offset, header.sh_size, elf.is_64, elf.little_endian, ar_offset)?;
	}
	Ok(())
}

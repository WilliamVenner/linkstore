use super::*;

pub(super) fn discover_linkstores<'a, IO: BinaryHandle<'a> + 'a>(
	embeds: &mut Linkstores,
	handle: &mut BufReader<Cursor<&[u8]>>,
	macho: &goblin::mach::MachO,
	fat_offset: u64,
) -> Result<(), Error> {
	for segment in macho.segments.iter() {
		for section in segment
			.sections()?
			.iter()
			.filter_map(|(section, _)| filter_map_linkstore_section(section.name().ok()?.as_bytes(), section))
		{
			Embedder::<IO>::decode_section(embeds, handle, section.offset as u64, section.size, fat_offset)?;
		}
	}
	Ok(())
}

pub(super) fn discover_linkstores_multiarch<'a, IO: BinaryHandle<'a> + 'a>(
	embeds: &mut Linkstores,
	handle: &mut BufReader<Cursor<&[u8]>>,
	multiarch: &goblin::mach::MultiArch,
) -> Result<(), Error> {
	for (i, arch) in multiarch.iter_arches().enumerate() {
		let offset = arch?.offset as u64;
		let arch = multiarch.get(i)?;
		match arch {
			goblin::mach::SingleArch::MachO(macho) => discover_linkstores::<IO>(embeds, handle, &macho, offset)?,
			goblin::mach::SingleArch::Archive(ar) => super::ar::discover_linkstores::<IO>(embeds, handle, &ar, offset)?,
		}
	}
	Ok(())
}

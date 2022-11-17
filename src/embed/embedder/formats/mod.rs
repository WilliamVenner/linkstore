use super::*;

mod ar;
mod coff;
mod elf;
mod pe;
pub mod macho;

fn filter_map_linkstore_section<'a, T>(name: &'a [u8], section: &'a T) -> Option<&'a T> {
	if name == b".lnkstre" {
		Some(section)
	} else {
		None
	}
}

pub(super) fn discover_linkstores<'a, IO: BinaryHandle<'a> + 'a>(
	bytes: &[u8],
	object: &goblin::Object,
	handle: &mut BufReader<Cursor<&[u8]>>,
	embeds: &mut Linkstores,
	ar_offset: u64,
) -> Result<(), Error> {
	match object {
		goblin::Object::Elf(elf) => elf::discover_linkstores::<IO>(embeds, handle, elf, ar_offset),
		goblin::Object::PE(pe) => pe::discover_linkstores::<IO>(embeds, handle, pe, ar_offset),
		goblin::Object::Archive(ar) => ar::discover_linkstores::<IO>(embeds, handle, ar, ar_offset),

		goblin::Object::Mach(goblin::mach::Mach::Binary(macho)) => macho::discover_linkstores::<IO>(embeds, handle, macho, ar_offset),
		goblin::Object::Mach(goblin::mach::Mach::Fat(fat)) => macho::discover_linkstores_multiarch::<IO>(embeds, handle, fat),

		goblin::Object::Unknown(_) => {
			if ar_offset != 0 {
				if let Ok(coff) = goblin::pe::Coff::parse(bytes) {
					return coff::discover_linkstores::<IO>(embeds, handle, &coff, ar_offset);
				}
			}
			Err(Error::Unrecognised)
		}
	}
}

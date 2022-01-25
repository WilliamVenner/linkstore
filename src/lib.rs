#[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
compile_error!("Unsupported pointer width");

#[cfg(not(any(feature = "embedder", feature = "store")))]
compile_error!("Please enable at least one of the following features for linkstore: `embedder`, `store`");

#[cfg(test)]
mod tests;

#[cfg(feature = "store")]
mod store;

mod embed;

/// Errors that can occur when using linkstore.
#[derive(thiserror::Error, Debug)]
pub enum Error {
	/// An error occured whilst parsing the executable format.
	#[error("{0}")]
	Goblin(#[from] goblin::error::Error),

	/// The size of the type you are reading or writing does not match the size stored in the executable
	#[error("Size of linkstore ({0} bytes) does not match size of value ({1} bytes)")]
	MismatchedSize(u64, usize),

	/// Binary doesn't contain a linkstore with this name
	#[error("Linkstore contains no key with name {0}")]
	NotPresent(String),

	/// Binary doesn't contain any linkstores
	#[error("Binary does not contain a Linkstore section")]
	NoLinkstore,

	/// The executable format is not recognised
	#[error("Unknown binary format")]
	Unrecognised,

	/// Generic error when decoding linkstores from a binary
	#[error("Format of .lnkstore section is corrupt, unsupported or a bug may be present")]
	DecodingError,

	/// The name of the linkstore key was invalid or failed to be read
	#[error("Format of .lnkstore section is corrupt, unsupported or a bug may be present")]
	NameDecodingError,

	/// Expected to read some more data from the binary, but it wasn't present
	#[error("Format of .lnkstore section is corrupt, unsupported or a bug may be present (unexpected EOF)")]
	UnexpectedEof,

	/// I/O error
	#[error("I/O error: {0}")]
	IoError(#[from] std::io::Error),
}

// Public exports
pub use goblin;

pub use embed::{encode::EncodeLinkstore, decode::{DecodeLinkstore, TryDecodeLinkstore}};

#[cfg(feature = "embedder")]
pub use embed::embedder::{open_binary, Embedder};

#[cfg(feature = "store")]
pub use store::private as __private;

// Export a sealed version of the `BinaryHandle` trait
/// A handle to a binary executable file that linkstore can use.
///
/// ## Implementors
///
/// [`std::fs::File`]
///
/// [`std::io::Cursor<&mut [u8]>`](https://doc.rust-lang.org/stable/std/io/struct.Cursor.html)
///
/// ## Example
///
/// ```no_run
/// // Open a binary file for use with linkstore
/// let mut file: std::fs::File = linkstore::open_binary("C:\\Windows\\System32\\kernel32.dll").unwrap();
///
/// // Alternatively, use a memory buffer for use with linkstore
/// let mut memory: Vec<u8> = std::fs::read("C:\\Windows\\System32\\kernel32.dll").unwrap();
/// let mut memory: std::io::Cursor<&mut [u8]> = std::io::Cursor::new(&mut memory);
/// ```
pub trait BinaryHandle<'a>: embed::io::BinaryHandle<'a> {}
impl<'a, PRIVATE: embed::io::BinaryHandle<'a>> BinaryHandle<'a> for PRIVATE {}
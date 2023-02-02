use shellexpand::tilde;

use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const PACKS_DIRECTORY: &str = "~/RustGit/.git/objects/pack";

// Reads a fixed number of bytes from a stream.
// Rust's "const generics" make this function very useful.
fn read_bytes<R: Read, const N: usize>(stream: &mut R) -> io::Result<[u8; N]> {
    let mut bytes = [0; N];
    stream.read_exact(&mut bytes)?;
    Ok(bytes)
}

// Reads a big-endian 32-bit (4-byte) integer from a stream
fn read_u32<R: Read>(stream: &mut R) -> io::Result<u32> {
    let bytes = read_bytes(stream)?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_pack_index(file: &str) -> io::Result<()> {
    let mut file = File::open(Path::new(&tilde(PACKS_DIRECTORY).to_string()).join(file))?;

    // Check index header
    let magic = read_bytes(&mut file)?;
    assert_eq!(magic, *b"\xfftOc");
    let version = read_u32(&mut file)?;
    assert_eq!(version, 2);

    // TODO: read the rest of the index
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().collect();
    let [_, index_file] = <[String; 2]>::try_from(args).unwrap();
    read_pack_index(&index_file)
}

use shellexpand::tilde;

use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const HASH_BYTES: usize = 20;
#[derive(PartialEq, PartialOrd)]
struct Hash([u8; HASH_BYTES]);

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

// Read an object hash from a stream
fn read_hash<R: Read>(stream: &mut R) -> io::Result<Hash> {
    let bytes = read_bytes(stream)?;
    Ok(Hash(bytes))
}

fn read_pack_index(file: &str) -> io::Result<()> {
    let mut file = File::open(Path::new(&tilde(PACKS_DIRECTORY).to_string()).join(file))?;

    // Check index header
    let magic = read_bytes(&mut file)?;
    assert_eq!(magic, *b"\xfftOc");
    let version = read_u32(&mut file)?;
    assert_eq!(version, 2);

    // For each of the 256 possible first bytes `b` of a hash,
    // read the cumulative number of objects with first byte <= `b`
    let mut cumulative_objects = [0; 1 << u8::BITS];
    for objects in &mut cumulative_objects {
        *objects = read_u32(&mut file)?;
    }

    // Read the hash of each of the objects.
    // Check that the hashes have the correct first byte and are sorted.
    let mut previous_objects = 0;
    for (first_byte, &objects) in cumulative_objects.iter().enumerate() {
        // The difference in the cumulative number of objects
        // is the number of objects with this first byte
        let mut previous_hash = None;
        for _ in 0..(objects - previous_objects) {
            // We already know the first byte of the hash, so ensure it matches
            let hash = read_hash(&mut file)?;
            assert_eq!(hash.0[0], first_byte as u8);
            if let Some(previous_hash) = previous_hash {
                assert!(hash > previous_hash);
            }
            previous_hash = Some(hash);
        }
        previous_objects = objects;
    }
    // `cumulative_objects[255]` is the total number of objects
    let _total_objects = previous_objects;

    // TODO: read the rest of the index
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().collect();
    let [_, index_file] = <[String; 2]>::try_from(args).unwrap();
    read_pack_index(&index_file)
}

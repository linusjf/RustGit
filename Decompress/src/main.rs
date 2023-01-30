use flate2::read::ZlibDecoder;
use shellexpand::tilde;
use std::fmt::{self, Display, Formatter};
use std::fs::File;
use std::fs::{self};
use std::io::Read;
use std::io::{self, Error, ErrorKind};
use std::str::{self, FromStr};
const HASH_BYTES: usize = 20;

const HEAD_FILE: &str = "~/RustGit/.git/HEAD";
const BRANCH_REFS_DIRECTORY: &str = "~/RustGit/.git/refs/heads/";
const REF_PREFIX: &str = "ref: refs/heads/";
// A (commit) hash is a 20-byte identifier.
// We will see that git also gives hashes to other things.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Hash([u8; HASH_BYTES]);

const OBJECTS_DIRECTORY: &str = "~/RustGit/.git/objects";

// The head is either at a specific commit or a named branch
enum Head {
    Commit(Hash),
    Branch(String),
}

fn hex_char_value(hex_char: u8) -> Option<u8> {
    match hex_char {
        b'0'..=b'9' => Some(hex_char - b'0'),
        b'a'..=b'f' => Some(hex_char - b'a' + 10),
        _ => None,
    }
}

fn hex_to_hash(hex_hash: &[u8]) -> Option<Hash> {
    const BITS_PER_CHAR: usize = 4;
    const CHARS_PER_BYTE: usize = 8 / BITS_PER_CHAR;

    let byte_chunks = hex_hash.chunks_exact(CHARS_PER_BYTE);
    if !byte_chunks.remainder().is_empty() {
        return None;
    }

    let bytes = byte_chunks
        .map(|hex_digits| {
            hex_digits.iter().try_fold(0, |value, &byte| {
                let char_value = hex_char_value(byte)?;
                Some(value << BITS_PER_CHAR | char_value)
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let bytes = <[u8; HASH_BYTES]>::try_from(bytes).ok()?;
    Some(Hash(bytes))
}

impl FromStr for Hash {
    type Err = Error;
    fn from_str(hex_hash: &str) -> io::Result<Self> {
        hex_to_hash(hex_hash.as_bytes())
            .ok_or_else(|| Error::new(ErrorKind::Other, format!("Invalid hash: {}", hex_hash)))
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Turn the hash back into a hexadecimal string
        for byte in self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

fn get_head() -> io::Result<Head> {
    use Head::*;

    let hash_contents = fs::read_to_string(tilde(HEAD_FILE).to_string())?;
    // Remove trailing newline
    let hash_contents = hash_contents.trim_end();
    // If .git/HEAD starts with `ref: refs/heads/`, it's a branch name.
    // Otherwise, it should be a commit hash.
    Ok(match hash_contents.strip_prefix(REF_PREFIX) {
        Some(branch) => Branch(branch.to_string()),
        _ => {
            let hash = Hash::from_str(hash_contents)?;
            Commit(hash)
        }
    })
}

impl Head {
    fn get_hash(&self) -> io::Result<Hash> {
        use Head::*;

        match self {
            Commit(hash) => Ok(*hash),
            Branch(branch) => {
                // Copied from get_branch_head()
                let ref_file = tilde(BRANCH_REFS_DIRECTORY).to_string() + branch;
                let hash_contents = fs::read_to_string(ref_file)?;
                Hash::from_str(hash_contents.trim_end())
            }
        }
    }
}

// Read the byte contents of an object
fn read_object(hash: Hash) -> io::Result<Vec<u8>> {
    // The first 2 characters of the hexadecimal hash form the directory;
    // the rest forms the filename
    let hex_hash = hash.to_string();
    let (directory_name, file_name) = hex_hash.split_at(2);
    let object_file = tilde(OBJECTS_DIRECTORY).to_string() + "/" + directory_name + "/" + file_name;
    let object_file = File::open(object_file)?;
    let mut contents = vec![];
    ZlibDecoder::new(object_file).read_to_end(&mut contents)?;
    Ok(contents)
}

fn main() -> io::Result<()> {
    let head = get_head()?;
    let head_hash = head.get_hash()?;
    let head_contents = read_object(head_hash)?;
    // Spoiler alert: the commit object is a text file, so print it as a string
    let head_contents = String::from_utf8(head_contents).unwrap();
    println!("Object {} contents:", head_hash);
    println!("{:?}", head_contents);
    Ok(())
}

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

const COMMIT_HEADER: &[u8] = b"commit ";
const TREE_LINE_PREFIX: &[u8] = b"tree ";
const PARENT_LINE_PREFIX: &[u8] = b"parent ";
const AUTHOR_LINE_PREFIX: &[u8] = b"author ";
const COMMITTER_LINE_PREFIX: &[u8] = b"committer ";

// Some helper functions for parsing objects

fn decimal_char_value(decimal_char: u8) -> Option<u8> {
    match decimal_char {
        b'0'..=b'9' => Some(decimal_char - b'0'),
        _ => None,
    }
}

// Parses a decimal string, e.g. "123", into its value, e.g. 123.
// Returns None if any characters are invalid or the value overflows a usize.
fn parse_decimal(decimal_str: &[u8]) -> Option<usize> {
    let mut value = 0usize;
    for &decimal_char in decimal_str {
        let char_value = decimal_char_value(decimal_char)?;
        value = value.checked_mul(10)?;
        value = value.checked_add(char_value as usize)?;
    }
    Some(value)
}

// Like str::split_once(), split the slice at the next delimiter
fn split_once<T: PartialEq>(slice: &[T], delimiter: T) -> Option<(&[T], &[T])> {
    let index = slice.iter().position(|element| *element == delimiter)?;
    Some((&slice[..index], &slice[index + 1..]))
}

// Checks that an object's header has the expected type, e.g. "commit ",
// and the object size is correct
fn check_header<'a>(object: &'a [u8], header: &[u8]) -> Option<&'a [u8]> {
    let object = object.strip_prefix(header)?;
    let (size, object) = split_once(object, b'\0')?;
    let size = parse_decimal(size)?;
    if object.len() != size {
        return None;
    }

    Some(object)
}

#[derive(Debug)]
struct Commit {
    _tree: Hash,
    _parents: Vec<Hash>,
    _author: String,    // name, email, and timestamp (not parsed)
    _committer: String, // same contents as `author`
    _message: String,   // includes commit description
}

fn parse_commit(object: &[u8]) -> Option<Commit> {
    let object = check_header(object, COMMIT_HEADER)?;

    let object = object.strip_prefix(TREE_LINE_PREFIX)?;
    let (tree, mut object) = split_once(object, b'\n')?;
    let tree = hex_to_hash(tree)?;

    let mut parents = vec![];
    while let Some(object_rest) = object.strip_prefix(PARENT_LINE_PREFIX) {
        let (parent, object_rest) = split_once(object_rest, b'\n')?;
        let parent = hex_to_hash(parent)?;
        parents.push(parent);
        object = object_rest;
    }

    let object = object.strip_prefix(AUTHOR_LINE_PREFIX)?;
    let (author, object) = split_once(object, b'\n')?;
    let author = String::from_utf8(author.to_vec()).ok()?;

    let object = object.strip_prefix(COMMITTER_LINE_PREFIX)?;
    let (committer, object) = split_once(object, b'\n')?;
    let committer = String::from_utf8(committer.to_vec()).ok()?;

    let object = object.strip_prefix(b"\n")?;
    let message = String::from_utf8(object.to_vec()).ok()?;

    Some(Commit {
        _tree: tree,
        _parents: parents,
        _author: author,
        _committer: committer,
        _message: message,
    })
}

fn read_commit(hash: Hash) -> io::Result<Commit> {
    let object = read_object(hash)?;
    parse_commit(&object).ok_or_else(|| {
        Error::new(
            ErrorKind::Other,
            format!("Malformed commit object: {}", hash),
        )
    })
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
    let commit = read_commit(head_hash)?;
    println!("Commit {}:", head_hash);
    println!("{:x?}", commit);
    Ok(())
}

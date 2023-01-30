use shellexpand::tilde;
use std::fs;
use std::io;

const HEAD_FILE: &str = "~/RustGit/.git/HEAD";

fn get_head() -> io::Result<String> {
    fs::read_to_string(tilde(HEAD_FILE).to_string())
}

fn main() -> io::Result<()> {
    let head = get_head()?;
    println!("Head file: {:?}", head);
    Ok(())
}

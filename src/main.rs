use shellexpand::tilde;
use std::fs;
use std::io;

fn get_head() -> io::Result<String> {
    fs::read_to_string(tilde("~/RustGit/.git/HEAD").to_string())
}

fn main() -> io::Result<()> {
    let head = get_head()?;
    println!("Head file: {:?}", head);
    Ok(())
}

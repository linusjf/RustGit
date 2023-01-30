use shellexpand::tilde;
use std::fs;
use std::io;

const BRANCH_REFS_DIRECTORY: &str = "~/RustGit/.git/refs/heads/";

fn get_branch_head(branch: &str) -> io::Result<String> {
    fs::read_to_string(tilde(BRANCH_REFS_DIRECTORY).to_string() + branch)
}

fn main() -> io::Result<()> {
    let main_head = get_branch_head("main")?;
    println!("main: {:?}", main_head);
    let dev_head = get_branch_head("development")?;
    println!("development: {:?}", dev_head);
    Ok(())
}

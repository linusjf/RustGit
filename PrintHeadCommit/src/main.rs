use shellexpand::tilde;
use std::fs;
use std::io;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::process::Command;

const BRANCH_REFS_DIRECTORY: &str = "~/RustGit/.git/refs/heads/";
const INFO_FILE: &str = "~/RustGit/.git/info/refs";

fn get_branch_head(branch: &str) -> io::Result<String> {
    let fname = tilde(BRANCH_REFS_DIRECTORY).to_string() + branch;
    let path = Path::new(&fname);
    if path.try_exists()? {
        fs::read_to_string(fname)
    } else {
        read_from_info(branch)
    }
}

fn read_from_info(branch: &str) -> io::Result<String> {
    let fname = tilde(INFO_FILE).to_string();
    let path = Path::new(&fname);
    if path.try_exists()? {
        let output = Command::new("sh")
            .arg("-c")
            .arg("grep 'refs/heads/".to_owned() + branch + "' " + &fname + " | awk '{print $1}'")
            .output()
            .expect("failed to execute process: ");
        match String::from_utf8(output.stdout) {
            Ok(v) => Ok(v),
            Err(e) => Err(io::Error::new(ErrorKind::Other, e)),
        }
    } else {
        Err(Error::new(ErrorKind::Other, fname + " does not exist!"))
    }
}

fn main() -> io::Result<()> {
    let main_head = get_branch_head("main")?;
    println!("main: {:?}", main_head);
    let dev_head = get_branch_head("development")?;
    println!("development: {:?}", dev_head);
    Ok(())
}

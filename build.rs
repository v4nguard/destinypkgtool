use std::env;
use std::path::Path;

fn main() {
    let dir = env::current_dir().unwrap();
    println!(
        "cargo:rustc-link-search=native={}",
        Path::new(&dir).display()
    );
}

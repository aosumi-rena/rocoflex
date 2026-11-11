#![cfg_attr(all(windows, feature = "desktop"), windows_subsystem = "windows")]

use rocoflex::Result;
use std::env;
#[cfg(feature = "desktop")]
use std::path::PathBuf;

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        #[cfg(feature = "desktop")]
        None => rocoflex::desktop::run(None),
        #[cfg(feature = "desktop")]
        Some("desktop") if args.len() <= 2 => {
            rocoflex::desktop::run(args.get(1).map(PathBuf::from))
        }
        _ => Err("Usage: rocoflex [desktop [PRESET]]".into()),
    }
}

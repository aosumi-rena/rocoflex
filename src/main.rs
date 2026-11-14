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
        Some("integration-info") if args.len() == 1 => {
            println!("{}", rocoflex::offline::integration_info());
            Ok(())
        }
        Some("integration-check-render") if args.len() == 1 => rocoflex::offline::check_models(),
        Some("offline-render") if args.len() == 2 => {
            let response = rocoflex::offline::run_request(std::path::Path::new(&args[1]))?;
            println!("{}", serde_json::to_string(&response).map_err(|error| error.to_string())?);
            Ok(())
        }
        #[cfg(feature = "desktop")]
        Some("integration-editor") if args.len() == 1 => {
            rocoflex::desktop::integration_editor()
        }
        #[cfg(feature = "desktop")]
        None => rocoflex::desktop::run(None),
        #[cfg(feature = "desktop")]
        Some("desktop") if args.len() <= 2 => {
            rocoflex::desktop::run(args.get(1).map(PathBuf::from))
        }
        _ => Err("Usage: rocoflex [desktop [PRESET] | integration-info | integration-editor | integration-check-render | offline-render REQUEST.json]".into()),
    }
}

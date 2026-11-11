use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn resource_compiler() -> PathBuf {
    if let Some(path) = env::var_os("RC") {
        return path.into();
    }
    if Command::new("rc.exe").arg("/?").output().is_ok() {
        return "rc.exe".into();
    }
    let root = env::var_os("WindowsSdkDir")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("ProgramFiles(x86)").map(|p| PathBuf::from(p).join("Windows Kits/10"))
        })
        .expect("Windows SDK is required to embed the application icon");
    let mut versions = fs::read_dir(root.join("bin"))
        .expect("Read Windows SDK versions")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect::<Vec<_>>();
    versions.sort();
    versions
        .into_iter()
        .rev()
        .map(|p| p.join("x64/rc.exe"))
        .find(|p| p.is_file())
        .expect("Windows SDK resource compiler was not found")
}

fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon.ico");
    println!("cargo:rerun-if-changed=resources/app.rc");
    println!("cargo:rerun-if-env-changed=RC");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
        || env::var_os("CARGO_FEATURE_DESKTOP").is_none()
    {
        return;
    }
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let output = Path::new(&env::var_os("OUT_DIR").unwrap()).join("app-icon.res");
    let status = Command::new(resource_compiler())
        .current_dir(&manifest)
        .arg("/nologo")
        .arg("/fo")
        .arg(&output)
        .arg("resources/app.rc")
        .status()
        .expect("Run Windows resource compiler");
    assert!(
        status.success(),
        "Application icon resource compilation failed"
    );
    println!("cargo:rustc-link-arg-bin=rocoflex={}", output.display());
}

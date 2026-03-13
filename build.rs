use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=rust_taskmgr.rc");
    println!("cargo:rerun-if-changed=sysmon.manifest");

    for asset in [
        "bitmap1.bmp",
        "bitmap2.bmp",
        "bmp00001.bmp",
        "bmpback.bmp",
        "bmpforwa.bmp",
        "default.ico",
        "ledlit.bmp",
        "ledunlit.bmp",
        "main.ico",
        "numbers.bmp",
        "sysmon.manifest",
        "tray0.ico",
        "tray1.ico",
        "tray2.ico",
        "tray3.ico",
        "tray4.ico",
        "tray5.ico",
        "tray6.ico",
        "tray7.ico",
        "tray8.ico",
        "tray9.ico",
        "tray10.ico",
        "tray11.ico",
    ] {
        println!("cargo:rerun-if-changed={asset}");
    }

    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        return;
    }

    if target.contains("msvc") {
        compile_with_rc();
    } else {
        compile_with_windres();
    }
}

fn compile_with_rc() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set"));
    let out_res = out_dir.join("rust_taskmgr.res");

    let rc_path = locate_rc_exe().expect("failed to locate rc.exe; install the Windows SDK or run cargo from a Developer Command Prompt");
    let mut command = Command::new(&rc_path);
    command.arg("/nologo");
    for include_dir in locate_sdk_include_dirs(&rc_path) {
        command.arg("/i").arg(include_dir);
    }
    let status = command
        .arg(format!("/fo{}", out_res.display()))
        .arg("rust_taskmgr.rc")
        .status()
        .expect("failed to invoke rc.exe; install Visual Studio Build Tools with the Windows SDK");

    assert!(status.success(), "rc.exe failed to compile rust_taskmgr.rc");
    println!("cargo:rustc-link-arg={}", out_res.display());
}

fn compile_with_windres() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set"));
    let out_obj = out_dir.join("rust-taskmgr-res.o");

    let status = Command::new("windres")
        .arg("--input")
        .arg("rust_taskmgr.rc")
        .arg("--output-format=coff")
        .arg("--output")
        .arg(&out_obj)
        .status()
        .expect("failed to invoke windres");

    assert!(status.success(), "windres failed to compile rust_taskmgr.rc");
    println!("cargo:rustc-link-arg={}", out_obj.display());
}

fn locate_rc_exe() -> Option<PathBuf> {
    if let (Some(sdk_dir), Some(sdk_version)) = (env::var_os("WindowsSdkDir"), env::var_os("WindowsSDKVersion")) {
        let candidate = Path::new(&sdk_dir)
            .join("bin")
            .join(sdk_version)
            .join("x64")
            .join("rc.exe");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Some(candidate) = locate_rc_from_windows_kits() {
        return Some(candidate);
    }

    if let Some(path_var) = env::var_os("PATH") {
        for entry in env::split_paths(&path_var) {
            let candidate = entry.join("rc.exe");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn locate_rc_from_windows_kits() -> Option<PathBuf> {
    let sdk_root = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let entries = std::fs::read_dir(sdk_root).ok()?;

    let mut version_dirs = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    version_dirs.sort();
    version_dirs.reverse();

    for version_dir in version_dirs {
        let candidate = version_dir.join("x64").join("rc.exe");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn locate_sdk_include_dirs(rc_path: &Path) -> Vec<PathBuf> {
    if let (Some(sdk_dir), Some(sdk_version)) = (env::var_os("WindowsSdkDir"), env::var_os("WindowsSDKVersion")) {
        let include_root = Path::new(&sdk_dir).join("Include").join(sdk_version);
        let dirs = collect_include_dirs(&include_root);
        if !dirs.is_empty() {
            return dirs;
        }
    }

    let Some(version_dir) = rc_path.parent().and_then(Path::parent) else {
        return Vec::new();
    };
    let Some(sdk_root) = version_dir.parent().and_then(Path::parent) else {
        return Vec::new();
    };

    collect_include_dirs(&sdk_root.join("Include").join(version_dir.file_name().unwrap_or_default()))
}

fn collect_include_dirs(include_root: &Path) -> Vec<PathBuf> {
    ["shared", "um", "ucrt", "winrt", "cppwinrt"]
        .into_iter()
        .map(|subdir| include_root.join(subdir))
        .filter(|path| path.exists())
        .collect()
}

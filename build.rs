//! Windows 构建期资源注入脚本。
//! 这里只负责把图标和 manifest 嵌进最终 exe，不参与运行期逻辑。

use std::env;
use std::path::PathBuf;

fn main() {
    // 资源和 manifest 变化后都需要触发重新构建。
    println!("cargo:rerun-if-changed=sysmon.manifest");
    println!("cargo:rerun-if-changed=main.ico");

    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") || !target.contains("msvc") {
        // 只有 Windows MSVC 目标才需要走这条资源注入链路。
        return;
    }

    let mut resources = winres::WindowsResource::new();
    resources.set_icon("main.ico");
    if let Err(error) = resources.compile() {
        panic!("failed to compile Windows icon resources: {error}");
    }

    let manifest_path = PathBuf::from("sysmon.manifest");
    // 直接把 manifest 嵌到最终链接产物里，避免重新引入 `.rc` 文件。
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTUAC:level='requireAdministrator' uiAccess='false'");
    println!(
        "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
        manifest_path.display()
    );
}

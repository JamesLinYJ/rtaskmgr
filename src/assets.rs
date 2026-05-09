//! 运行时内嵌资产加载。
//! 这里负责从当前 exe 模块资源加载图标、位图，并构建应用用到的加速键表。

use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::HINSTANCE;
use windows_sys::Win32::Graphics::Gdi::HBITMAP;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_DELETE, VK_ESCAPE, VK_F5, VK_RETURN, VK_TAB,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateAcceleratorTableW, LoadImageW, ACCEL, FCONTROL, FNOINVERT, FSHIFT, FVIRTKEY, HACCEL,
    HICON, IMAGE_BITMAP, IMAGE_ICON,
};

use crate::resource::{IDC_ENDTASK, IDC_NEXTTAB, IDC_PREVTAB, IDC_SWITCHTO, IDM_HIDE, IDM_REFRESH};

pub const MAIN_ICON_RESOURCE: &str = "APP_MAIN_ICON";
pub const DEFAULT_ICON_RESOURCE: &str = "APP_DEFAULT_ICON";
pub const STRIP_LIT_BITMAP_RESOURCE: &str = "APP_BITMAP_STRIP_LIT";
pub const STRIP_LIT_RED_BITMAP_RESOURCE: &str = "APP_BITMAP_STRIP_LIT_RED";
pub const STRIP_UNLIT_BITMAP_RESOURCE: &str = "APP_BITMAP_STRIP_UNLIT";

pub const TRAY_ICON_RESOURCES: [&str; 12] = [
    "APP_TRAY_0_ICON",
    "APP_TRAY_1_ICON",
    "APP_TRAY_2_ICON",
    "APP_TRAY_3_ICON",
    "APP_TRAY_4_ICON",
    "APP_TRAY_5_ICON",
    "APP_TRAY_6_ICON",
    "APP_TRAY_7_ICON",
    "APP_TRAY_8_ICON",
    "APP_TRAY_9_ICON",
    "APP_TRAY_10_ICON",
    "APP_TRAY_11_ICON",
];

fn to_wide_resource_name(resource_name: &str) -> Vec<u16> {
    resource_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect()
}

fn current_module() -> HINSTANCE {
    // 安全性: null module name asks Windows for the module handle of the current process image.
    unsafe { GetModuleHandleW(null::<u16>()) as HINSTANCE }
}

pub fn load_icon_resource(resource_name: &str, width: i32, height: i32, flags: u32) -> HICON {
    let module = current_module();
    if module.is_null() {
        return null_mut();
    }

    let wide = to_wide_resource_name(resource_name);
    // 安全性: `wide` is a live, NUL-terminated UTF-16 resource name and `LoadImageW` only
    // borrows it for the duration of the call.
    unsafe { LoadImageW(module, wide.as_ptr(), IMAGE_ICON, width, height, flags) as HICON }
}

pub fn load_bitmap_resource(resource_name: &str) -> HBITMAP {
    let module = current_module();
    if module.is_null() {
        return null_mut();
    }

    let wide = to_wide_resource_name(resource_name);
    // 安全性: `wide` is a live, NUL-terminated UTF-16 resource name and `LoadImageW` only
    // borrows it for the duration of the call.
    unsafe { LoadImageW(module, wide.as_ptr(), IMAGE_BITMAP, 0, 0, 0) as HBITMAP }
}

pub fn create_accelerator_table() -> HACCEL {
    // 加速键表在 Rust 侧声明，运行时一次性创建成 Win32 `HACCEL`。
    let accelerators = [
        ACCEL {
            fVirt: FVIRTKEY | FNOINVERT,
            key: VK_DELETE,
            cmd: IDC_ENDTASK as u16,
        },
        ACCEL {
            fVirt: FVIRTKEY | FSHIFT | FNOINVERT,
            key: VK_ESCAPE,
            cmd: IDM_HIDE,
        },
        ACCEL {
            fVirt: FVIRTKEY | FNOINVERT,
            key: VK_F5,
            cmd: IDM_REFRESH,
        },
        ACCEL {
            fVirt: FVIRTKEY | FNOINVERT,
            key: VK_RETURN,
            cmd: IDC_SWITCHTO as u16,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL | FNOINVERT,
            key: VK_TAB,
            cmd: IDC_NEXTTAB,
        },
        ACCEL {
            fVirt: FVIRTKEY | FSHIFT | FCONTROL | FNOINVERT,
            key: VK_TAB,
            cmd: IDC_PREVTAB,
        },
    ];
    // 安全性: `accelerators` is a valid slice of ACCEL entries and the API copies the table.
    unsafe { CreateAcceleratorTableW(accelerators.as_ptr(), accelerators.len() as i32) }
}

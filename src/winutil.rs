use std::ffi::OsStr;

// 跨模块复用的 Win32 工具函数。
// 这里集中放 UTF-16 转换、菜单裁剪、ListView 子类化、重绘控制以及
// 一些与指针宽度相关的安全包装逻辑。
use std::iter;
use std::mem::zeroed;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HINSTANCE, HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CombineRgn, CreateRectRgn, CreateSolidBrush, DeleteObject, FillRgn, HBRUSH, HDC, HRGN,
    GetSysColor, InvalidateRect, SetRectRgn, UpdateWindow, COLOR_WINDOW, RGN_DIFF, RGN_OR,
};
use windows_sys::Win32::System::Threading::{IsWow64Process, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
use windows_sys::Win32::UI::Controls::{
    LVIR_BOUNDS, LVIS_SELECTED, LVM_GETITEMCOUNT, LVM_GETITEMRECT, LVM_GETITEMSTATE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, DeleteMenu, GetClientRect, GetSystemMetrics, GetWindowLongPtrW, HMENU,
    LoadStringW, SendMessageW, SetWindowLongPtrW, GWL_STYLE, GWLP_USERDATA, GWLP_WNDPROC,
    MF_BYCOMMAND, SM_CXEDGE, WM_ERASEBKGND, WM_SETREDRAW, WM_SYSCOLORCHANGE, DWLP_MSGRESULT,
};

use crate::localization::{localized_string, text, TextKey};
use crate::resource::{IDM_ALLCPUS, IDM_RUN};

const REST_NORUN: u32 = 0x0000_0001;
const LVM_SETEXTENDEDLISTVIEWSTYLE: u32 = 0x1036;
const LVS_EX_DOUBLEBUFFER: usize = 0x0001_0000;
static mut LIST_VIEW_WNDPROC: isize = 0;
static mut LIST_VIEW_BRUSH: HBRUSH = null_mut();
static mut LIST_VIEW_VIEW_RGN: HRGN = null_mut();
static mut LIST_VIEW_CLIP_RGN: HRGN = null_mut();

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHRestricted(rest: u32) -> u32;
}

pub fn to_wide_null(text: &str) -> Vec<u16> {
    OsStr::new(text)
        .encode_wide()
        .chain(iter::once(0))
        .collect()
}

pub unsafe fn load_string(hinstance: HINSTANCE, id: u32) -> String {
    // 优先走我们自己的语言表覆盖；如果当前语言没提供这条文案，再回退到资源字符串。
    if let Some(text) = localized_string(id) {
        return text.to_string();
    }

    let mut buffer = vec![0u16; 512];
    let length = LoadStringW(hinstance, id, buffer.as_mut_ptr(), buffer.len() as i32);
    if length <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buffer[..length as usize])
    }
}

pub fn format_resource_string(template: &str, values: &[String]) -> String {
    // 这里实现的是 Task Manager 旧式资源格式里最常见的 `%d/%u/%s/%%` 子集，
    // 足够满足状态栏和托盘提示等场景，不必引入完整的 printf 解析器。
    let mut rendered = String::with_capacity(template.len() + values.iter().map(String::len).sum::<usize>());
    let mut chars = template.chars().peekable();
    let mut index = 0usize;

    while let Some(ch) = chars.next() {
        if ch == '%' {
            match chars.peek().copied() {
                Some('%') => {
                    rendered.push('%');
                    chars.next();
                }
                Some('d') | Some('u') | Some('s') => {
                    chars.next();
                    if let Some(value) = values.get(index) {
                        rendered.push_str(value);
                        index += 1;
                    }
                }
                _ => rendered.push(ch),
            }
        } else {
            rendered.push(ch);
        }
    }

    rendered
}

pub const fn make_int_resource(id: u16) -> *const u16 {
    id as usize as *const u16
}

unsafe fn set_window_long_ptr_value(hwnd: HWND, index: i32, value: isize) -> isize {
    SetWindowLongPtrW(hwnd, index, value as _) as isize
}

unsafe fn get_window_long_ptr_value(hwnd: HWND, index: i32) -> isize {
    GetWindowLongPtrW(hwnd, index) as isize
}

pub unsafe fn set_window_userdata(hwnd: HWND, value: isize) {
    let _ = set_window_long_ptr_value(hwnd, GWLP_USERDATA, value);
}

pub unsafe fn get_window_userdata(hwnd: HWND) -> isize {
    get_window_long_ptr_value(hwnd, GWLP_USERDATA)
}

pub unsafe fn set_style(hwnd: HWND, style: u32) {
    let _ = set_window_long_ptr_value(hwnd, GWL_STYLE, style as isize);
}

pub unsafe fn set_dialog_msg_result(hwnd: HWND, value: isize) {
    let _ = set_window_long_ptr_value(hwnd, DWLP_MSGRESULT as i32, value);
}

pub fn width(rect: &RECT) -> i32 {
    rect.right - rect.left
}

pub fn height(rect: &RECT) -> i32 {
    rect.bottom - rect.top
}

pub fn loword(value: usize) -> u16 {
    (value & 0xFFFF) as u16
}

pub fn hiword(value: usize) -> u16 {
    ((value >> 16) & 0xFFFF) as u16
}

pub unsafe fn sanitize_task_manager_menu(menu: HMENU, processor_count: usize) {
    // 某些菜单项是否可见由系统策略和 CPU 数量决定。
    // 这里在每次加载菜单后做一次裁剪，避免资源文件里维护多套变体。
    if menu.is_null() {
        return;
    }

    if SHRestricted(REST_NORUN) != 0 {
        DeleteMenu(menu, IDM_RUN as u32, MF_BYCOMMAND);
    }

    if processor_count <= 1 {
        DeleteMenu(menu, IDM_ALLCPUS as u32, MF_BYCOMMAND);
    }
}

pub unsafe fn subclass_list_view(hwnd: HWND) {
    // 统一给列表启用双缓冲和自定义背景擦除逻辑，减少自动刷新时的闪烁。
    if hwnd.is_null() {
        return;
    }

    SendMessageW(
        hwnd,
        LVM_SETEXTENDEDLISTVIEWSTYLE,
        LVS_EX_DOUBLEBUFFER,
        LVS_EX_DOUBLEBUFFER as isize,
    );

    if get_window_long_ptr_value(hwnd, GWLP_WNDPROC) == list_view_wnd_proc as *const () as usize as isize {
        return;
    }

    ensure_list_view_paint_state();
    let previous = set_window_long_ptr_value(
        hwnd,
        GWLP_WNDPROC,
        list_view_wnd_proc as *const () as usize as isize,
    );
    if LIST_VIEW_WNDPROC == 0 {
        LIST_VIEW_WNDPROC = previous;
    }
}

pub unsafe fn finish_list_view_update(hwnd: HWND) {
    // 批量更新结束后统一恢复重绘并触发一次刷新，避免每条消息都导致重绘。
    if hwnd.is_null() {
        return;
    }

    SendMessageW(hwnd, WM_SETREDRAW, 1, 0);
    InvalidateRect(hwnd, null(), 0);
    UpdateWindow(hwnd);
}

pub unsafe fn is_32_bit_process_handle(handle: HANDLE) -> bool {
    if handle.is_null() {
        return false;
    }

    let mut wow64 = 0;
    IsWow64Process(handle, &mut wow64) != 0 && wow64 != 0
}

pub unsafe fn is_32_bit_process_pid(pid: u32) -> bool {
    // 只为了查询位数时，打开最低限度的查询句柄即可，减少权限失败的概率。
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if handle.is_null() {
        return false;
    }

    let is_32_bit = is_32_bit_process_handle(handle);
    CloseHandle(handle);
    is_32_bit
}

pub fn append_32_bit_suffix(label: &str, is_32_bit: bool) -> String {
    if !is_32_bit {
        return label.to_string();
    }

    format!("{label} {}", text(TextKey::Bitness32Suffix))
}

unsafe fn ensure_list_view_paint_state() {
    // 这些 GDI 对象在所有 ListView 子类间共享，避免每个控件都单独创建/销毁画刷和区域。
    if LIST_VIEW_BRUSH.is_null() {
        LIST_VIEW_BRUSH = CreateSolidBrush(GetSysColor(COLOR_WINDOW));
    }
    if LIST_VIEW_VIEW_RGN.is_null() {
        LIST_VIEW_VIEW_RGN = CreateRectRgn(0, 0, 0, 0);
    }
    if LIST_VIEW_CLIP_RGN.is_null() {
        LIST_VIEW_CLIP_RGN = CreateRectRgn(0, 0, 0, 0);
    }
}

unsafe fn set_rect_rgn_indirect(region: HRGN, rect: &RECT) {
    SetRectRgn(region, rect.left, rect.top, rect.right, rect.bottom);
}

unsafe fn list_view_get_view_rgn(hwnd: HWND) {
    // 这里会把“未选中项的可视区域”合成为一个区域，
    // 供自定义擦背景时只清理真正需要的空白区域，减少整窗闪烁。
    ensure_list_view_paint_state();
    if LIST_VIEW_VIEW_RGN.is_null() || LIST_VIEW_CLIP_RGN.is_null() {
        return;
    }

    SetRectRgn(LIST_VIEW_VIEW_RGN, 0, 0, 0, 0);
    let item_count = SendMessageW(hwnd, LVM_GETITEMCOUNT, 0, 0) as i32;
    let edge_width = GetSystemMetrics(SM_CXEDGE);

    for index in 0..item_count {
        if (SendMessageW(hwnd, LVM_GETITEMSTATE, index as usize, LVIS_SELECTED as isize) as u32 & LVIS_SELECTED) != 0
        {
            continue;
        }

        let mut item_rect = RECT {
            left: LVIR_BOUNDS as i32,
            ..zeroed()
        };
        if SendMessageW(
            hwnd,
            LVM_GETITEMRECT,
            index as usize,
            &mut item_rect as *mut _ as LPARAM,
        ) == 0
        {
            continue;
        }

        item_rect.left += edge_width;
        set_rect_rgn_indirect(LIST_VIEW_CLIP_RGN, &item_rect);
        CombineRgn(LIST_VIEW_VIEW_RGN, LIST_VIEW_VIEW_RGN, LIST_VIEW_CLIP_RGN, RGN_OR);
    }
}

unsafe extern "system" fn list_view_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    // 自定义 ListView 窗口过程只接管背景擦除相关消息，其余消息回落给原始过程。
    match msg {
        WM_SYSCOLORCHANGE => {
            if !LIST_VIEW_BRUSH.is_null() {
                DeleteObject(LIST_VIEW_BRUSH as _);
                LIST_VIEW_BRUSH = null_mut();
            }
            ensure_list_view_paint_state();
            InvalidateRect(hwnd, null_mut(), 1);
        }
        WM_ERASEBKGND => {
            ensure_list_view_paint_state();
            if !LIST_VIEW_BRUSH.is_null() && !LIST_VIEW_VIEW_RGN.is_null() && !LIST_VIEW_CLIP_RGN.is_null() {
                let hdc = wparam as HDC;
                let mut client_rect = zeroed::<RECT>();
                GetClientRect(hwnd, &mut client_rect);
                list_view_get_view_rgn(hwnd);
                set_rect_rgn_indirect(LIST_VIEW_CLIP_RGN, &client_rect);
                CombineRgn(LIST_VIEW_CLIP_RGN, LIST_VIEW_CLIP_RGN, LIST_VIEW_VIEW_RGN, RGN_DIFF);
                FillRgn(hdc, LIST_VIEW_CLIP_RGN, LIST_VIEW_BRUSH);
                return 1;
            }
        }
        _ => {}
    }

    if LIST_VIEW_WNDPROC != 0 {
        CallWindowProcW(Some(std::mem::transmute(LIST_VIEW_WNDPROC)), hwnd, msg, wparam, lparam)
    } else {
        0
    }
}

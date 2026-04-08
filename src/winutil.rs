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
    CombineRgn, CreateRectRgn, CreateSolidBrush, DeleteObject, FillRgn, GetSysColor,
    InvalidateRect, SetRectRgn, UpdateWindow, COLOR_WINDOW, HBRUSH, HDC, HRGN, RGN_DIFF, RGN_OR,
};
use windows_sys::Win32::System::Threading::{
    IsWow64Process, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::Controls::{LVIR_BOUNDS, LVM_GETITEMCOUNT, LVM_GETITEMRECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, DeleteMenu, GetClientRect, GetSystemMetrics, GetWindowLongPtrW, SendMessageW,
    SetWindowLongPtrW, DWLP_MSGRESULT, GWLP_USERDATA, GWLP_WNDPROC, GWL_STYLE, HMENU, MF_BYCOMMAND,
    SM_CXEDGE, WM_ERASEBKGND, WM_NCDESTROY, WM_SETREDRAW, WM_SYSCOLORCHANGE, WNDPROC,
};

use crate::language::{localized_string, text, TextKey};
use crate::resource::{IDM_ALLCPUS, IDM_RUN};

const REST_NORUN: u32 = 0x0000_0001;
const LVM_SETEXTENDEDLISTVIEWSTYLE: u32 = 0x1036;
const LVS_EX_DOUBLEBUFFER: usize = 0x0001_0000;

struct ListViewPaintState {
    original_wndproc: WNDPROC,
    brush: HBRUSH,
    view_rgn: HRGN,
    clip_rgn: HRGN,
}

impl ListViewPaintState {
    fn new(original_wndproc: WNDPROC) -> Self {
        Self {
            original_wndproc,
            brush: null_mut(),
            view_rgn: null_mut(),
            clip_rgn: null_mut(),
        }
    }

    unsafe fn ensure_resources(&mut self) {
        if self.brush.is_null() {
            self.brush = CreateSolidBrush(GetSysColor(COLOR_WINDOW));
        }
        if self.view_rgn.is_null() {
            self.view_rgn = CreateRectRgn(0, 0, 0, 0);
        }
        if self.clip_rgn.is_null() {
            self.clip_rgn = CreateRectRgn(0, 0, 0, 0);
        }
    }
}

impl Drop for ListViewPaintState {
    fn drop(&mut self) {
        unsafe {
            if !self.brush.is_null() {
                DeleteObject(self.brush as _);
            }
            if !self.view_rgn.is_null() {
                DeleteObject(self.view_rgn as _);
            }
            if !self.clip_rgn.is_null() {
                DeleteObject(self.clip_rgn as _);
            }
        }
    }
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHRestricted(rest: u32) -> u32;
}

pub fn to_wide_null(text: &str) -> Vec<u16> {
    // 大部分 Win32 文本 API 都要求零结尾 UTF-16，这个转换在全项目复用最多。
    OsStr::new(text)
        .encode_wide()
        .chain(iter::once(0))
        .collect()
}

pub unsafe fn load_string(_hinstance: HINSTANCE, id: u32) -> String {
    // 字符串已经转到 Rust 语言层，这里保留旧接口形状以减少调用点改动。
    localized_string(id).unwrap_or_default().to_string()
}

pub fn format_resource_string(template: &str, values: &[String]) -> String {
    // 这里实现的是 Task Manager 旧式资源格式里最常见的 `%d/%u/%s/%%` 子集，
    // 足够满足状态栏和托盘提示等场景，不必引入完整的 printf 解析器。
    let mut rendered =
        String::with_capacity(template.len() + values.iter().map(String::len).sum::<usize>());
    let mut chars = template.chars().peekable();
    let mut index = 0usize;

    while let Some(ch) = chars.next() {
        if ch == '%' {
            match chars.peek().copied() {
                Some('%') => {
                    rendered.push('%');
                    chars.next();
                }
                Some('d' | 'u' | 's') => {
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

pub unsafe fn set_window_userdata_ptr<T>(hwnd: HWND, value: *mut T) {
    set_window_userdata(hwnd, value as isize);
}

pub unsafe fn get_window_userdata_ptr<T>(hwnd: HWND) -> *mut T {
    get_window_userdata(hwnd) as *mut T
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
        DeleteMenu(menu, u32::from(IDM_RUN), MF_BYCOMMAND);
    }

    if processor_count <= 1 {
        DeleteMenu(menu, u32::from(IDM_ALLCPUS), MF_BYCOMMAND);
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

    if !list_view_state_ptr(hwnd).is_null() {
        return;
    }

    let previous = set_window_long_ptr_value(
        hwnd,
        GWLP_WNDPROC,
        list_view_wnd_proc as *const () as usize as isize,
    );
    let state = Box::new(ListViewPaintState::new(isize_to_wndproc(previous)));
    set_window_userdata_ptr(hwnd, Box::into_raw(state));
}

unsafe fn finish_list_view_update_internal(hwnd: HWND, invalidate: bool, immediate: bool) {
    if hwnd.is_null() {
        return;
    }

    SendMessageW(hwnd, WM_SETREDRAW, 1, 0);
    if invalidate {
        InvalidateRect(hwnd, null(), 0);
    }
    if immediate {
        UpdateWindow(hwnd);
    }
}

pub unsafe fn finish_list_view_update(hwnd: HWND) {
    // 适合需要立刻看到完整结果的页面：恢复重绘后马上同步刷新。
    finish_list_view_update_internal(hwnd, true, true);
}

pub unsafe fn finish_list_view_update_deferred(hwnd: HWND) {
    // 高频刷新的列表只恢复重绘，不整窗失效；调用方自己决定该重画哪些行。
    finish_list_view_update_internal(hwnd, false, false);
}

pub unsafe fn is_32_bit_process_handle(handle: HANDLE) -> bool {
    if handle.is_null() {
        return false;
    }

    let mut wow64 = 0;
    IsWow64Process(handle, &raw mut wow64) != 0 && wow64 != 0
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

unsafe fn wndproc_to_isize(wndproc: WNDPROC) -> isize {
    std::mem::transmute::<WNDPROC, isize>(wndproc)
}

unsafe fn isize_to_wndproc(value: isize) -> WNDPROC {
    std::mem::transmute::<isize, WNDPROC>(value)
}

unsafe fn list_view_state_ptr(hwnd: HWND) -> *mut ListViewPaintState {
    get_window_userdata_ptr(hwnd)
}

unsafe fn set_rect_rgn_indirect(region: HRGN, rect: &RECT) {
    SetRectRgn(region, rect.left, rect.top, rect.right, rect.bottom);
}

unsafe fn list_view_get_view_rgn(hwnd: HWND, state: &mut ListViewPaintState) {
    // 这里把“所有可视项区域”合成为一个区域，
    // 让背景擦除只覆盖真正的空白区，而不是先把选中行也擦掉。
    state.ensure_resources();
    if state.view_rgn.is_null() || state.clip_rgn.is_null() {
        return;
    }

    SetRectRgn(state.view_rgn, 0, 0, 0, 0);
    let item_count = SendMessageW(hwnd, LVM_GETITEMCOUNT, 0, 0) as i32;
    let edge_width = GetSystemMetrics(SM_CXEDGE);

    for index in 0..item_count {
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
        set_rect_rgn_indirect(state.clip_rgn, &item_rect);
        CombineRgn(state.view_rgn, state.view_rgn, state.clip_rgn, RGN_OR);
    }
}

unsafe extern "system" fn list_view_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    // 自定义 ListView 窗口过程只接管背景擦除相关消息，其余消息回落给原始过程。
    let state_ptr = list_view_state_ptr(hwnd);
    let Some(state) = state_ptr.as_mut() else {
        return 0;
    };

    match msg {
        WM_SYSCOLORCHANGE => {
            if !state.brush.is_null() {
                DeleteObject(state.brush as _);
                state.brush = null_mut();
            }
            state.ensure_resources();
            InvalidateRect(hwnd, null_mut(), 1);
        }
        WM_ERASEBKGND => {
            state.ensure_resources();
            if !state.brush.is_null() && !state.view_rgn.is_null() && !state.clip_rgn.is_null() {
                let hdc = wparam as HDC;
                let mut client_rect = zeroed::<RECT>();
                GetClientRect(hwnd, &mut client_rect);
                list_view_get_view_rgn(hwnd, state);
                set_rect_rgn_indirect(state.clip_rgn, &client_rect);
                CombineRgn(state.clip_rgn, state.clip_rgn, state.view_rgn, RGN_DIFF);
                FillRgn(hdc, state.clip_rgn, state.brush);
                return 1;
            }
        }
        WM_NCDESTROY => {
            let original_wndproc = state.original_wndproc;
            set_window_userdata_ptr::<ListViewPaintState>(hwnd, null_mut());
            let _ =
                set_window_long_ptr_value(hwnd, GWLP_WNDPROC, wndproc_to_isize(original_wndproc));
            let result = if let Some(wndproc) = original_wndproc {
                CallWindowProcW(Some(wndproc), hwnd, msg, wparam, lparam)
            } else {
                0
            };
            drop(Box::from_raw(state_ptr));
            return result;
        }
        _ => {}
    }

    if let Some(wndproc) = state.original_wndproc {
        CallWindowProcW(Some(wndproc), hwnd, msg, wparam, lparam)
    } else {
        0
    }
}

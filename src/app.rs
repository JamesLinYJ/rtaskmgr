//! 应用主控模块。
//! 这里负责 Win32 启动、主窗口生命周期、消息循环、菜单与托盘状态，
//! 并统一协调各个页面的初始化、激活和定时刷新。

use std::env;
use std::mem::{size_of, transmute, zeroed};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    CloseHandle, FILETIME, FreeLibrary, HANDLE, HINSTANCE, HWND, INVALID_HANDLE_VALUE, LPARAM,
    POINT, RECT, TRUE, WPARAM, ERROR_ALREADY_EXISTS,
};
use windows_sys::Win32::Graphics::Gdi::{
    CreateRectRgn, DeleteObject, FillRect, GetDC, GetDCEx, GetDeviceCaps, GetSysColorBrush,
    GetUpdateRgn, ReleaseDC, COLOR_3DFACE, DCX_CACHE, DCX_CLIPSIBLINGS, DCX_INTERSECTRGN,
    LOGPIXELSX,
};
use windows_sys::Win32::Graphics::Gdi::MapWindowPoints;
use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
};
use windows_sys::Win32::System::SystemInformation::{
    GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, SYSTEM_INFO,
};
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetSystemTimes, ReleaseMutex, SetProcessShutdownParameters, WaitForSingleObject,
};
use windows_sys::Win32::UI::Controls::{
    InitCommonControlsEx, NMHDR, SB_SETPARTS, SB_SETTEXTW, SB_SIMPLE, SB_SIMPLEID,
    SBARS_SIZEGRIP, STATUSCLASSNAMEW, SBT_NOBORDERS, TCM_ADJUSTRECT, TCM_GETCURSEL,
    TCM_INSERTITEMW, TCM_SETCURSEL, TCN_SELCHANGE,
    ICC_BAR_CLASSES, ICC_LISTVIEW_CLASSES, ICC_TAB_CLASSES, INITCOMMONCONTROLSEX,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, ReleaseCapture, SetCapture, VK_CONTROL,
};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, ShellAboutW, WinHelpW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD,
    NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, CheckMenuItem, CheckMenuRadioItem, CreateDialogParamW, CreateWindowExW,
    DefWindowProcW, DeleteMenu, DestroyIcon, DestroyMenu, DestroyWindow, DispatchMessageW, DrawMenuBar,
    EnableMenuItem,
    FindWindowW, GetClassInfoW, GetClientRect, GetCursorPos, GetDlgItem, GetForegroundWindow,
    GetMenu, GetMenuItemInfoW, GetMessageW, GetWindowPlacement, GetWindowRect, GetWindowLongW,
    GetShellWindow, HACCEL, HELP_FINDER, HICON, HMENU, IsDialogMessageW, IsIconic,
    IsWindowVisible, IsZoomed, KillTimer, LoadMenuW, LoadAcceleratorsW, LoadIconW, LoadImageW,
    MENUITEMINFOW, MessageBoxW, MoveWindow, OpenIcon,
    PostQuitMessage, RemoveMenu, SendMessageTimeoutW,
    RegisterClassW, SendMessageW, SetForegroundWindow, SetMenu, SetMenuDefaultItem, SetTimer,
    SetWindowLongW, SetWindowPos, SetWindowTextW, ShowWindow, TrackPopupMenuEx, TranslateAcceleratorW,
    TranslateMessage, WINDOWPLACEMENT, GWL_STYLE, HTCAPTION, HTCLIENT, IDCANCEL, MB_ICONSTOP, MB_OK,
    MF_BYCOMMAND, MF_BYPOSITION, MF_CHECKED, MF_ENABLED, MF_GRAYED, MF_POPUP, MF_SEPARATOR,
    MF_SYSMENU, MF_UNCHECKED, MIIM_ID, MINMAXINFO, MSG, SIZE_MINIMIZED, SMTO_ABORTIFHUNG, SW_HIDE,
    SW_MINIMIZE, SW_SHOW, SW_SHOWMAXIMIZED, SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOREDRAW, SWP_NOSIZE, SWP_NOZORDER,
    TPM_RETURNCMD, WM_CLOSE,
    WM_COMMAND, WM_DESTROY, WM_ENDSESSION, WM_GETMINMAXINFO, WM_INITDIALOG, WM_INITMENU,
    WM_CREATE, WM_ERASEBKGND, WM_LBUTTONDBLCLK, WM_MENUSELECT, WM_MOVE, WM_NCHITTEST,
    WM_NCLBUTTONDBLCLK, WM_NCRBUTTONDOWN, WM_NCRBUTTONUP, WM_NOTIFY, WM_RBUTTONDOWN,
    WM_RBUTTONUP, WM_SETICON,
    WM_SIZE, WM_TIMER, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_POPUP, WS_CLIPSIBLINGS, WS_DLGFRAME,
    WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_TILEDWINDOW, WS_VISIBLE, IMAGE_ICON,
    LR_DEFAULTCOLOR, LR_DEFAULTSIZE, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST,
};

use crate::options::Options;
use crate::pages::{default_pages, DialogPage};
use crate::localization::{localize_dialog, localize_menu};
use crate::resource::*;
use crate::winutil::{
    format_resource_string, height, hiword, load_string, loword, make_int_resource,
    sanitize_task_manager_menu, set_dialog_msg_result, set_style, to_wide_null, width,
};

const STARTUP_MUTEX_NAME: &str = "NTShell Taskman Startup Mutex";
const FINDME_TIMEOUT: u32 = 10_000;
const RUN_DIALOG_CALC_DIRECTORY: u32 = 0x0000_0004;
const NOTIFY_ICON_TIP_CAPACITY: usize = 128;

static mut APP_INSTANCE: Option<App> = None;
static mut FRAME_BASE_WNDPROC: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> isize> = None;

const PERF_FRAME_CLASS_NAME: &str = "TaskManagerFrame";
const BUTTON_CLASS: &str = "Button";

#[derive(Default)]
struct GlobalStrings {
    app_title: String,
    fmt_procs: String,
    fmt_cpu: String,
    fmt_mem: String,
}

#[derive(Default)]
struct RuntimeStats {
    // 运行期统计信息会被状态栏、托盘提示和部分页面共享。
    // 它是页面采样结果与主框架 UI 之间的中转缓存。
    cpu_usage: u8,
    mem_usage_kb: u32,
    mem_limit_kb: u32,
    process_count: u32,
    processor_count: u8,
    previous_idle: u64,
    previous_kernel: u64,
    previous_user: u64,
}

pub struct App {
    // 主应用状态对象统一持有主窗口、菜单、页面和托盘/定时器相关状态。
    hinstance: HINSTANCE,
    main_hwnd: HWND,
    status_hwnd: HWND,
    startup_mutex: HANDLE,
    accelerator_table: HACCEL,
    current_menu: HMENU,
    tray_icons: Vec<HICON>,
    strings: GlobalStrings,
    options: Options,
    pages: [DialogPage; NUM_PAGES],
    stats: RuntimeStats,
    framed_style: u32,
    borderless_style: u32,
    min_width: i32,
    min_height: i32,
    already_applied_initial_position: bool,
    menu_tracking: bool,
    cant_hide: bool,
    in_popup: bool,
    temporarily_hidden: bool,
}

pub fn run() -> i32 {
    // 整个程序只维护一个全局 `App` 实例。
    // 入口负责创建它、运行主循环，并在退出时清空全局指针。
    unsafe {
        let hinstance = GetModuleHandleW(null());
        APP_INSTANCE = Some(App::new(hinstance));
        let exit_code = app().run_main();
        APP_INSTANCE = None;
        exit_code
    }
}

unsafe fn app() -> &'static mut App {
    // 主窗口过程和若干全局回调都会回到这里取当前应用状态。
    // 如果实例不存在，说明程序已经进入不可恢复的关闭阶段，直接终止进程。
    let app_instance = std::ptr::addr_of_mut!(APP_INSTANCE);
    if let Some(app) = (*app_instance).as_mut() {
        app
    } else {
        windows_sys::Win32::System::Threading::ExitProcess(0);
    }
}

unsafe extern "system" fn perf_frame_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    // 性能页里的“框架控件”需要自绘背景，否则图表重绘时容易出现撕裂和闪烁。
    match msg {
        WM_CREATE => {
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            SetWindowLongW(hwnd, GWL_STYLE, (style | WS_CLIPSIBLINGS) as i32);
            0
        }
        WM_ERASEBKGND => {
            let mut hdc = wparam as _;
            let mut region = null_mut();

            if wparam == 0 {
                region = CreateRectRgn(0, 0, 0, 0);
                if !region.is_null() {
                    GetUpdateRgn(hwnd, region, 1);
                    hdc = GetDCEx(hwnd, region, DCX_CACHE | DCX_CLIPSIBLINGS | DCX_INTERSECTRGN);
                }
            }

            if !hdc.is_null() {
                let mut client_rect = zeroed::<RECT>();
                GetClientRect(hwnd, &mut client_rect);
                FillRect(hdc, &client_rect, GetSysColorBrush(COLOR_3DFACE));
            }

            if wparam == 0 {
                if !hdc.is_null() {
                    ReleaseDC(hwnd, hdc);
                }
                if !region.is_null() {
                    DeleteObject(region as _);
                }
            }
            TRUE as isize
        }
        _ => {
            if let Some(base_wndproc) = FRAME_BASE_WNDPROC {
                CallWindowProcW(Some(base_wndproc), hwnd, msg, wparam, lparam)
            } else {
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
    }
}

type RunFileDialogFn =
    unsafe extern "system" fn(HWND, HICON, *const u16, *const u16, *const u16, u32) -> i32;

impl App {
    fn new(hinstance: HINSTANCE) -> Self {
        Self {
            hinstance,
            main_hwnd: null_mut(),
            status_hwnd: null_mut(),
            startup_mutex: null_mut(),
            accelerator_table: null_mut(),
            current_menu: null_mut(),
            tray_icons: Vec::new(),
            strings: GlobalStrings::default(),
            options: Options::default(),
            pages: default_pages(),
            stats: RuntimeStats::default(),
            framed_style: 0,
            borderless_style: 0,
            min_width: 0,
            min_height: 0,
            already_applied_initial_position: false,
            menu_tracking: false,
            cant_hide: false,
            in_popup: false,
            temporarily_hidden: false,
        }
    }

    unsafe fn run_main(&mut self) -> i32 {
        // 启动链路按“单实例检查 -> 环境初始化 -> 创建主对话框 -> 进入消息循环”展开。
        // 这样既能兼容经典 Task Manager 的行为，也便于在失败点提前退出。
        self.acquire_startup_mutex();
        if self.activate_existing_instance() {
            self.release_startup_mutex();
            return 0;
        }

        if self.task_manager_disabled() {
            self.release_startup_mutex();
            return 1;
        }

        self.initialize_common_controls();
        self.register_custom_controls();
        self.load_global_resources();
        self.stats.processor_count = self.query_processor_count();

        self.main_hwnd = CreateDialogParamW(
            self.hinstance,
            make_int_resource(IDD_MAINWND),
            null_mut(),
            Some(main_window_proc),
            0,
        );
        if self.main_hwnd.is_null() {
            self.release_startup_mutex();
            return 1;
        }

        self.already_applied_initial_position = true;
        let saved_rect = self.options.window_rect;
        if width(&saved_rect) > 0 && height(&saved_rect) > 0 {
            SetWindowPos(
                self.main_hwnd,
                null_mut(),
                saved_rect.left,
                saved_rect.top,
                width(&saved_rect),
                height(&saved_rect),
                SWP_NOZORDER,
            );
        }

        ShowWindow(self.main_hwnd, SW_SHOW);
        self.release_startup_mutex();
        SetProcessShutdownParameters(1, 0);

        let mut message = zeroed::<MSG>();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            let page_hwnd = if self.options.current_page >= 0 {
                self.pages[self.options.current_page as usize].hwnd()
            } else {
                null_mut()
            };

            let mut handled = !self.accelerator_table.is_null()
                && TranslateAcceleratorW(self.main_hwnd, self.accelerator_table, &mut message) != 0;

            if !handled && !page_hwnd.is_null() && !self.accelerator_table.is_null() {
                handled = TranslateAcceleratorW(page_hwnd, self.accelerator_table, &mut message) != 0;
            }

            if !handled && IsDialogMessageW(self.main_hwnd, &mut message) == 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }

        message.wParam as i32
    }

    unsafe fn acquire_startup_mutex(&mut self) {
        // 命名互斥体用于串行化启动窗口，避免两个实例同时完成“是否已有实例”的判断。
        let mutex_name = to_wide_null(STARTUP_MUTEX_NAME);
        self.startup_mutex = CreateMutexW(null_mut(), TRUE, mutex_name.as_ptr());
        if !self.startup_mutex.is_null()
            && windows_sys::Win32::Foundation::GetLastError() == ERROR_ALREADY_EXISTS
        {
            WaitForSingleObject(self.startup_mutex, FINDME_TIMEOUT);
        }
    }

    unsafe fn release_startup_mutex(&mut self) {
        // 一旦主窗口已经创建或确认无需继续启动，就及时释放互斥体，避免阻塞后续实例探测。
        if !self.startup_mutex.is_null() {
            ReleaseMutex(self.startup_mutex);
            CloseHandle(self.startup_mutex);
            self.startup_mutex = null_mut();
        }
    }

    unsafe fn activate_existing_instance(&self) -> bool {
        // 与历史版本一致，靠主窗口标题找到已运行实例，并通过自定义消息把它激活到前台。
        let title = load_string(self.hinstance, IDS_APPTITLE);
        if title.is_empty() {
            return false;
        }

        let title_wide = to_wide_null(&title);
        let existing_hwnd = FindWindowW(null(), title_wide.as_ptr());
        if existing_hwnd.is_null() {
            return false;
        }

        let mut result = 0usize;
        SendMessageTimeoutW(
            existing_hwnd,
            PWM_ACTIVATE,
            0,
            0,
            SMTO_ABORTIFHUNG,
            FINDME_TIMEOUT,
            &mut result,
        ) != 0
            && result as u32 == PWM_ACTIVATE
    }

    unsafe fn task_manager_disabled(&self) -> bool {
        // 企业策略或系统策略可能禁用 Task Manager。
        // 这里在真正启动 UI 前读取策略位，并按系统工具习惯弹出阻止提示。
        let policy_key = to_wide_null("Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\System");
        let value_name = to_wide_null("DisableTaskMgr");
        let mut key: HKEY = null_mut();

        if RegOpenKeyExW(HKEY_CURRENT_USER, policy_key.as_ptr(), 0, KEY_READ, &mut key) != 0 {
            return false;
        }

        let mut value_type = 0u32;
        let mut raw_value = 0u32;
        let mut raw_size = size_of::<u32>() as u32;
        let status = RegQueryValueExW(
            key,
            value_name.as_ptr(),
            null_mut(),
            &mut value_type,
            &mut raw_value as *mut u32 as *mut u8,
            &mut raw_size,
        );
        RegCloseKey(key);

        if status == 0 && raw_value != 0 {
            let title = to_wide_null(&load_string(self.hinstance, IDS_TASKMGR));
            let body = to_wide_null(&load_string(self.hinstance, IDS_TASKMGRDISABLED));
            MessageBoxW(null_mut(), body.as_ptr(), title.as_ptr(), MB_OK | MB_ICONSTOP);
            true
        } else {
            false
        }
    }

    unsafe fn initialize_common_controls(&self) {
        // 页面里依赖 Tab、ListView、StatusBar 等公共控件类，必须在创建前统一注册。
        let mut classes = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LISTVIEW_CLASSES | ICC_TAB_CLASSES | ICC_BAR_CLASSES,
        };
        InitCommonControlsEx(&mut classes);
    }

    unsafe fn load_global_resources(&mut self) {
        // 这些资源会被菜单、状态栏和托盘图标反复使用，启动时一次性加载可以减少分散的 API 调用。
        self.accelerator_table = LoadAcceleratorsW(self.hinstance, make_int_resource(IDR_ACCELERATORS));
        self.strings.app_title = load_string(self.hinstance, IDS_APPTITLE);
        self.strings.fmt_procs = load_string(self.hinstance, IDS_FMTPROCS);
        self.strings.fmt_cpu = load_string(self.hinstance, IDS_FMTCPU);
        self.strings.fmt_mem = load_string(self.hinstance, IDS_FMTMEM);
        self.tray_icons.clear();
        for icon_id in TRAY_ICON_IDS {
            let icon_handle = LoadImageW(
                self.hinstance,
                make_int_resource(icon_id),
                IMAGE_ICON,
                0,
                0,
                LR_DEFAULTCOLOR | LR_DEFAULTSIZE,
            );
            self.tray_icons.push(icon_handle);
        }
    }

    unsafe fn query_processor_count(&self) -> u8 {
        let mut sysinfo = zeroed::<SYSTEM_INFO>();
        GetSystemInfo(&mut sysinfo);
        sysinfo.dwNumberOfProcessors as u8
    }

    unsafe fn on_init_dialog(&mut self, hwnd: HWND) -> isize {
        // 主对话框初始化会把“窗口样式、状态栏、标签页、托盘、定时器”全部串起来，
        // 这也是运行期状态第一次与持久化配置合流的地方。
        self.main_hwnd = hwnd;
        localize_dialog(hwnd, IDD_MAINWND);

        let mut window_rect = zeroed::<RECT>();
        GetWindowRect(hwnd, &mut window_rect);
        self.min_width = width(&window_rect);
        self.min_height = height(&window_rect);
        self.framed_style = framed_window_style(GetWindowLongW(hwnd, GWL_STYLE) as u32);
        self.borderless_style = borderless_window_style(self.framed_style);

        self.options.load(self.min_width, self.min_height);

        if self.options.always_on_top() {
            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        }

        self.create_status_bar();

        // Set status bar TOPMOST so it doesn't slide under the tab control
        if !self.status_hwnd.is_null() {
            SetWindowPos(
                self.status_hwnd,
                HWND_TOPMOST,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOREDRAW,
            );
        }

        self.set_window_title();

        let icon_handle = LoadIconW(self.hinstance, make_int_resource(IDI_MAIN));
        if !icon_handle.is_null() {
            SendMessageW(hwnd, WM_SETICON, 1, icon_handle as LPARAM);
        }

        if let Some(first_icon) = self.tray_icons.first().copied() {
            self.update_tray(NIM_ADD, first_icon, "");
        }

        let tabs_hwnd = GetDlgItem(hwnd, IDC_TABS);
        for (index, page) in self.pages.iter_mut().enumerate() {
            if let Err(error) = page.initialize(
                self.hinstance,
                self.main_hwnd,
                tabs_hwnd,
                self.stats.processor_count as usize,
            ) {
                let title = to_wide_null(&self.strings.app_title);
                let message = to_wide_null(&format!("Failed to initialize page {} (Win32 error {}).", index, error));
                MessageBoxW(hwnd, message.as_ptr(), title.as_ptr(), MB_OK | MB_ICONSTOP);
                return 0;
            }

            let title = page.title(self.hinstance);
            let mut title_wide = to_wide_null(&title);
            let mut item = windows_sys::Win32::UI::Controls::TCITEMW {
                mask: windows_sys::Win32::UI::Controls::TCIF_TEXT,
                dwState: 0,
                dwStateMask: 0,
                pszText: title_wide.as_mut_ptr(),
                cchTextMax: title_wide.len() as i32,
                iImage: 0,
                lParam: 0,
            };

            SendMessageW(tabs_hwnd, TCM_INSERTITEMW, index, &mut item as *mut _ as LPARAM);
        }

        self.update_menu_states();
        if self.options.current_page < 0 {
            self.options.current_page = 0;
        }

        SendMessageW(tabs_hwnd, TCM_SETCURSEL, self.options.current_page as usize, 0);
        let _ = self.activate_page(self.options.current_page as usize);

        let mut client_rect = zeroed::<RECT>();
        GetClientRect(hwnd, &mut client_rect);
        self.on_size(hwnd, 0, width(&client_rect), height(&client_rect));

        if self.options.timer_interval != 0 {
            SetTimer(hwnd, 0, self.options.timer_interval, None);
        }

        self.on_timer(hwnd);

        if self.stats.processor_count <= 1 {
            let menu = GetMenu(hwnd);
            if !menu.is_null() {
                EnableMenuItem(menu, IDM_MULTIGRAPH as u32, MF_BYCOMMAND | MF_GRAYED);
            }
        }

        1
    }

    unsafe fn create_status_bar(&mut self) {
        self.status_hwnd = CreateWindowExW(
            0,
            STATUSCLASSNAMEW,
            null(),
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | SBARS_SIZEGRIP,
            0,
            0,
            0,
            0,
            self.main_hwnd,
            IDC_STATUSWND as usize as HMENU,
            self.hinstance,
            null_mut(),
        );

        let hdc = GetDC(null_mut());
        let pixels_per_inch = GetDeviceCaps(hdc, LOGPIXELSX as i32);
        ReleaseDC(null_mut(), hdc);

        let parts = [
            pixels_per_inch,
            pixels_per_inch + (pixels_per_inch * 5) / 4,
            pixels_per_inch + (pixels_per_inch * 15) / 4,
            -1,
        ];
        SendMessageW(
            self.status_hwnd,
            SB_SETPARTS,
            parts.len(),
            parts.as_ptr() as LPARAM,
        );
    }

    unsafe fn register_custom_controls(&self) {
        // 性能页的 frame 控件借用了 Button 类的外观，但需要自定义背景擦除过程来降低闪烁。
        let mut button_class = zeroed::<WNDCLASSW>();
        let button_name = to_wide_null(BUTTON_CLASS);
        if GetClassInfoW(null_mut(), button_name.as_ptr(), &mut button_class) == 0 {
            return;
        }

        FRAME_BASE_WNDPROC = button_class.lpfnWndProc;
        button_class.hInstance = self.hinstance;
        button_class.lpfnWndProc = Some(perf_frame_wndproc);
        let class_name = to_wide_null(PERF_FRAME_CLASS_NAME);
        button_class.lpszClassName = class_name.as_ptr();
        let _ = RegisterClassW(&button_class);
    }

    unsafe fn set_window_title(&self) {
        let title = to_wide_null(&self.strings.app_title);
        SetWindowTextW(self.main_hwnd, title.as_ptr());
    }

    unsafe fn activate_page(&mut self, index: usize) -> bool {
        // 切页不仅是隐藏/显示子对话框，还要同步菜单、页面选项和尺寸布局。
        // 如果新页面激活失败，会尽量恢复上一个页面，避免主窗口进入空白状态。
        if index >= self.pages.len() {
            return false;
        }

        let previous_page = self.options.current_page;
        let switching_pages = previous_page >= 0 && previous_page as usize != index;

        if switching_pages {
            self.pages[previous_page as usize].deactivate(&mut self.options);
        }

        if self.pages[index]
            .activate(
                self.hinstance,
                self.main_hwnd,
                &self.options,
                self.stats.processor_count as usize,
                &mut self.current_menu,
            )
            .is_ok()
        {
            self.options.current_page = index as i32;
            self.sync_performance_page();
            self.update_menu_states();
            self.size_active_page();
            true
        } else {
            if switching_pages {
                let previous_index = previous_page as usize;
                let _ = self.pages[previous_index].activate(
                    self.hinstance,
                    self.main_hwnd,
                    &self.options,
                    self.stats.processor_count as usize,
                    &mut self.current_menu,
                );
                self.options.current_page = previous_page;
                let tabs_hwnd = GetDlgItem(self.main_hwnd, IDC_TABS);
                if !tabs_hwnd.is_null() {
                    SendMessageW(tabs_hwnd, TCM_SETCURSEL, previous_index, 0);
                }
                self.update_menu_states();
                self.size_active_page();
            }
            false
        }
    }

    unsafe fn update_menu_states(&self) {
        // 菜单状态完全由 `options` 和当前页状态派生，每次切页/改选项后都重新同步，
        // 避免菜单勾选与真实行为脱节。
        let menu = GetMenu(self.main_hwnd);
        if menu.is_null() {
            return;
        }

        sanitize_task_manager_menu(menu, self.stats.processor_count as usize);

        CheckMenuRadioItem(
            menu,
            VM_FIRST as u32,
            VM_LAST as u32,
            (VM_FIRST + self.options.view_mode as u16) as u32,
            MF_BYCOMMAND,
        );
        CheckMenuRadioItem(
            menu,
            CM_FIRST as u32,
            CM_LAST as u32,
            (CM_FIRST + self.options.cpu_history_mode as u16) as u32,
            MF_BYCOMMAND,
        );
        CheckMenuRadioItem(
            menu,
            US_FIRST as u32,
            US_LAST as u32,
            (US_FIRST + self.options.update_speed as u16) as u32,
            MF_BYCOMMAND,
        );

        self.check_menu(menu, IDM_ALWAYSONTOP, self.options.always_on_top());
        self.check_menu(menu, IDM_MINIMIZEONUSE, self.options.minimize_on_use());
        self.check_menu(menu, IDM_CONFIRMATIONS, self.options.confirmations());
        self.check_menu(menu, IDM_KERNELTIMES, self.options.kernel_times());
        self.check_menu(menu, IDM_NOTITLE, self.options.no_title());
        self.check_menu(menu, IDM_HIDEWHENMIN, self.options.hide_when_minimized());
        if self.options.current_page == USER_PAGE as i32 {
            self.check_menu(
                menu,
                IDM_SHOWDOMAINNAMES,
                self.pages[USER_PAGE].user_show_domain_names().unwrap_or(false),
            );
        }

        EnableMenuItem(
            menu,
            IDM_MULTIGRAPH as u32,
            MF_BYCOMMAND
                | if self.stats.processor_count <= 1 {
                    MF_GRAYED
                } else {
                    MF_ENABLED
                },
        );
    }

    unsafe fn check_menu(&self, menu: HMENU, item_id: u16, checked: bool) {
        CheckMenuItem(
            menu,
            item_id as u32,
            MF_BYCOMMAND | if checked { MF_CHECKED } else { MF_UNCHECKED },
        );
    }

    unsafe fn sync_performance_page(&mut self) {
        self.pages[PERF_PAGE].apply_options(&self.options, self.stats.processor_count as usize);
    }

    unsafe fn apply_options_to_pages(&mut self) {
        for page in self.pages.iter_mut() {
            page.apply_options(&self.options, self.stats.processor_count as usize);
        }
    }

    unsafe fn refresh_task_page(&mut self) {
        self.pages[TASK_PAGE].apply_options(&self.options, self.stats.processor_count as usize);
        self.pages[TASK_PAGE].timer_event(&self.options, self.stats.processor_count as usize);
    }

    unsafe fn refresh_performance_page(&mut self) {
        self.pages[PERF_PAGE].apply_options(&self.options, self.stats.processor_count as usize);
        self.pages[PERF_PAGE].timer_event(&self.options, self.stats.processor_count as usize);
    }

    unsafe fn size_active_page(&mut self) {
        // 无标题模式和普通模式的布局入口不同：
        // 前者让活动页直接占满主窗口客户区，后者则受 Tab 控件内容区约束。
        if self.options.current_page < 0 {
            return;
        }

        let active_hwnd = self.pages[self.options.current_page as usize].hwnd();
        if active_hwnd.is_null() {
            return;
        }

        if self.options.no_title() {
            let mut client_rect = zeroed::<RECT>();
            GetClientRect(self.main_hwnd, &mut client_rect);
            SetWindowPos(
                active_hwnd,
                null_mut(),
                client_rect.left,
                client_rect.top,
                width(&client_rect),
                height(&client_rect),
                SWP_NOZORDER | SWP_NOACTIVATE,
            );

            // Compute borderless style from live window style (matching C++ behavior)
            let current_style = GetWindowLongW(self.main_hwnd, GWL_STYLE) as u32;
            let live_borderless = current_style & !(WS_DLGFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX);
            set_style(self.main_hwnd, live_borderless);
            SetMenu(self.main_hwnd, null_mut());
            SetWindowPos(
                self.main_hwnd,
                null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
            DrawMenuBar(self.main_hwnd);
        } else {
            // Compute framed style from live window style (matching C++ behavior)
            let current_style = GetWindowLongW(self.main_hwnd, GWL_STYLE) as u32;
            let live_framed = framed_window_style(current_style);
            set_style(self.main_hwnd, live_framed);

            if !self.current_menu.is_null() {
                SetMenu(self.main_hwnd, self.current_menu);
                self.update_menu_states();
            }
            SetWindowPos(
                self.main_hwnd,
                null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
            DrawMenuBar(self.main_hwnd);
            self.set_window_title();

            let tabs_hwnd = GetDlgItem(self.main_hwnd, IDC_TABS);
            let tabs_rect = adjusted_tab_page_rect(tabs_hwnd, self.main_hwnd);
            SetWindowPos(
                active_hwnd,
                null_mut(),
                tabs_rect.left,
                tabs_rect.top,
                width(&tabs_rect),
                height(&tabs_rect),
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    unsafe fn on_size(&mut self, hwnd: HWND, state: u32, width_px: i32, height_px: i32) {
        if state == SIZE_MINIMIZED && self.options.hide_when_minimized() && !GetShellWindow().is_null() {
            ShowWindow(hwnd, SW_HIDE);
        }

        if !self.status_hwnd.is_null() {
            SendMessageW(self.status_hwnd, WM_SIZE, state as usize, 0);
        }

        let tabs_hwnd = GetDlgItem(hwnd, IDC_TABS);
        if !tabs_hwnd.is_null() && !self.status_hwnd.is_null() {
            let mut status_rect = zeroed::<RECT>();
            GetClientRect(self.status_hwnd, &mut status_rect);
            MapWindowPoints(self.status_hwnd, self.main_hwnd, &mut status_rect as *mut _ as _, 2);

            let mut tabs_rect = zeroed::<RECT>();
            GetWindowRect(tabs_hwnd, &mut tabs_rect);
            MapWindowPoints(null_mut(), self.main_hwnd, &mut tabs_rect as *mut _ as _, 2);

            let adjusted_width = width_px - 2 * tabs_rect.left;
            let adjusted_height = height_px - (height_px - status_rect.top) - tabs_rect.top * 2;
            SetWindowPos(
                tabs_hwnd,
                null_mut(),
                tabs_rect.left,
                tabs_rect.top,
                adjusted_width,
                adjusted_height,
                SWP_NOZORDER,
            );
        }

        self.size_active_page();
    }

    unsafe fn on_timer(&mut self, hwnd: HWND) {
        // 按住 Ctrl 时暂停自动刷新，这与经典 Task Manager 的交互保持一致。
        if GetForegroundWindow() == hwnd && GetAsyncKeyState(VK_CONTROL as i32) < 0 {
            return;
        }

        for page in self.pages.iter_mut() {
            page.timer_event(&self.options, self.stats.processor_count as usize);
        }

        if let Some(snapshot) = self.pages[PERF_PAGE].performance_snapshot() {
            self.stats.cpu_usage = snapshot.cpu_usage;
            self.stats.mem_usage_kb = snapshot.mem_usage_kb;
            self.stats.mem_limit_kb = snapshot.mem_limit_kb;
            self.stats.process_count = snapshot.process_count;
        } else {
            self.refresh_runtime_stats();
        }

        self.refresh_tray_icon();
        self.refresh_status_bar();
    }

    unsafe fn refresh_runtime_stats(&mut self) {
        // 当性能页快照不可用时，主框架自己补采一份轻量级运行时统计，
        // 用于状态栏和托盘图标，不依赖页面是否处于激活状态。
        let mut idle = zeroed::<FILETIME>();
        let mut kernel = zeroed::<FILETIME>();
        let mut user = zeroed::<FILETIME>();
        if GetSystemTimes(&mut idle, &mut kernel, &mut user) != 0 {
            let idle_value = filetime_to_u64(idle);
            let kernel_value = filetime_to_u64(kernel);
            let user_value = filetime_to_u64(user);

            if self.stats.previous_idle != 0 {
                let delta_idle = idle_value.saturating_sub(self.stats.previous_idle);
                let delta_total = kernel_value
                    .saturating_sub(self.stats.previous_kernel)
                    .saturating_add(user_value.saturating_sub(self.stats.previous_user));

                if delta_total != 0 {
                    let active_ticks = delta_total.saturating_sub(delta_idle);
                    self.stats.cpu_usage = ((active_ticks * 100) / delta_total) as u8;
                }
            }

            self.stats.previous_idle = idle_value;
            self.stats.previous_kernel = kernel_value;
            self.stats.previous_user = user_value;
        }

        let mut memory = MEMORYSTATUSEX {
            dwLength: size_of::<MEMORYSTATUSEX>() as u32,
            ..zeroed()
        };
        if GlobalMemoryStatusEx(&mut memory) != 0 {
            self.stats.mem_usage_kb = ((memory.ullTotalPhys - memory.ullAvailPhys) / 1024) as u32;
            self.stats.mem_limit_kb = (memory.ullTotalPhys / 1024) as u32;
        }

        self.stats.process_count = process_count();
    }

    unsafe fn refresh_status_bar(&self) {
        if self.status_hwnd.is_null() || self.menu_tracking {
            return;
        }

        let process_text = format_resource_string(
            &self.strings.fmt_procs,
            &[self.stats.process_count.to_string()],
        );
        let cpu_text = format_resource_string(&self.strings.fmt_cpu, &[self.stats.cpu_usage.to_string()]);
        let mem_text = format_resource_string(
            &self.strings.fmt_mem,
            &[
                self.stats.mem_usage_kb.to_string(),
                self.stats.mem_limit_kb.to_string(),
            ],
        );

        let process_wide = to_wide_null(&process_text);
        let cpu_wide = to_wide_null(&cpu_text);
        let mem_wide = to_wide_null(&mem_text);

        SendMessageW(self.status_hwnd, SB_SETTEXTW, 0, process_wide.as_ptr() as LPARAM);
        SendMessageW(self.status_hwnd, SB_SETTEXTW, 1, cpu_wide.as_ptr() as LPARAM);
        SendMessageW(self.status_hwnd, SB_SETTEXTW, 2, mem_wide.as_ptr() as LPARAM);
    }

    unsafe fn update_tray(&self, command: u32, icon: HICON, tip: &str) {
        let mut data = zeroed::<NOTIFYICONDATAW>();
        data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = self.main_hwnd;
        data.uID = PWM_TRAYICON;
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = PWM_TRAYICON;
        data.hIcon = icon;

        let tip_wide = to_wide_null(tip);
        for (index, code_unit) in tip_wide.iter().copied().enumerate() {
            if index >= NOTIFY_ICON_TIP_CAPACITY {
                break;
            }
            data.szTip[index] = code_unit;
        }

        Shell_NotifyIconW(command, &mut data);
    }

    unsafe fn refresh_tray_icon(&self) {
        // 托盘图标按 CPU 使用率映射到离散图标序列，行为上尽量贴近经典任务管理器。
        if self.tray_icons.is_empty() {
            return;
        }

        let mut icon_index = (self.stats.cpu_usage as usize * self.tray_icons.len()) / 100;
        if icon_index >= self.tray_icons.len() {
            icon_index = self.tray_icons.len() - 1;
        }

        let tooltip = format_resource_string(&self.strings.fmt_cpu, &[self.stats.cpu_usage.to_string()]);
        self.update_tray(NIM_MODIFY, self.tray_icons[icon_index], &tooltip);
    }

    unsafe fn show_running_instance(&self) {
        OpenIcon(self.main_hwnd);
        SetForegroundWindow(self.main_hwnd);
        SetWindowPos(
            self.main_hwnd,
            if self.options.always_on_top() {
                HWND_TOPMOST
            } else {
                HWND_TOP
            },
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE,
        );
    }

    unsafe fn load_popup_menu(&self, resource_id: u16) -> HMENU {
        let menu = LoadMenuW(self.hinstance, make_int_resource(resource_id));
        if menu.is_null() {
            return null_mut();
        }
        localize_menu(menu, resource_id);

        let popup = windows_sys::Win32::UI::WindowsAndMessaging::GetSubMenu(menu, 0);
        RemoveMenu(menu, 0, MF_BYPOSITION);
        DestroyMenu(menu);
        sanitize_task_manager_menu(popup, self.stats.processor_count as usize);
        popup
    }

    unsafe fn on_tray_notification(&mut self, lparam: LPARAM) {
        // 托盘图标承担“恢复窗口”和“快速菜单”两个入口，所以这里单独处理鼠标消息。
        match lparam as u32 {
            windows_sys::Win32::UI::WindowsAndMessaging::WM_LBUTTONDBLCLK => self.show_running_instance(),
            WM_RBUTTONDOWN => {
                let popup = self.load_popup_menu(IDR_TRAYMENU);
                if !popup.is_null() {
                    let mut cursor = zeroed::<POINT>();
                    GetCursorPos(&mut cursor);

                    if IsWindowVisible(self.main_hwnd) != 0 {
                        DeleteMenu(popup, IDM_RESTORETASKMAN as u32, MF_BYCOMMAND);
                    } else {
                        SetMenuDefaultItem(popup, IDM_RESTORETASKMAN as u32, 0);
                    }

                    self.check_menu(popup, IDM_ALWAYSONTOP, self.options.always_on_top());
                    SetForegroundWindow(self.main_hwnd);
                    self.in_popup = true;
                    let command = TrackPopupMenuEx(
                        popup,
                        TPM_RETURNCMD,
                        cursor.x,
                        cursor.y,
                        self.main_hwnd,
                        null(),
                    );
                    self.in_popup = false;
                    if command != 0 {
                        SendMessageW(self.main_hwnd, WM_COMMAND, command as usize, 0);
                    }
                    DestroyMenu(popup);
                }
            }
            _ => {}
        }
    }

    unsafe fn show_help(&self, hwnd: HWND) {
        let help_path = to_wide_null("taskmgr.hlp");
        WinHelpW(hwnd, help_path.as_ptr(), HELP_FINDER, 0);
    }

    unsafe fn on_menu_select(&mut self, wparam: WPARAM, lparam: LPARAM) -> isize {
        // 菜单高亮时，状态栏会临时切到“帮助文本”模式；
        // 退出菜单跟踪后，再恢复回实时统计栏。
        if self.status_hwnd.is_null() {
            return 0;
        }

        let mut item_id = loword(wparam) as u32;
        let flags = hiword(wparam) as u32;
        let menu = lparam as HMENU;

        if (item_id == 0xFFFF && menu.is_null()) || (flags & (MF_SYSMENU | MF_SEPARATOR)) != 0 {
            self.menu_tracking = false;
            self.cant_hide = false;
            SendMessageW(self.status_hwnd, SB_SIMPLE, 0, 0);
            self.refresh_status_bar();
            return 0;
        }

        if (flags & MF_POPUP) != 0 && !menu.is_null() {
            let mut submenu_info = MENUITEMINFOW {
                cbSize: size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_ID,
                ..zeroed()
            };
            if GetMenuItemInfoW(menu, item_id, 1, &mut submenu_info) != 0 {
                item_id = submenu_info.wID;
            }
        }

        let status_text = load_string(self.hinstance, item_id);
        let status_wide = to_wide_null(&status_text);
        self.menu_tracking = true;
        SendMessageW(
            self.status_hwnd,
            SB_SETTEXTW,
            (SBT_NOBORDERS | SB_SIMPLEID) as usize,
            status_wide.as_ptr() as LPARAM,
        );
        SendMessageW(self.status_hwnd, SB_SIMPLE, 1, 0);
        SendMessageW(
            self.status_hwnd,
            SB_SETTEXTW,
            SBT_NOBORDERS as usize,
            status_wide.as_ptr() as LPARAM,
        );
        0
    }

    unsafe fn on_init_menu(&mut self) -> isize {
        self.cant_hide = true;
        0
    }

    unsafe fn on_popup_state(&mut self, active: bool) -> isize {
        self.in_popup = active;
        0
    }

    unsafe fn on_right_button_down(&mut self, hwnd: HWND) -> isize {
        if !self.in_popup
            && !self.temporarily_hidden
            && !self.cant_hide
            && self.options.always_on_top()
        {
            ShowWindow(hwnd, SW_HIDE);
            SetCapture(hwnd);
            self.temporarily_hidden = true;
        }
        0
    }

    unsafe fn on_right_button_up(&mut self, hwnd: HWND) -> isize {
        if self.temporarily_hidden {
            ReleaseCapture();
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_SHOWMINNOACTIVE);
            } else if IsZoomed(hwnd) != 0 {
                ShowWindow(hwnd, SW_SHOWMAXIMIZED);
                SetForegroundWindow(hwnd);
            } else {
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                SetForegroundWindow(hwnd);
            }
            self.temporarily_hidden = false;
        }
        0
    }

    unsafe fn show_run_dialog(&self) -> bool {
        // 新建任务对话框复用 shell32 导出的 RunFileDlg，
        // 这样能得到与系统一致的“运行”体验，而不是自造一个近似实现。
        let shell32_name = to_wide_null("shell32.dll");
        let shell32 = LoadLibraryW(shell32_name.as_ptr());
        if shell32.is_null() {
            return false;
        }

        let run_file_dlg = match GetProcAddress(shell32, 61usize as *const u8) {
            Some(proc_address) => transmute::<unsafe extern "system" fn() -> isize, RunFileDialogFn>(proc_address),
            None => {
                FreeLibrary(shell32);
                return false;
            }
        };

        let mut current_dir = to_wide_null(&env::current_dir().unwrap_or_default().to_string_lossy());
        let mut title = to_wide_null(&load_string(self.hinstance, IDS_RUNTITLE));
        let mut prompt = to_wide_null(&load_string(self.hinstance, IDS_RUNTEXT));
        let icon = LoadImageW(
            self.hinstance,
            make_int_resource(IDI_MAIN),
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTCOLOR | LR_DEFAULTSIZE,
        );

        let shown = if !icon.is_null() {
            run_file_dlg(
                self.main_hwnd,
                icon,
                current_dir.as_mut_ptr(),
                title.as_mut_ptr(),
                prompt.as_mut_ptr(),
                RUN_DIALOG_CALC_DIRECTORY,
            );
            DestroyIcon(icon);
            true
        } else {
            false
        };

        FreeLibrary(shell32);
        shown
    }

    unsafe fn on_command(&mut self, hwnd: HWND, command_id: u16) {
        // 主命令分发层只负责修改全局选项、切页和把页面专属命令转发到对应子页面。
        // 真正的进程/任务/用户操作都在各自页面状态对象里完成。
        match command_id {
            IDM_HIDE => {
                ShowWindow(hwnd, SW_MINIMIZE);
            }
            id if id == IDCANCEL as u16 || id == IDM_EXIT => {
                DestroyWindow(hwnd);
            }
            IDM_RESTORETASKMAN => {
                self.show_running_instance();
            }
            IDC_NEXTTAB | IDC_PREVTAB => {
                self.switch_tabs(command_id == IDC_NEXTTAB);
            }
            IDM_ALWAYSONTOP => {
                let always_on_top = !self.options.always_on_top();
                self.options.set_always_on_top(always_on_top);
                SetWindowPos(
                    hwnd,
                    if always_on_top {
                        HWND_TOPMOST
                    } else {
                        HWND_NOTOPMOST
                    },
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE,
                );
                self.update_menu_states();
            }
            IDM_HIDEWHENMIN => {
                self.options
                    .set_hide_when_minimized(!self.options.hide_when_minimized());
                self.update_menu_states();
            }
            IDM_MINIMIZEONUSE => {
                self.options
                    .set_minimize_on_use(!self.options.minimize_on_use());
                self.update_menu_states();
            }
            IDM_CONFIRMATIONS => {
                self.options
                    .set_confirmations(!self.options.confirmations());
                self.update_menu_states();
            }
            IDM_NOTITLE => {
                self.options.set_no_title(!self.options.no_title());
                self.apply_options_to_pages();
                self.update_menu_states();
                self.size_active_page();
            }
            IDM_KERNELTIMES => {
                self.options.set_kernel_times(!self.options.kernel_times());
                self.refresh_performance_page();
                self.update_menu_states();
            }
            IDM_LARGEICONS | IDM_SMALLICONS | IDM_DETAILS => {
                self.options.view_mode = (command_id - VM_FIRST) as i32;
                self.update_menu_states();
                self.refresh_task_page();
            }
            IDM_ALLCPUS | IDM_MULTIGRAPH => {
                self.options.cpu_history_mode = (command_id - CM_FIRST) as i32;
                self.refresh_performance_page();
                self.update_menu_states();
            }
            IDM_HIGH | IDM_NORMAL | IDM_LOW | IDM_PAUSED => {
                const TIMER_DELAYS: [u32; 4] = [500, 2000, 4000, 0];

                self.options.update_speed = (command_id - US_FIRST) as i32;
                let timer_delay = TIMER_DELAYS[self.options.update_speed as usize];
                self.options.timer_interval = timer_delay;

                KillTimer(self.main_hwnd, 0);
                if timer_delay != 0 {
                    SetTimer(self.main_hwnd, 0, timer_delay, None);
                }

                self.update_menu_states();
            }
            IDM_REFRESH => {
                self.on_timer(hwnd);
            }
            IDM_ABOUT => {
                let title = to_wide_null(&self.strings.app_title);
                let icon = LoadIconW(self.hinstance, make_int_resource(IDI_MAIN));
                if !icon.is_null() {
                    ShellAboutW(hwnd, title.as_ptr(), null(), icon);
                    DestroyIcon(icon);
                }
            }
            IDM_TASK_CASCADE
            | IDM_TASK_MINIMIZE
            | IDM_TASK_MAXIMIZE
            | IDM_TASK_TILEHORZ
            | IDM_TASK_TILEVERT
            | IDM_TASK_BRINGTOFRONT => {
                let task_hwnd = self.pages[TASK_PAGE].hwnd();
                if !task_hwnd.is_null() {
                    SendMessageW(task_hwnd, WM_COMMAND, command_id as usize, 0);
                }
            }
            IDM_PROCCOLS
            | IDM_AFFINITY
            | IDM_PROC_DEBUG
            | IDM_PROC_TERMINATE
            | IDM_PROC_REALTIME
            | IDM_PROC_HIGH
            | IDM_PROC_NORMAL
            | IDM_PROC_LOW => {
                if self.options.current_page == PROC_PAGE as i32 {
                    let _ = self.pages[PROC_PAGE]
                        .handle_process_command(command_id, Some(&mut self.options));
                } else {
                    MessageBeep(0);
                }
            }
            IDM_SHOWDOMAINNAMES | IDM_SENDMESSAGE | IDM_DISCONNECT | IDM_LOGOFF => {
                if self.options.current_page == USER_PAGE as i32 {
                    let handled = self.pages[USER_PAGE].handle_user_command(command_id);
                    if handled {
                        self.update_menu_states();
                    }
                } else {
                    MessageBeep(0);
                }
            }
            IDM_RUN => {
                let _ = self.show_run_dialog();
            }
            IDM_HELP => {
                self.show_help(hwnd);
            }
            _ => {}
        }
    }

    unsafe fn switch_tabs(&mut self, move_forward: bool) {
        let current_index = self.options.current_page.max(0) as usize;
        let next_index = if move_forward {
            (current_index + 1) % self.pages.len()
        } else if current_index == 0 {
            self.pages.len() - 1
        } else {
            current_index - 1
        };

        let tabs_hwnd = GetDlgItem(self.main_hwnd, IDC_TABS);
        SendMessageW(tabs_hwnd, TCM_SETCURSEL, next_index, 0);
        let _ = self.activate_page(next_index);
    }

    unsafe fn record_window_rect(&mut self, hwnd: HWND) {
        // 只有在初始位置已经应用过之后，后续移动/缩放才应该回写配置，
        // 否则会把对话框默认位置误记成用户偏好。
        if !self.already_applied_initial_position {
            return;
        }

        let mut placement = WINDOWPLACEMENT {
            length: size_of::<WINDOWPLACEMENT>() as u32,
            ..zeroed()
        };
        if GetWindowPlacement(hwnd, &mut placement) != 0 {
            self.options.window_rect = placement.rcNormalPosition;
        }
    }

    unsafe fn on_notify(&mut self, lparam: LPARAM) -> isize {
        let header = &*(lparam as *const NMHDR);
        if header.idFrom as i32 == IDC_TABS && header.code == TCN_SELCHANGE as u32 {
            let tabs_hwnd = GetDlgItem(self.main_hwnd, IDC_TABS);
            let selected = SendMessageW(tabs_hwnd, TCM_GETCURSEL, 0, 0) as usize;
            return self.activate_page(selected) as isize;
        }

        0
    }

    unsafe fn on_find_process(&mut self, thread_id: u32, pid: u32) -> isize {
        // “转到进程”来自任务页，需要先切到进程页，再尝试把对应进程行选中并滚动到可见区域。
        let tabs_hwnd = GetDlgItem(self.main_hwnd, IDC_TABS);
        if tabs_hwnd.is_null() {
            MessageBeep(0);
            return 0;
        }

        SendMessageW(tabs_hwnd, TCM_SETCURSEL, PROC_PAGE, 0);
        if self.activate_page(PROC_PAGE) {
            self.pages[PROC_PAGE].find_process(thread_id, pid) as isize
        } else {
            MessageBeep(0);
            0
        }
    }

    unsafe fn shutdown(&mut self) {
        // 关闭顺序按“停定时器 -> 让页面保存状态 -> 销毁页面资源 -> 移除托盘 -> 写配置”执行，
        // 避免还在刷新的页面访问已经销毁的窗口或句柄。
        KillTimer(self.main_hwnd, 0);

        if self.options.current_page >= 0 {
            self.pages[self.options.current_page as usize].deactivate(&mut self.options);
        }
        for page in self.pages.iter_mut() {
            page.destroy();
        }

        self.update_tray(NIM_DELETE, null_mut(), "");
        let _ = self.options.save();

        if !self.current_menu.is_null() {
            DestroyMenu(self.current_menu);
            self.current_menu = null_mut();
        }

        PostQuitMessage(0);
    }
}

unsafe fn adjusted_tab_page_rect(tabs_hwnd: HWND, owner_hwnd: HWND) -> RECT {
    // Tab 控件的客户区需要通过 `TCM_ADJUSTRECT` 扣掉页签边框后，才能得到真正的页面矩形。
    let mut page_rect = zeroed::<RECT>();
    GetClientRect(tabs_hwnd, &mut page_rect);
    SendMessageW(tabs_hwnd, TCM_ADJUSTRECT, 0, &mut page_rect as *mut _ as LPARAM);
    MapWindowPoints(tabs_hwnd, owner_hwnd, &mut page_rect as *mut _ as _, 2);
    page_rect
}

fn framed_window_style(current_style: u32) -> u32 {
    let preserved_style_bits = current_style & !(WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_DLGFRAME);
    preserved_style_bits | WS_TILEDWINDOW
}

fn borderless_window_style(framed_style: u32) -> u32 {
    framed_style & !(WS_DLGFRAME | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX)
}

unsafe extern "system" fn main_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    // 主窗口过程只做最薄的一层 Win32 消息路由，
    // 具体行为统一委托给 `App`，避免消息逻辑散落在全局回调里。
    let application = app();

    if msg == WM_SIZE || msg == WM_MOVE {
        application.record_window_rect(hwnd);
    }

    match msg {
        WM_INITDIALOG => application.on_init_dialog(hwnd),
        WM_SIZE => {
            let width_px = (lparam & 0xFFFF) as i32;
            let height_px = ((lparam >> 16) & 0xFFFF) as i32;
            application.on_size(hwnd, wparam as u32, width_px, height_px);
            0
        }
        WM_TIMER => {
            application.on_timer(hwnd);
            0
        }
        WM_COMMAND => {
            application.on_command(hwnd, (wparam & 0xFFFF) as u16);
            0
        }
        WM_NOTIFY => application.on_notify(lparam),
        WM_MENUSELECT => application.on_menu_select(wparam, lparam),
        WM_INITMENU => application.on_init_menu(),
        WM_FINDPROC => application.on_find_process(wparam as u32, lparam as u32),
        PWM_INPOPUP => application.on_popup_state(wparam != 0),
        WM_GETMINMAXINFO => {
            if !application.options.no_title() {
                let info = &mut *(lparam as *mut MINMAXINFO);
                info.ptMinTrackSize.x = application.min_width;
                info.ptMinTrackSize.y = application.min_height;
            }
            0
        }
        PWM_TRAYICON => {
            application.on_tray_notification(lparam);
            0
        }
        PWM_ACTIVATE => {
            application.show_running_instance();
            PWM_ACTIVATE as isize
        }
        WM_NCHITTEST => {
            let mut result = DefWindowProcW(hwnd, msg, wparam, lparam);
            if application.options.no_title() && result == HTCLIENT as isize && IsZoomed(hwnd) == 0 {
                result = HTCAPTION as isize;
            }
            set_dialog_msg_result(hwnd, result);
            1
        }
        WM_RBUTTONDOWN | WM_NCRBUTTONDOWN => application.on_right_button_down(hwnd),
        WM_RBUTTONUP | WM_NCRBUTTONUP => application.on_right_button_up(hwnd),
        WM_NCLBUTTONDBLCLK => {
            // Only fall through to toggle no-title if we're already in no-title mode
            if !application.options.no_title() {
                return 0;
            }
            // Fall through to WM_LBUTTONDBLCLK logic
            application.options.set_no_title(!application.options.no_title());
            application.apply_options_to_pages();
            application.refresh_performance_page();
            application.update_menu_states();
            let mut window_rect = zeroed::<RECT>();
            GetWindowRect(hwnd, &mut window_rect);
            MoveWindow(
                hwnd,
                window_rect.left,
                window_rect.top,
                width(&window_rect),
                height(&window_rect),
                1,
            );
            application.size_active_page();
            0
        }
        WM_LBUTTONDBLCLK => {
            application.options.set_no_title(!application.options.no_title());
            application.apply_options_to_pages();
            application.refresh_performance_page();
            application.update_menu_states();
            let mut window_rect = zeroed::<RECT>();
            GetWindowRect(hwnd, &mut window_rect);
            MoveWindow(
                hwnd,
                window_rect.left,
                window_rect.top,
                width(&window_rect),
                height(&window_rect),
                1,
            );
            application.size_active_page();
            0
        }
        WM_ENDSESSION => {
            if wparam != 0 {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            application.shutdown();
            0
        }
        _ => 0,
    }
}

fn filetime_to_u64(filetime: FILETIME) -> u64 {
    ((filetime.dwHighDateTime as u64) << 32) | filetime.dwLowDateTime as u64
}

unsafe fn process_count() -> u32 {
    // ToolHelp 快照比逐进程句柄探测更便宜，足够支撑状态栏和托盘里的进程数量统计。
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snapshot == INVALID_HANDLE_VALUE {
        return 0;
    }

    let mut count = 0u32;
    let mut process_entry = zeroed::<PROCESSENTRY32W>();
    process_entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

    if Process32FirstW(snapshot, &mut process_entry) != 0 {
        count += 1;
        while Process32NextW(snapshot, &mut process_entry) != 0 {
            count += 1;
        }
    }

    CloseHandle(snapshot);
    count
}

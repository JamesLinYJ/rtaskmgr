use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, WPARAM};

// 页面宿主层。
// 该模块把资源对话框与各页面状态对象粘合起来，统一处理页面的创建、
// 激活、焦点切换、菜单切换以及 Win32 消息分发。

use windows_sys::Win32::Graphics::Gdi::{GetStockObject, BLACK_BRUSH};
use windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyMenu, DestroyWindow, DrawMenuBar, GetDlgCtrlID, GetDlgItem, GetWindowLongW,
    SendMessageW, SetMenu, SetWindowLongW, SetWindowPos, ShowWindow, GWL_STYLE, HMENU, HTCAPTION,
    SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOW, WM_COMMAND, WM_CONTEXTMENU, WM_CTLCOLORBTN,
    WM_DRAWITEM, WM_INITDIALOG, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEWHEEL,
    WM_NCLBUTTONDBLCLK, WM_NOTIFY, WM_SHOWWINDOW, WM_SIZE, WM_VSCROLL, WS_CLIPCHILDREN,
};

use crate::dialog_templates::create_dialog;
use crate::language::localize_dialog;
use crate::menus::build_main_menu;
use crate::netpage::NetworkPageState;
use crate::options::Options;
use crate::perfpage::{PerformancePageState, PerformanceSnapshot};
use crate::procpage::ProcessPageState;
use crate::resource::{
    IDC_CPUMETER, IDC_MEMGRAPH, IDC_MEMMETER, IDC_NICTOTALS, IDC_PROCLIST, IDC_TASKLIST,
    IDC_USERLIST, IDD_NETPAGE, IDD_PERFPAGE, IDD_PROCPAGE, IDD_TASKPAGE, IDD_USERSPAGE,
    IDR_MAINMENU_NET, IDR_MAINMENU_PERF, IDR_MAINMENU_PROC, IDR_MAINMENU_TASK, IDR_MAINMENU_USER,
    IDS_NETPAGETITLE, IDS_PERFPAGETITLE, IDS_PROCPAGETITLE, IDS_TASKPAGETITLE, IDS_USERPAGETITLE,
};
use crate::taskpage::TaskPageState;
use crate::userpage::UserPageState;
use crate::winutil::{get_window_userdata, load_string, set_window_userdata};

enum PageFocusTarget {
    // 每个页面激活后都有一个更自然的初始焦点目标。
    None,
    Tabs,
    Control(i32),
}

pub struct DialogPage {
    // `DialogPage` 是资源对话框与具体页面状态对象之间的适配层。
    // 不同页面共享同一套激活/隐藏/菜单切换流程，只把真正的业务状态交给子页面实现。
    hinstance: HINSTANCE,
    hwnd: HWND,
    hwnd_tabs: HWND,
    main_hwnd: HWND,
    dialog_id: u16,
    menu_id: u16,
    title_id: u32,
    initial_focus: PageFocusTarget,
    perf_state: Option<PerformancePageState>,
    proc_state: Option<ProcessPageState>,
    task_state: Option<TaskPageState>,
    net_state: Option<NetworkPageState>,
    user_state: Option<UserPageState>,
}

impl DialogPage {
    pub fn task_page() -> Self {
        // 页面构造阶段只声明类型和元数据，不创建真实窗口。
        Self {
            hinstance: null_mut(),
            hwnd: null_mut(),
            hwnd_tabs: null_mut(),
            main_hwnd: null_mut(),
            dialog_id: IDD_TASKPAGE,
            menu_id: IDR_MAINMENU_TASK,
            title_id: IDS_TASKPAGETITLE,
            initial_focus: PageFocusTarget::Control(IDC_TASKLIST),
            perf_state: None,
            proc_state: None,
            task_state: Some(TaskPageState::new()),
            net_state: None,
            user_state: None,
        }
    }

    pub fn process_page() -> Self {
        Self {
            hinstance: null_mut(),
            hwnd: null_mut(),
            hwnd_tabs: null_mut(),
            main_hwnd: null_mut(),
            dialog_id: IDD_PROCPAGE,
            menu_id: IDR_MAINMENU_PROC,
            title_id: IDS_PROCPAGETITLE,
            initial_focus: PageFocusTarget::Tabs,
            perf_state: None,
            proc_state: Some(ProcessPageState::new()),
            task_state: None,
            net_state: None,
            user_state: None,
        }
    }

    pub fn performance_page() -> Self {
        Self {
            hinstance: null_mut(),
            hwnd: null_mut(),
            hwnd_tabs: null_mut(),
            main_hwnd: null_mut(),
            dialog_id: IDD_PERFPAGE,
            menu_id: IDR_MAINMENU_PERF,
            title_id: IDS_PERFPAGETITLE,
            initial_focus: PageFocusTarget::None,
            perf_state: Some(PerformancePageState::new()),
            proc_state: None,
            task_state: None,
            net_state: None,
            user_state: None,
        }
    }

    pub fn network_page() -> Self {
        Self {
            hinstance: null_mut(),
            hwnd: null_mut(),
            hwnd_tabs: null_mut(),
            main_hwnd: null_mut(),
            dialog_id: IDD_NETPAGE,
            menu_id: IDR_MAINMENU_NET,
            title_id: IDS_NETPAGETITLE,
            initial_focus: PageFocusTarget::Control(IDC_NICTOTALS),
            perf_state: None,
            proc_state: None,
            task_state: None,
            net_state: Some(NetworkPageState::new()),
            user_state: None,
        }
    }

    pub fn users_page() -> Self {
        Self {
            hinstance: null_mut(),
            hwnd: null_mut(),
            hwnd_tabs: null_mut(),
            main_hwnd: null_mut(),
            dialog_id: IDD_USERSPAGE,
            menu_id: IDR_MAINMENU_USER,
            title_id: IDS_USERPAGETITLE,
            initial_focus: PageFocusTarget::Control(IDC_USERLIST),
            perf_state: None,
            proc_state: None,
            task_state: None,
            net_state: None,
            user_state: Some(UserPageState::new()),
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn performance_snapshot(&self) -> Option<PerformanceSnapshot> {
        // 主框架只从性能页取轻量快照，不直接读它的内部状态。
        self.perf_state.as_ref().map(PerformancePageState::snapshot)
    }

    pub unsafe fn title(&self, hinstance: HINSTANCE) -> String {
        load_string(hinstance, self.title_id)
    }

    pub unsafe fn initialize(
        &mut self,
        hinstance: HINSTANCE,
        main_hwnd: HWND,
        hwnd_tabs: HWND,
        processor_count: usize,
    ) -> Result<(), u32> {
        // 页面初始化分成两段：
        // 先准备纯状态资源，再创建对话框，最后补上依赖真实 HWND 的后置初始化。
        self.hinstance = hinstance;
        self.main_hwnd = main_hwnd;
        self.hwnd_tabs = hwnd_tabs;

        if let Some(perf_state) = self.perf_state.as_mut() {
            perf_state.initialize(hinstance, processor_count);
        }
        if let Some(task_state) = self.task_state.as_mut() {
            task_state.prepare_initialize(hinstance, main_hwnd)?;
        }

        let proc: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> isize> =
            if self.dialog_id == IDD_PERFPAGE {
                Some(performance_page_proc)
            } else if self.dialog_id == IDD_TASKPAGE {
                Some(task_page_proc)
            } else if self.dialog_id == IDD_PROCPAGE {
                Some(proc_page_proc)
            } else if self.dialog_id == IDD_NETPAGE {
                Some(network_page_proc)
            } else if self.dialog_id == IDD_USERSPAGE {
                Some(users_page_proc)
            } else {
                Some(dialog_page_proc)
            };

        self.hwnd = create_dialog(
            hinstance,
            self.dialog_id,
            main_hwnd,
            proc,
            self as *mut DialogPage as LPARAM,
        );

        if self.hwnd.is_null() {
            if let Some(task_state) = self.task_state.as_mut() {
                task_state.destroy();
            }
            Err(GetLastError())
        } else {
            localize_dialog(self.hwnd, self.dialog_id);
            if let Some(task_state) = self.task_state.as_mut() {
                if let Err(error) = task_state.complete_initialize() {
                    DestroyWindow(self.hwnd);
                    self.hwnd = null_mut();
                    task_state.destroy();
                    return Err(error);
                }
            }
            Ok(())
        }
    }

    pub unsafe fn activate(
        &mut self,
        _hinstance: HINSTANCE,
        main_hwnd: HWND,
        options: &Options,
        processor_count: usize,
        current_menu: &mut HMENU,
    ) -> Result<(), u32> {
        // 激活页面时顺带切换主菜单和焦点目标，
        // 这样每个页面都能看起来像自己“拥有”一套独立菜单。
        if self.hwnd.is_null() {
            return Err(GetLastError());
        }

        ShowWindow(self.hwnd, SW_SHOW);
        SetWindowPos(self.hwnd, null_mut(), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);

        match self.initial_focus {
            PageFocusTarget::None => {}
            PageFocusTarget::Tabs => {
                if !self.hwnd_tabs.is_null() {
                    SetFocus(self.hwnd_tabs);
                }
            }
            PageFocusTarget::Control(control_id) => {
                let focus_hwnd = GetDlgItem(self.hwnd, control_id);
                if !focus_hwnd.is_null() {
                    SetFocus(focus_hwnd);
                }
            }
        }

        let previous_menu = *current_menu;
        let Some(next_menu) = build_main_menu(self.menu_id, processor_count) else {
            return Err(GetLastError());
        };

        *current_menu = next_menu;
        if !options.no_title() {
            SetMenu(main_hwnd, next_menu);
            DrawMenuBar(main_hwnd);
        }

        if !previous_menu.is_null() {
            DestroyMenu(previous_menu);
        }

        self.apply_options(options, processor_count);

        Ok(())
    }

    pub unsafe fn apply_options(&mut self, options: &Options, processor_count: usize) {
        // 宿主层只负责把全局选项广播到实际持有状态的那一页，
        // 页面内部再决定哪些控件需要重排、重绘或重建列。
        if let Some(perf_state) = self.perf_state.as_mut() {
            perf_state.apply_options(self.hwnd, options, processor_count);
            perf_state.size_page(self.hwnd, self.main_hwnd);
        }
        if let Some(proc_state) = self.proc_state.as_mut() {
            proc_state.apply_options(options, processor_count);
        }
        if let Some(task_state) = self.task_state.as_mut() {
            task_state.apply_options(options);
        }
        if let Some(net_state) = self.net_state.as_mut() {
            net_state.apply_options(options);
        }
        if let Some(user_state) = self.user_state.as_mut() {
            user_state.apply_options(options);
        }
    }

    pub unsafe fn timer_event(&mut self, options: &Options, processor_count: usize) {
        // 定时刷新同样走统一入口，避免主框架需要知道每个页面各自的刷新细节。
        if let Some(perf_state) = self.perf_state.as_mut() {
            perf_state.timer_event(self.hwnd, self.main_hwnd);
        }
        if let Some(proc_state) = self.proc_state.as_mut() {
            proc_state.apply_options(options, processor_count);
            proc_state.timer_event(options);
        }
        if let Some(task_state) = self.task_state.as_mut() {
            task_state.timer_event(options);
        }
        if let Some(net_state) = self.net_state.as_mut() {
            net_state.apply_options(options);
            net_state.timer_event();
        }
        if let Some(user_state) = self.user_state.as_mut() {
            user_state.apply_options(options);
            user_state.timer_event();
        }
    }

    pub unsafe fn deactivate(&mut self, options: &mut Options) {
        // 页面切走前只保存必要的易失状态，比如进程页列宽；
        // 其它页面如果没有额外状态，就只需要隐藏窗口。
        if let Some(proc_state) = self.proc_state.as_mut() {
            proc_state.deactivate(options);
        }
        if !self.hwnd.is_null() {
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub unsafe fn destroy(&mut self) {
        // 页面销毁分为“业务资源销毁”和“窗口销毁”两层，前者有些并不依赖窗口仍然存在。
        if let Some(perf_state) = self.perf_state.as_mut() {
            perf_state.destroy();
        }
        if let Some(proc_state) = self.proc_state.as_mut() {
            proc_state.destroy();
        }
        if let Some(task_state) = self.task_state.as_mut() {
            task_state.destroy();
        }
        if let Some(net_state) = self.net_state.as_mut() {
            net_state.destroy();
        }
        if let Some(user_state) = self.user_state.as_mut() {
            user_state.destroy();
        }
        if !self.hwnd.is_null() {
            DestroyWindow(self.hwnd);
            self.hwnd = null_mut();
        }
    }

    pub unsafe fn handle_process_command(
        &mut self,
        command_id: u16,
        options: Option<&mut Options>,
    ) -> bool {
        if let Some(proc_state) = self.proc_state.as_mut() {
            proc_state.handle_command(command_id, options);
            true
        } else {
            false
        }
    }

    pub unsafe fn handle_user_command(&mut self, command_id: u16) -> bool {
        if let Some(user_state) = self.user_state.as_mut() {
            user_state.handle_command(command_id)
        } else {
            false
        }
    }

    pub fn user_show_domain_names(&self) -> Option<bool> {
        self.user_state
            .as_ref()
            .map(UserPageState::show_domain_names)
    }

    pub unsafe fn find_process(&mut self, thread_id: u32, pid: u32) -> bool {
        self.proc_state
            .as_mut()
            .is_some_and(|proc_state| proc_state.find_process(thread_id, pid))
    }
}

pub fn default_pages() -> [DialogPage; 5] {
    [
        DialogPage::task_page(),
        DialogPage::process_page(),
        DialogPage::performance_page(),
        DialogPage::network_page(),
        DialogPage::users_page(),
    ]
}

unsafe fn page_from_hwnd(hwnd: HWND, lparam: LPARAM) -> *mut DialogPage {
    // 初次进入对话框过程时，`lparam` 带着页面指针；
    // 绑定完成后，后续消息都从窗口用户数据中回取同一对象。
    let page = get_window_userdata(hwnd) as *mut DialogPage;
    if !page.is_null() {
        page
    } else {
        lparam as *mut DialogPage
    }
}

unsafe extern "system" fn dialog_page_proc(
    hwnd: HWND,
    msg: u32,
    _wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    // 通用页面过程只负责完成对象绑定和最基础的对话框初始化。
    match msg {
        WM_INITDIALOG => {
            let page = lparam as *mut DialogPage;
            if !page.is_null() {
                (*page).hwnd = hwnd;
                set_window_userdata(hwnd, lparam);
            }
            1
        }
        _ => 0,
    }
}

unsafe extern "system" fn task_page_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    // 各具体页面过程在通用对话框流程之上，补充自己的通知和命令处理。
    let page = page_from_hwnd(hwnd, lparam);

    match msg {
        WM_INITDIALOG => {
            if !page.is_null() {
                (*page).hwnd = hwnd;
                set_window_userdata(hwnd, lparam);
                if let Some(task_state) = (*page).task_state.as_mut() {
                    return task_state.handle_init_dialog(hwnd);
                }
            }
            0
        }
        WM_LBUTTONUP | WM_LBUTTONDOWN => {
            if !page.is_null()
                && (*page)
                    .task_state
                    .as_ref()
                    .is_some_and(TaskPageState::no_title)
            {
                SendMessageW(
                    (*page).main_hwnd,
                    if msg == WM_LBUTTONUP {
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONUP
                    } else {
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONDOWN
                    },
                    HTCAPTION as usize,
                    lparam,
                );
            }
            0
        }
        WM_COMMAND => {
            if !page.is_null() {
                if let Some(task_state) = (*page).task_state.as_mut() {
                    task_state.handle_command((wparam & 0xFFFF) as u16);
                }
            }
            1
        }
        WM_NOTIFY => {
            if !page.is_null() {
                if let Some(task_state) = (*page).task_state.as_mut() {
                    return task_state.handle_notify(lparam);
                }
            }
            0
        }
        WM_CONTEXTMENU => {
            if !page.is_null() && wparam as HWND == GetDlgItem(hwnd, IDC_TASKLIST) {
                if let Some(task_state) = (*page).task_state.as_mut() {
                    task_state.show_context_menu(
                        i32::from((lparam & 0xFFFF) as i16),
                        i32::from(((lparam >> 16) & 0xFFFF) as i16),
                    );
                    return 1;
                }
            }
            0
        }
        WM_SHOWWINDOW | WM_SIZE => {
            if !page.is_null() {
                if let Some(task_state) = (*page).task_state.as_ref() {
                    task_state.size_page();
                }
            }
            1
        }
        _ => 0,
    }
}

unsafe extern "system" fn proc_page_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    // 进程页除了基本的点击/命令转发，还要打开 `WS_CLIPCHILDREN`
    // 来减轻列表与按钮区域一起重绘时的闪烁。
    let page = page_from_hwnd(hwnd, lparam);

    match msg {
        WM_INITDIALOG => {
            if !page.is_null() {
                (*page).hwnd = hwnd;
                set_window_userdata(hwnd, lparam);
                let current_style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
                SetWindowLongW(hwnd, GWL_STYLE, (current_style | WS_CLIPCHILDREN) as i32);
                if let Some(proc_state) = (*page).proc_state.as_mut() {
                    let _ = proc_state.initialize((*page).hinstance, hwnd, (*page).main_hwnd);
                }
            }
            1
        }
        WM_LBUTTONUP | WM_LBUTTONDOWN => {
            if !page.is_null()
                && (*page)
                    .proc_state
                    .as_ref()
                    .is_some_and(ProcessPageState::no_title)
            {
                SendMessageW(
                    (*page).main_hwnd,
                    if msg == WM_LBUTTONUP {
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONUP
                    } else {
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONDOWN
                    },
                    HTCAPTION as usize,
                    lparam,
                );
            }
            0
        }
        WM_NCLBUTTONDBLCLK | WM_LBUTTONDBLCLK => {
            if !page.is_null() {
                SendMessageW((*page).main_hwnd, msg, wparam, lparam);
            }
            0
        }
        WM_COMMAND => {
            if !page.is_null() {
                if let Some(proc_state) = (*page).proc_state.as_mut() {
                    proc_state.handle_command((wparam & 0xFFFF) as u16, None);
                }
            }
            1
        }
        WM_NOTIFY => {
            if !page.is_null() {
                if let Some(proc_state) = (*page).proc_state.as_mut() {
                    return proc_state.handle_notify(lparam);
                }
            }
            0
        }
        WM_CONTEXTMENU => {
            if !page.is_null() && wparam as HWND == GetDlgItem(hwnd, IDC_PROCLIST) {
                if let Some(proc_state) = (*page).proc_state.as_mut() {
                    proc_state.show_context_menu(
                        i32::from((lparam & 0xFFFF) as i16),
                        i32::from(((lparam >> 16) & 0xFFFF) as i16),
                    );
                    return 1;
                }
            }
            0
        }
        WM_SHOWWINDOW | WM_SIZE => {
            if !page.is_null() {
                if let Some(proc_state) = (*page).proc_state.as_ref() {
                    proc_state.size_page();
                }
            }
            1
        }
        _ => 0,
    }
}

unsafe extern "system" fn performance_page_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    // 性能页会处理自绘图表和自绘仪表，因此消息种类比其它页面更丰富。
    let page = page_from_hwnd(hwnd, lparam);

    match msg {
        WM_INITDIALOG => {
            if !page.is_null() {
                (*page).hwnd = hwnd;
                set_window_userdata(hwnd, lparam);

                let current_style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
                SetWindowLongW(hwnd, GWL_STYLE, (current_style | WS_CLIPCHILDREN) as i32);
            }
            1
        }
        WM_LBUTTONUP | WM_LBUTTONDOWN => {
            if !page.is_null()
                && (*page)
                    .perf_state
                    .as_ref()
                    .is_some_and(PerformancePageState::no_title)
            {
                SendMessageW(
                    (*page).main_hwnd,
                    if msg == WM_LBUTTONUP {
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONUP
                    } else {
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONDOWN
                    },
                    HTCAPTION as usize,
                    lparam,
                );
            }
            0
        }
        WM_NCLBUTTONDBLCLK | WM_LBUTTONDBLCLK => {
            if !page.is_null() {
                SendMessageW((*page).main_hwnd, msg, wparam, lparam);
            }
            0
        }
        WM_CTLCOLORBTN => {
            let control_id = GetDlgCtrlID(lparam as HWND);
            if !page.is_null() {
                if let Some(perf_state) = (*page).perf_state.as_ref() {
                    if perf_state.is_graph_control(control_id) {
                        return GetStockObject(BLACK_BRUSH) as isize;
                    }
                }
            }
            0
        }
        WM_DRAWITEM => {
            if page.is_null() {
                return 0;
            }

            let draw_item = &*(lparam as *const DRAWITEMSTRUCT);
            let Some(perf_state) = (*page).perf_state.as_ref() else {
                return 0;
            };

            match wparam as i32 {
                id if perf_state.cpu_graph_pane_index(id).is_some() => {
                    let pane_index = perf_state.cpu_graph_pane_index(id).unwrap_or_default();
                    perf_state.draw_cpu_graph(draw_item.hDC, draw_item.rcItem, pane_index);
                    1
                }
                IDC_CPUMETER => {
                    perf_state.draw_cpu_meter(draw_item.hDC, draw_item.rcItem);
                    1
                }
                IDC_MEMMETER => {
                    perf_state.draw_mem_meter(draw_item.hDC, draw_item.rcItem);
                    1
                }
                IDC_MEMGRAPH => {
                    perf_state.draw_mem_graph(draw_item.hDC, draw_item.rcItem);
                    1
                }
                _ => 0,
            }
        }
        WM_SHOWWINDOW | WM_SIZE => {
            if !page.is_null() {
                if let Some(perf_state) = (*page).perf_state.as_mut() {
                    perf_state.size_page(hwnd, (*page).main_hwnd);
                }
            }
            1
        }
        _ => 0,
    }
}

unsafe extern "system" fn network_page_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    // 网络页除了普通刷新，还要响应滚动条和图表 owner-draw 消息。
    let page = page_from_hwnd(hwnd, lparam);

    match msg {
        WM_INITDIALOG => {
            if !page.is_null() {
                (*page).hwnd = hwnd;
                set_window_userdata(hwnd, lparam);
                let current_style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
                SetWindowLongW(hwnd, GWL_STYLE, (current_style | WS_CLIPCHILDREN) as i32);
                if let Some(net_state) = (*page).net_state.as_mut() {
                    net_state.initialize(hwnd, (*page).main_hwnd, (*page).hwnd_tabs);
                }
            }
            1
        }
        WM_LBUTTONUP | WM_LBUTTONDOWN => {
            if !page.is_null()
                && (*page)
                    .net_state
                    .as_ref()
                    .is_some_and(NetworkPageState::no_title)
            {
                SendMessageW(
                    (*page).main_hwnd,
                    if msg == WM_LBUTTONUP {
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONUP
                    } else {
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONDOWN
                    },
                    HTCAPTION as usize,
                    lparam,
                );
            }
            0
        }
        WM_NCLBUTTONDBLCLK | WM_LBUTTONDBLCLK => {
            if !page.is_null() {
                SendMessageW((*page).main_hwnd, msg, wparam, lparam);
            }
            0
        }
        WM_DRAWITEM => {
            if page.is_null() {
                return 0;
            }

            let Some(net_state) = (*page).net_state.as_ref() else {
                return 0;
            };
            let draw_item = &*(lparam as *const DRAWITEMSTRUCT);
            if let Some(pane_index) = net_state.graph_pane_index(draw_item.CtlID as i32) {
                net_state.draw_graph(draw_item.hDC, draw_item.rcItem, pane_index);
                1
            } else {
                0
            }
        }
        WM_VSCROLL => {
            if !page.is_null() {
                if let Some(net_state) = (*page).net_state.as_mut() {
                    return net_state.handle_vscroll(wparam);
                }
            }
            0
        }
        WM_MOUSEWHEEL => {
            if !page.is_null() {
                if let Some(net_state) = (*page).net_state.as_mut() {
                    return net_state.handle_mouse_wheel(wparam);
                }
            }
            0
        }
        WM_SHOWWINDOW | WM_SIZE => {
            if !page.is_null() {
                if let Some(net_state) = (*page).net_state.as_mut() {
                    net_state.size_page();
                }
            }
            1
        }
        _ => 0,
    }
}

unsafe extern "system" fn users_page_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    // 用户页的交互主要围绕 ListView 选择变化和上下文菜单操作展开。
    let page = page_from_hwnd(hwnd, lparam);

    match msg {
        WM_INITDIALOG => {
            if !page.is_null() {
                (*page).hwnd = hwnd;
                set_window_userdata(hwnd, lparam);
                if let Some(user_state) = (*page).user_state.as_mut() {
                    user_state.initialize(hwnd);
                }
            }
            1
        }
        WM_LBUTTONUP | WM_LBUTTONDOWN => {
            if !page.is_null()
                && (*page)
                    .user_state
                    .as_ref()
                    .is_some_and(UserPageState::no_title)
            {
                SendMessageW(
                    (*page).main_hwnd,
                    if msg == WM_LBUTTONUP {
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONUP
                    } else {
                        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONDOWN
                    },
                    HTCAPTION as usize,
                    lparam,
                );
            }
            0
        }
        WM_NCLBUTTONDBLCLK | WM_LBUTTONDBLCLK => {
            if !page.is_null() {
                SendMessageW((*page).main_hwnd, msg, wparam, lparam);
            }
            0
        }
        WM_NOTIFY => {
            if !page.is_null() {
                if let Some(user_state) = (*page).user_state.as_mut() {
                    return user_state.handle_notify(lparam);
                }
            }
            0
        }
        WM_COMMAND => {
            if !page.is_null() {
                if let Some(user_state) = (*page).user_state.as_mut() {
                    return isize::from(user_state.handle_command((wparam & 0xFFFF) as u16));
                }
            }
            0
        }
        WM_CONTEXTMENU => {
            if !page.is_null() && wparam as HWND == GetDlgItem(hwnd, IDC_USERLIST) {
                if let Some(user_state) = (*page).user_state.as_mut() {
                    user_state.show_context_menu(
                        i32::from((lparam & 0xFFFF) as i16),
                        i32::from(((lparam >> 16) & 0xFFFF) as i16),
                    );
                    return 1;
                }
            }
            0
        }
        WM_SHOWWINDOW | WM_SIZE => {
            if !page.is_null() {
                if let Some(user_state) = (*page).user_state.as_ref() {
                    user_state.size_page();
                }
            }
            1
        }
        _ => 0,
    }
}

use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::CStr;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use std::slice;

use windows_sys::Win32::Foundation::{
    CloseHandle, FILETIME, FreeLibrary, HANDLE, HINSTANCE, HMODULE, HWND, INVALID_HANDLE_VALUE, LPARAM,
    POINT, RECT, WPARAM,
};
use windows_sys::Win32::Graphics::Gdi::MapWindowPoints;
use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, IsWellKnownSid, LookupAccountSidW, SID_NAME_USE, TOKEN_QUERY, TOKEN_USER,
    TokenSessionId, TokenUser, WinLocalServiceSid, WinLocalSystemSid, WinNetworkServiceSid,
};
use windows_sys::Win32::System::RemoteDesktop::{
    WTSEnumerateProcessesW, WTSFreeMemory, WTS_CURRENT_SERVER_HANDLE, WTS_PROCESS_INFOW,
};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, GetPriorityClass, GetProcessAffinityMask, GetProcessHandleCount,
    GetProcessTimes, GetSystemTimes, GetThreadTimes, OpenProcess, OpenProcessToken, OpenThread, SetPriorityClass,
    SetProcessAffinityMask, TerminateProcess, HIGH_PRIORITY_CLASS, IDLE_PRIORITY_CLASS,
    NORMAL_PRIORITY_CLASS, PROCESS_INFORMATION, PROCESS_QUERY_INFORMATION,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION, PROCESS_TERMINATE,
    PROCESS_VM_READ, REALTIME_PRIORITY_CLASS, STARTUPINFOW, THREAD_QUERY_INFORMATION,
};
use windows_sys::Win32::System::VirtualDosMachines::{VDMENUMTASKWOWEXPROC, VDMTERMINATETASKINWOWPROC};
use windows_sys::Win32::UI::Controls::{
    CheckDlgButton, IsDlgButtonChecked, BST_CHECKED, BST_UNCHECKED, LVCFMT_LEFT, LVCFMT_RIGHT,
    LVCF_FMT, LVCF_SUBITEM, LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_PARAM, LVIF_STATE, LVIF_TEXT,
    LVIS_FOCUSED, LVIS_SELECTED, LVITEMW, LVN_COLUMNCLICK, LVN_GETDISPINFOW, LVN_ITEMCHANGED,
    LVNI_SELECTED, NMHDR, NMLVDISPINFOW,
    LVM_DELETEALLITEMS, LVM_DELETECOLUMN, LVM_DELETEITEM, LVM_ENSUREVISIBLE, LVM_GETCOLUMNWIDTH,
    LVM_GETITEMCOUNT, LVM_GETITEMW, LVM_GETNEXTITEM, LVM_INSERTCOLUMNW, LVM_INSERTITEMW,
    LVM_REDRAWITEMS, LVM_SETITEMSTATE, LVM_SETITEMW, LVS_SHOWSELALWAYS, NMLISTVIEW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BeginDeferWindowPos, CheckMenuRadioItem, DeferWindowPos, DialogBoxParamW, DestroyMenu,
    EnableMenuItem, EndDeferWindowPos, EndDialog, GetClientRect, GetCursorPos, GetDlgItem,
    GetSubMenu, GetWindowLongW, LoadMenuW, MessageBoxW, RemoveMenu, SendMessageW, SetWindowLongW,
    GWL_STYLE, HMENU, IDCANCEL, IDOK, IDYES, MB_ICONERROR, MB_ICONEXCLAMATION,
    MB_OK, MB_YESNO, MF_BYCOMMAND, MF_BYPOSITION, MF_DISABLED, MF_GRAYED, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, TPM_RETURNCMD, TrackPopupMenuEx, WM_COMMAND,
    WM_ENABLE, WM_INITDIALOG, WM_SETREDRAW,
};

use crate::options::Options;
use crate::options::{ColumnId, UpdateSpeed};
use crate::localization::{localize_dialog, localize_menu};
use crate::resource::*;
use crate::winutil::{
    append_32_bit_suffix, get_window_userdata, is_32_bit_process_handle, load_string, loword,
    make_int_resource, sanitize_task_manager_menu, set_window_userdata, subclass_list_view,
    to_wide_null,
};

const PROCESS_COLUMNS: [ProcessColumn; NUM_COLUMN] = [
    ProcessColumn::new(IDS_COL_IMAGENAME, LVCFMT_LEFT, 107),
    ProcessColumn::new(IDS_COL_PID, LVCFMT_RIGHT, 50),
    ProcessColumn::new(IDS_COL_USERNAME, LVCFMT_LEFT, 107),
    ProcessColumn::new(IDS_COL_SESSIONID, LVCFMT_RIGHT, 60),
    ProcessColumn::new(IDS_COL_CPU, LVCFMT_RIGHT, 35),
    ProcessColumn::new(IDS_COL_CPUTIME, LVCFMT_RIGHT, 70),
    ProcessColumn::new(IDS_COL_MEMUSAGE, LVCFMT_RIGHT, 70),
    ProcessColumn::new(IDS_COL_MEMUSAGEDIFF, LVCFMT_RIGHT, 70),
    ProcessColumn::new(IDS_COL_PAGEFAULTS, LVCFMT_RIGHT, 70),
    ProcessColumn::new(IDS_COL_PAGEFAULTSDIFF, LVCFMT_RIGHT, 70),
    ProcessColumn::new(IDS_COL_COMMITCHARGE, LVCFMT_RIGHT, 70),
    ProcessColumn::new(IDS_COL_PAGEDPOOL, LVCFMT_RIGHT, 70),
    ProcessColumn::new(IDS_COL_NONPAGEDPOOL, LVCFMT_RIGHT, 70),
    ProcessColumn::new(IDS_COL_BASEPRIORITY, LVCFMT_RIGHT, 60),
    ProcessColumn::new(IDS_COL_HANDLECOUNT, LVCFMT_RIGHT, 60),
    ProcessColumn::new(IDS_COL_THREADCOUNT, LVCFMT_RIGHT, 60),
];

const COLUMN_DIALOG_IDS: [i32; NUM_COLUMN] = [
    IDC_IMAGENAME,
    IDC_PID,
    IDC_USERNAME,
    IDC_SESSIONID,
    IDC_CPU,
    IDC_CPUTIME,
    IDC_MEMUSAGE,
    IDC_MEMUSAGEDIFF,
    IDC_PAGEFAULTS,
    IDC_PAGEFAULTSDIFF,
    IDC_COMMITCHARGE,
    IDC_PAGEDPOOL,
    IDC_NONPAGEDPOOL,
    IDC_BASEPRIORITY,
    IDC_HANDLECOUNT,
    IDC_THREADCOUNT,
];

const DEFAULT_MARGIN: i32 = 8;
const TEXT_CALLBACK_WIDE: *mut u16 = -1isize as *mut u16;

#[derive(Clone, Copy)]
struct ProcessColumn {
    title_id: u32,
    fmt: i32,
    default_width: i32,
}

impl ProcessColumn {
    const fn new(title_id: u32, fmt: i32, default_width: i32) -> Self {
        Self {
            title_id,
            fmt,
            default_width,
        }
    }
}

#[derive(Clone, Default)]
struct PreviousProcSample {
    raw_cpu_time_100ns: u64,
    display_cpu_time_100ns: u64,
    mem_usage_kb: u32,
    page_faults: u32,
}

#[derive(Clone, Copy, Default)]
struct DirtyColumns(u32);

impl DirtyColumns {
    fn all() -> Self {
        Self(u32::MAX)
    }

    fn from_column(column_id: ColumnId) -> Self {
        Self(1u32 << column_id as u32)
    }

    fn mark(&mut self, column_id: ColumnId) {
        self.0 |= Self::from_column(column_id).0;
    }

    fn any(self) -> bool {
        self.0 != 0
    }
}

#[derive(Clone)]
pub struct ProcEntry {
    pid: u32,
    real_pid: u32,
    image_name: String,
    is_32_bit: bool,
    user_name: String,
    session_id: u32,
    cpu: u8,
    cpu_time_100ns: u64,
    display_cpu_time_100ns: u64,
    mem_usage_kb: u32,
    mem_diff_kb: i64,
    page_faults: u32,
    page_faults_diff: i64,
    commit_charge_kb: u32,
    paged_pool_kb: u32,
    nonpaged_pool_kb: u32,
    priority_class: u32,
    handle_count: u32,
    thread_count: u32,
    wow_task_handle: u16,
    is_wow_task: bool,
    pass_count: u64,
    dirty_columns: DirtyColumns,
}

#[derive(Default)]
struct ProcessStrings {
    warning: String,
    invalid_option: String,
    no_affinity_mask: String,
    kill: String,
    debug: String,
    prichange: String,
    cant_kill: String,
    cant_debug: String,
    cant_change_priority: String,
    cant_set_affinity: String,
    priority_low: String,
    priority_normal: String,
    priority_high: String,
    priority_realtime: String,
    priority_unknown: String,
}

struct ColumnDialogContext {
    page: *mut ProcessPageState,
    options: *mut Options,
}

struct AffinityDialogContext {
    page: *mut ProcessPageState,
    process_mask: usize,
}

pub struct ProcessPageState {
    hinstance: HINSTANCE,
    hwnd_page: HWND,
    main_hwnd: HWND,
    entries: Vec<ProcEntry>,
    previous_samples: HashMap<u32, PreviousProcSample>,
    previous_system_time: u64,
    active_columns: Vec<ColumnId>,
    selected_pid: Option<u32>,
    sort_column: ColumnId,
    sort_direction: i32,
    paused: bool,
    confirmations: bool,
    no_title: bool,
    show_16bit: bool,
    processor_count: usize,
    debugger_path: Option<String>,
    strings: ProcessStrings,
    pass_count: u64,
}

impl Default for ProcessPageState {
    fn default() -> Self {
        Self {
            hinstance: null_mut(),
            hwnd_page: null_mut(),
            main_hwnd: null_mut(),
            entries: Vec::new(),
            previous_samples: HashMap::new(),
            previous_system_time: 0,
            active_columns: Vec::new(),
            selected_pid: None,
            sort_column: ColumnId::Pid,
            sort_direction: 1,
            paused: false,
            confirmations: true,
            no_title: false,
            show_16bit: true,
            processor_count: 1,
            debugger_path: None,
            strings: ProcessStrings::default(),
            pass_count: 0,
        }
    }
}

impl ProcessPageState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn no_title(&self) -> bool {
        self.no_title
    }

    pub unsafe fn initialize(
        &mut self,
        hinstance: HINSTANCE,
        hwnd_page: HWND,
        main_hwnd: HWND,
    ) -> Result<(), u32> {
        self.hinstance = hinstance;
        self.hwnd_page = hwnd_page;
        self.main_hwnd = main_hwnd;
        self.load_strings();
        self.debugger_path = load_debugger_path();

        let list_hwnd = self.list_hwnd();
        subclass_list_view(list_hwnd);
        let current_style = GetWindowLongW(list_hwnd, GWL_STYLE) as u32;
        SetWindowLongW(list_hwnd, GWL_STYLE, (current_style | LVS_SHOWSELALWAYS) as i32);
        self.update_ui_state();
        Ok(())
    }

    pub unsafe fn apply_options(&mut self, options: &Options, processor_count: usize) {
        self.no_title = options.no_title();
        self.confirmations = options.confirmations();
        self.show_16bit = options.show_16bit();
        self.processor_count = processor_count.max(1);

        let desired_columns = columns_from_options(options);
        if desired_columns != self.active_columns {
            self.active_columns = desired_columns;
            self.setup_columns(options);
            self.refresh_processes();
        }
    }

    pub unsafe fn timer_event(&mut self, options: &Options) {
        self.paused = options.update_speed == UpdateSpeed::Paused as i32;
        if !self.paused {
            self.refresh_processes();
        }
    }

    pub unsafe fn deactivate(&mut self, options: &mut Options) {
        self.save_column_widths(options);
    }

    pub unsafe fn destroy(&mut self) {
        self.entries.clear();
        self.previous_samples.clear();
    }

    pub unsafe fn handle_notify(&mut self, lparam: LPARAM) -> isize {
        let notify_header = &*(lparam as *const NMHDR);
        match notify_header.code {
            code if code == LVN_GETDISPINFOW => {
                let display_info = &mut *(lparam as *mut NMLVDISPINFOW);
                self.fill_display_info(&mut display_info.item);
                1
            }
            code if code == LVN_ITEMCHANGED => {
                let notify = &*(lparam as *const NMLISTVIEW);
                if (notify.uChanged & LVIF_STATE as u32) != 0 {
                    self.selected_pid = self.current_selected_pid();
                    self.update_ui_state();
                }
                1
            }
            code if code == LVN_COLUMNCLICK => {
                let notify = &*(lparam as *const NMLISTVIEW);
                let clicked = self
                    .active_columns
                    .get(notify.iSubItem as usize)
                    .copied()
                    .unwrap_or(ColumnId::Pid);
                if self.sort_column == clicked {
                    self.sort_direction *= -1;
                } else {
                    self.sort_column = clicked;
                    self.sort_direction = -1;
                }
                self.refresh_processes();
                1
            }
            _ => 0,
        }
    }

    pub unsafe fn handle_command(&mut self, command_id: u16, options: Option<&mut Options>) {
        match command_id {
            id if id == IDC_TERMINATE as u16 || id == IDM_PROC_TERMINATE => {
                if let Some(pid) = self.current_selected_pid() {
                    self.kill_process(pid);
                }
            }
            id if id == IDC_DEBUG as u16 || id == IDM_PROC_DEBUG => {
                if let Some(pid) = self.current_selected_pid() {
                    self.attach_debugger(pid);
                }
            }
            IDM_AFFINITY => {
                if let Some(pid) = self.current_selected_pid() {
                    self.set_affinity(pid);
                }
            }
            IDM_PROC_REALTIME | IDM_PROC_HIGH | IDM_PROC_NORMAL | IDM_PROC_LOW => {
                if let Some(pid) = self.current_selected_pid() {
                    self.set_priority(pid, command_id);
                }
            }
            IDM_PROCCOLS => {
                if let Some(options) = options {
                    self.pick_columns(options);
                }
            }
            _ => {}
        }
    }

    pub unsafe fn show_context_menu(&mut self, x: i32, y: i32) {
        self.selected_pid = self.current_selected_pid();
        let Some(entry) = self.selected_entry() else {
            return;
        };

        let popup = load_popup_menu(self.hinstance, IDR_PROC_CONTEXT);
        if popup.is_null() {
            return;
        }

        if self.debugger_path.is_none() || entry.is_wow_task {
            EnableMenuItem(
                popup,
                IDM_PROC_DEBUG as u32,
                MF_BYCOMMAND | MF_GRAYED | MF_DISABLED,
            );
        }

        if entry.is_wow_task {
            for priority_id in [IDM_PROC_REALTIME, IDM_PROC_HIGH, IDM_PROC_NORMAL, IDM_PROC_LOW] {
                EnableMenuItem(
                    popup,
                    priority_id as u32,
                    MF_BYCOMMAND | MF_GRAYED | MF_DISABLED,
                );
            }
        }

        if self.processor_count <= 1 || entry.is_wow_task {
            RemoveMenu(popup, IDM_AFFINITY as u32, MF_BYCOMMAND);
        }

        let priority_submenu = GetSubMenu(popup, 3);
        if !priority_submenu.is_null() {
            let checked = match entry.priority_class {
                value if value == IDLE_PRIORITY_CLASS => IDM_PROC_LOW,
                value if value == HIGH_PRIORITY_CLASS => IDM_PROC_HIGH,
                value if value == REALTIME_PRIORITY_CLASS => IDM_PROC_REALTIME,
                _ => IDM_PROC_NORMAL,
            };
            CheckMenuRadioItem(
                priority_submenu,
                IDM_PROC_REALTIME as u32,
                IDM_PROC_LOW as u32,
                checked as u32,
                MF_BYCOMMAND,
            );
        }

        self.paused = true;
        let mut cursor = POINT { x, y };
        if cursor.x == -1 && cursor.y == -1 {
            GetCursorPos(&mut cursor);
        }

        SendMessageW(self.main_hwnd, crate::resource::PWM_INPOPUP, 1, 0);
        let command = TrackPopupMenuEx(
            popup,
            TPM_RETURNCMD,
            cursor.x,
            cursor.y,
            self.hwnd_page,
            null(),
        );
        SendMessageW(self.main_hwnd, crate::resource::PWM_INPOPUP, 0, 0);
        DestroyMenu(popup);
        self.paused = false;

        if command != 0 {
            self.handle_command(command as u16, None);
        }
    }

    pub unsafe fn size_page(&self) {
        let mut parent_rect = zeroed::<RECT>();
        GetClientRect(self.hwnd_page, &mut parent_rect);

        let mut hdwp = BeginDeferWindowPos(10);
        if hdwp.is_null() {
            return;
        }

        let terminate_hwnd = GetDlgItem(self.hwnd_page, IDC_TERMINATE);
        let list_hwnd = self.list_hwnd();
        if terminate_hwnd.is_null() || list_hwnd.is_null() {
            EndDeferWindowPos(hdwp);
            return;
        }

        let terminate_rect = window_rect_relative_to_page(terminate_hwnd, self.hwnd_page);
        let dx = (parent_rect.right - DEFAULT_MARGIN * 2) - terminate_rect.right;
        let dy = (parent_rect.bottom - DEFAULT_MARGIN * 2) - terminate_rect.bottom;

        hdwp = DeferWindowPos(
            hdwp,
            terminate_hwnd,
            null_mut(),
            terminate_rect.left + dx,
            terminate_rect.top + dy,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );

        let list_rect = window_rect_relative_to_page(list_hwnd, self.hwnd_page);
        hdwp = DeferWindowPos(
            hdwp,
            list_hwnd,
            null_mut(),
            0,
            0,
            (terminate_rect.right - list_rect.left + dx).max(0),
            (terminate_rect.top - list_rect.top + dy - DEFAULT_MARGIN).max(0),
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );

        EndDeferWindowPos(hdwp);
    }

    pub unsafe fn find_process(&mut self, thread_id: u32, pid: u32) -> bool {
        let target_pid = if thread_id != 0 {
            self.entries
                .iter()
                .find(|entry| entry.is_wow_task && entry.pid == thread_id)
                .map(|entry| entry.pid)
                .unwrap_or(pid)
        } else {
            pid
        };

        let Some(index) = self.entries.iter().position(|entry| entry.pid == target_pid) else {
            return false;
        };

        self.selected_pid = Some(target_pid);
        let list_hwnd = self.list_hwnd();
        for item_index in 0..self.entries.len() {
            let mut item = LVITEMW {
                stateMask: LVIS_SELECTED | LVIS_FOCUSED,
                state: if item_index == index {
                    LVIS_SELECTED | LVIS_FOCUSED
                } else {
                    0
                },
                ..zeroed()
            };
            SendMessageW(
                list_hwnd,
                LVM_SETITEMSTATE,
                item_index,
                &mut item as *mut _ as LPARAM,
            );
        }
        SendMessageW(list_hwnd, LVM_ENSUREVISIBLE, index, 0);
        self.update_ui_state();
        true
    }

    fn list_hwnd(&self) -> HWND {
        unsafe { GetDlgItem(self.hwnd_page, IDC_PROCLIST) }
    }

    unsafe fn load_strings(&mut self) {
        self.strings.warning = load_string(self.hinstance, IDS_WARNING);
        self.strings.invalid_option = load_string(self.hinstance, IDS_INVALIDOPTION);
        self.strings.no_affinity_mask = load_string(self.hinstance, IDS_NOAFFINITYMASK);
        self.strings.kill = load_string(self.hinstance, IDS_KILL);
        self.strings.debug = load_string(self.hinstance, IDS_DEBUG);
        self.strings.prichange = load_string(self.hinstance, IDS_PRICHANGE);
        self.strings.cant_kill = load_string(self.hinstance, IDS_CANTKILL);
        self.strings.cant_debug = load_string(self.hinstance, IDS_CANTDEBUG);
        self.strings.cant_change_priority = load_string(self.hinstance, IDS_CANTCHANGEPRI);
        self.strings.cant_set_affinity = load_string(self.hinstance, IDS_CANTSETAFFINITY);
        self.strings.priority_low = load_string(self.hinstance, IDS_LOW);
        self.strings.priority_normal = load_string(self.hinstance, IDS_NORMAL);
        self.strings.priority_high = load_string(self.hinstance, IDS_HIGH);
        self.strings.priority_realtime = load_string(self.hinstance, IDS_REALTIME);
        self.strings.priority_unknown = load_string(self.hinstance, IDS_UNKNOWN);
    }

    unsafe fn update_ui_state(&self) {
        let has_selection = self.current_selected_pid().is_some();
        let terminate_button = GetDlgItem(self.hwnd_page, IDC_TERMINATE);
        if !terminate_button.is_null() {
            SendMessageW(terminate_button, WM_ENABLE, has_selection as usize, 0);
        }
    }

    unsafe fn refresh_processes(&mut self) {
        let previous_selection = self.current_selected_pid().or(self.selected_pid);
        let system_total = current_system_time();
        let total_delta = system_total.saturating_sub(self.previous_system_time);
        let previous_samples = self.previous_samples.clone();
        let (entries, next_samples) =
            collect_process_entries(&previous_samples, total_delta, self.show_16bit);
        let current_pass = self.pass_count;

        for snapshot in entries {
            if let Some(existing) = self
                .entries
                .iter_mut()
                .find(|entry| same_entry_identity(entry, &snapshot))
            {
                update_process_entry(existing, &snapshot, current_pass);
            } else {
                self.entries.push(snapshot.with_pass_count(current_pass));
            }
        }

        self.remove_stale_entries(current_pass);
        self.resort_entries();
        self.previous_samples = next_samples;
        self.previous_system_time = system_total;
        self.selected_pid = previous_selection;
        self.rebuild_listview();
        self.pass_count = self.pass_count.wrapping_add(1);
    }

    fn resort_entries(&mut self) {
        let sort_context = build_sort_context(&self.entries);
        self.entries.sort_by(|left, right| {
            compare_entries(
                left,
                right,
                &sort_context,
                self.sort_column,
                self.sort_direction,
            )
        });
    }

    fn remove_stale_entries(&mut self, current_pass: u64) {
        let mut index = 0;
        while index < self.entries.len() {
            if self.entries[index].pass_count == current_pass {
                index += 1;
            } else {
                self.entries.remove(index);
            }
        }
    }

    unsafe fn rebuild_listview(&mut self) {
        let list_hwnd = self.list_hwnd();
        SendMessageW(list_hwnd, WM_SETREDRAW, 0, 0);

        let selected_pid = self.selected_pid;
        let mut selected_index = None;
        let mut existing_count = SendMessageW(list_hwnd, LVM_GETITEMCOUNT, 0, 0) as usize;
        let common_count = existing_count.min(self.entries.len());

        for index in 0..common_count {
            let entry = &self.entries[index];
            let mut current_item = LVITEMW {
                mask: LVIF_PARAM,
                iItem: index as i32,
                ..zeroed()
            };
            let current_pid = if SendMessageW(list_hwnd, LVM_GETITEMW, 0, &mut current_item as *mut _ as LPARAM) != 0 {
                Some(current_item.lParam as u32)
            } else {
                None
            };

            let item_state = if selected_pid == Some(entry.pid) {
                selected_index = Some(index);
                LVIS_SELECTED | LVIS_FOCUSED
            } else {
                0
            };

            if current_pid != Some(entry.pid) {
                self.replace_row(list_hwnd, index, entry, item_state);
                self.entries[index].dirty_columns = DirtyColumns::default();
            } else {
                if entry.dirty_columns.any() {
                    SendMessageW(list_hwnd, LVM_REDRAWITEMS, index, index as LPARAM);
                    self.entries[index].dirty_columns = DirtyColumns::default();
                }
            }
        }

        while existing_count > self.entries.len() {
            existing_count -= 1;
            SendMessageW(list_hwnd, LVM_DELETEITEM, existing_count, 0);
        }

        for index in common_count..self.entries.len() {
            let entry = &self.entries[index];
            let item_state = if selected_pid == Some(entry.pid) {
                selected_index = Some(index);
                LVIS_SELECTED | LVIS_FOCUSED
            } else {
                0
            };
            self.insert_row(list_hwnd, index, entry, item_state);
            self.entries[index].dirty_columns = DirtyColumns::default();
        }

        SendMessageW(list_hwnd, WM_SETREDRAW, 1, 0);

        if selected_index.is_none() {
            self.selected_pid = None;
        }

        self.update_ui_state();
    }

    unsafe fn insert_row(&self, list_hwnd: HWND, index: usize, entry: &ProcEntry, item_state: u32) {
        let mut item = LVITEMW {
            mask: LVIF_TEXT | LVIF_PARAM | LVIF_STATE,
            iItem: index as i32,
            iSubItem: 0,
            pszText: TEXT_CALLBACK_WIDE,
            cchTextMax: 0,
            lParam: entry.pid as isize,
            stateMask: LVIS_SELECTED | LVIS_FOCUSED,
            state: item_state,
            ..zeroed()
        };
        SendMessageW(list_hwnd, LVM_INSERTITEMW, 0, &mut item as *mut _ as LPARAM);
    }

    unsafe fn replace_row(&self, list_hwnd: HWND, index: usize, entry: &ProcEntry, item_state: u32) {
        let mut item = LVITEMW {
            mask: LVIF_TEXT | LVIF_PARAM | LVIF_STATE,
            iItem: index as i32,
            iSubItem: 0,
            pszText: TEXT_CALLBACK_WIDE,
            cchTextMax: 0,
            lParam: entry.pid as isize,
            stateMask: LVIS_SELECTED | LVIS_FOCUSED,
            state: item_state,
            ..zeroed()
        };
        SendMessageW(list_hwnd, LVM_SETITEMW, 0, &mut item as *mut _ as LPARAM);
        SendMessageW(list_hwnd, LVM_REDRAWITEMS, index, index as LPARAM);
    }

    unsafe fn fill_display_info(&self, item: &mut LVITEMW) {
        if (item.mask & LVIF_TEXT) == 0 || item.iItem < 0 || item.pszText.is_null() || item.cchTextMax <= 0 {
            return;
        }

        let entry = if item.lParam != 0 {
            self.entries
                .iter()
                .find(|entry| entry.pid == item.lParam as u32)
        } else {
            self.entries.get(item.iItem as usize)
        };
        let Some(entry) = entry else {
            *item.pszText = 0;
            return;
        };
        let Some(column_id) = self.active_columns.get(item.iSubItem as usize).copied() else {
            *item.pszText = 0;
            return;
        };

        if entry.is_wow_task
            && !matches!(
                column_id,
                ColumnId::ImageName
                    | ColumnId::Username
                    | ColumnId::SessionId
                    | ColumnId::BasePriority
                    | ColumnId::ThreadCount
                    | ColumnId::CpuTime
                    | ColumnId::Cpu
            )
        {
            *item.pszText = 0;
            return;
        }

        let text = column_text(entry, column_id, &self.strings);
        copy_text_to_callback_buffer(item.pszText, item.cchTextMax as usize, &text);
    }

    unsafe fn setup_columns(&self, options: &Options) {
        let list_hwnd = self.list_hwnd();
        SendMessageW(list_hwnd, LVM_DELETEALLITEMS, 0, 0);
        while SendMessageW(list_hwnd, LVM_DELETECOLUMN, 0, 0) != 0 {}

        for (index, column_id) in self.active_columns.iter().enumerate() {
            let column = PROCESS_COLUMNS[*column_id as usize];
            let width = options
                .column_widths
                .get(index)
                .copied()
                .filter(|value| *value > 0)
                .unwrap_or(column.default_width);
            let title = load_string(self.hinstance, column.title_id);
            let mut title_wide = to_wide_null(&title);
            let mut lv_column = LVCOLUMNW {
                mask: LVCF_FMT | LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM,
                fmt: column.fmt,
                cx: width,
                pszText: title_wide.as_mut_ptr(),
                cchTextMax: title_wide.len() as i32,
                iSubItem: index as i32,
                ..zeroed()
            };
            SendMessageW(
                list_hwnd,
                LVM_INSERTCOLUMNW,
                index,
                &mut lv_column as *mut _ as LPARAM,
            );
        }
    }

    unsafe fn save_column_widths(&mut self, options: &mut Options) {
        for value in options.column_widths.iter_mut() {
            *value = -1;
        }

        for index in 0..self.active_columns.len() {
            let cx = SendMessageW(self.list_hwnd(), LVM_GETCOLUMNWIDTH, index, 0) as i32;
            if index < options.column_widths.len() {
                options.column_widths[index] = cx;
            }
        }
    }

    unsafe fn current_selected_pid(&self) -> Option<u32> {
        let list_hwnd = self.list_hwnd();
        let index = SendMessageW(list_hwnd, LVM_GETNEXTITEM, usize::MAX, LVNI_SELECTED as LPARAM) as i32;
        if index < 0 {
            return None;
        }

        let mut item = LVITEMW {
            mask: LVIF_PARAM,
            iItem: index,
            ..zeroed()
        };
        if SendMessageW(list_hwnd, LVM_GETITEMW, 0, &mut item as *mut _ as LPARAM) != 0 {
            Some(item.lParam as u32)
        } else {
            None
        }
    }

    fn selected_entry(&self) -> Option<&ProcEntry> {
        let pid = self.selected_pid?;
        self.entries.iter().find(|entry| entry.pid == pid)
    }

    unsafe fn quick_confirm(&self, title: &str, body: &str) -> bool {
        if !self.confirmations {
            return true;
        }

        let title_wide = to_wide_null(title);
        let body_wide = to_wide_null(body);
        MessageBoxW(
            self.hwnd_page,
            body_wide.as_ptr(),
            title_wide.as_ptr(),
            MB_ICONEXCLAMATION | MB_YESNO,
        ) == IDYES
    }

    unsafe fn show_failure_message(&self, body: &str, error: u32) {
        let title = if self.strings.warning.is_empty() {
            "Task Manager".to_string()
        } else {
            self.strings.warning.clone()
        };
        let message = format!("{body}\r\n\r\nWin32 error: {error}");
        let title_wide = to_wide_null(&title);
        let message_wide = to_wide_null(&message);
        MessageBoxW(
            self.hwnd_page,
            message_wide.as_ptr(),
            title_wide.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }

    unsafe fn kill_process(&mut self, pid: u32) -> bool {
        if !self.quick_confirm(&self.strings.warning, &self.strings.kill) {
            return false;
        }

        if let Some(entry) = self.entries.iter().find(|entry| entry.pid == pid && entry.is_wow_task) {
            let Some(vdmdbg) = VdmDbgApi::load() else {
                self.show_failure_message(&self.strings.cant_kill, 0);
                return false;
            };
            let terminated = if let Some(terminate_task) = vdmdbg.terminate_task {
                terminate_task(entry.real_pid, entry.wow_task_handle)
            } else {
                0
            };

            if terminated == 0 {
                self.show_failure_message(&self.strings.cant_kill, windows_sys::Win32::Foundation::GetLastError());
                false
            } else {
                self.paused = false;
                self.refresh_processes();
                true
            }
        } else {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() {
            self.show_failure_message(&self.strings.cant_kill, windows_sys::Win32::Foundation::GetLastError());
            return false;
        }

        let result = TerminateProcess(handle, 1);
        let error = windows_sys::Win32::Foundation::GetLastError();
        CloseHandle(handle);

        if result == 0 {
            self.show_failure_message(&self.strings.cant_kill, error);
            false
        } else {
            self.paused = false;
            self.refresh_processes();
            true
        }
        }
    }

    unsafe fn attach_debugger(&mut self, pid: u32) -> bool {
        let Some(debugger_path) = self.debugger_path.as_ref() else {
            MessageBeep(0);
            return false;
        };

        if !self.quick_confirm(&self.strings.warning, &self.strings.debug) {
            return false;
        }

        let command_line = format!("{debugger_path} -p {pid}");
        let mut command_line_wide = to_wide_null(&command_line);
        let mut startup_info = STARTUPINFOW {
            cb: size_of::<STARTUPINFOW>() as u32,
            ..zeroed()
        };
        let mut process_info = zeroed::<PROCESS_INFORMATION>();

        let created = CreateProcessW(
            null(),
            command_line_wide.as_mut_ptr(),
            null_mut(),
            null_mut(),
            0,
            windows_sys::Win32::System::Threading::CREATE_NEW_CONSOLE,
            null(),
            null(),
            &mut startup_info,
            &mut process_info,
        );

        if created == 0 {
            self.show_failure_message(&self.strings.cant_debug, windows_sys::Win32::Foundation::GetLastError());
            false
        } else {
            CloseHandle(process_info.hThread);
            CloseHandle(process_info.hProcess);
            true
        }
    }

    unsafe fn set_priority(&mut self, pid: u32, command_id: u16) -> bool {
        let priority_class = match command_id {
            IDM_PROC_LOW => IDLE_PRIORITY_CLASS,
            IDM_PROC_HIGH => HIGH_PRIORITY_CLASS,
            IDM_PROC_REALTIME => REALTIME_PRIORITY_CLASS,
            _ => NORMAL_PRIORITY_CLASS,
        };

        if !self.quick_confirm(&self.strings.warning, &self.strings.prichange) {
            return false;
        }

        let handle = OpenProcess(PROCESS_SET_INFORMATION, 0, pid);
        if handle.is_null() {
            self.show_failure_message(
                &self.strings.cant_change_priority,
                windows_sys::Win32::Foundation::GetLastError(),
            );
            return false;
        }

        let result = SetPriorityClass(handle, priority_class);
        let error = windows_sys::Win32::Foundation::GetLastError();
        CloseHandle(handle);

        if result == 0 {
            self.show_failure_message(&self.strings.cant_change_priority, error);
            false
        } else {
            self.paused = false;
            self.refresh_processes();
            true
        }
    }

    unsafe fn set_affinity(&mut self, pid: u32) -> bool {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_INFORMATION, 0, pid);
        if handle.is_null() {
            self.show_failure_message(
                &self.strings.cant_set_affinity,
                windows_sys::Win32::Foundation::GetLastError(),
            );
            return false;
        }

        let mut process_mask = 0usize;
        let mut system_mask = 0usize;
        let mut success = false;

        if GetProcessAffinityMask(handle, &mut process_mask, &mut system_mask) != 0 {
            process_mask &= system_mask;
            let mut context = AffinityDialogContext {
                page: self as *mut ProcessPageState,
                process_mask,
            };
            if DialogBoxParamW(
                self.hinstance,
                make_int_resource(IDD_AFFINITY),
                self.hwnd_page,
                Some(affinity_dialog_proc),
                &mut context as *mut AffinityDialogContext as LPARAM,
            ) == IDOK as isize
            {
                if SetProcessAffinityMask(handle, context.process_mask) == 0 {
                    self.show_failure_message(
                        &self.strings.cant_set_affinity,
                        windows_sys::Win32::Foundation::GetLastError(),
                    );
                } else {
                    self.refresh_processes();
                    success = true;
                }
            }
        }

        CloseHandle(handle);
        success
    }

    unsafe fn pick_columns(&mut self, options: &mut Options) {
        let mut context = ColumnDialogContext {
            page: self as *mut ProcessPageState,
            options: options as *mut Options,
        };
        DialogBoxParamW(
            self.hinstance,
            make_int_resource(IDD_SELECTPROCCOLS),
            self.main_hwnd,
            Some(column_select_dialog_proc),
            &mut context as *mut ColumnDialogContext as LPARAM,
        );
    }
}

unsafe extern "system" fn column_select_dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match msg {
        WM_INITDIALOG => {
            set_window_userdata(hwnd, lparam);
            localize_dialog(hwnd, IDD_SELECTPROCCOLS);
            let context = &*(lparam as *const ColumnDialogContext);
            let options = &*context.options;

            for &control_id in &COLUMN_DIALOG_IDS {
                CheckDlgButton(hwnd, control_id, BST_UNCHECKED as u32);
            }
            CheckDlgButton(hwnd, IDC_IMAGENAME, BST_CHECKED as u32);

            for column in columns_from_options(options) {
                CheckDlgButton(hwnd, COLUMN_DIALOG_IDS[column as usize], BST_CHECKED as u32);
            }
            1
        }
        WM_COMMAND => match loword(wparam) as i32 {
            IDOK => {
                let context = &mut *(get_window_userdata(hwnd) as *mut ColumnDialogContext);
                let page = &mut *context.page;
                let options = &mut *context.options;

                page.save_column_widths(options);
                apply_selected_columns(hwnd, options);
                page.active_columns = columns_from_options(options);
                page.setup_columns(options);
                page.refresh_processes();
                EndDialog(hwnd, IDOK as isize);
                1
            }
            IDCANCEL => {
                EndDialog(hwnd, IDCANCEL as isize);
                1
            }
            _ => 0,
        },
        _ => 0,
    }
}

unsafe extern "system" fn affinity_dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match msg {
        WM_INITDIALOG => {
            set_window_userdata(hwnd, lparam);
            localize_dialog(hwnd, IDD_AFFINITY);
            let context = &*(lparam as *const AffinityDialogContext);
            let page = &*context.page;

            for cpu_index in 0..=MAX_AFFINITY_CPU {
                let control_id = IDC_CPU0 + cpu_index;
                let enabled = cpu_index < page.processor_count as i32;
                SendMessageW(GetDlgItem(hwnd, control_id), WM_ENABLE, enabled as usize, 0);
                CheckDlgButton(
                    hwnd,
                    control_id,
                    if enabled && (context.process_mask & (1usize << cpu_index)) != 0 {
                        BST_CHECKED as u32
                    } else {
                        BST_UNCHECKED as u32
                    },
                );
            }
            1
        }
        WM_COMMAND => match loword(wparam) as i32 {
            IDCANCEL => {
                EndDialog(hwnd, IDCANCEL as isize);
                1
            }
            IDOK => {
                let context = &mut *(get_window_userdata(hwnd) as *mut AffinityDialogContext);
                let page = &*context.page;

                context.process_mask = 0;
                for cpu_index in 0..page.processor_count.min((MAX_AFFINITY_CPU + 1) as usize) {
                    if IsDlgButtonChecked(hwnd, IDC_CPU0 + cpu_index as i32) == BST_CHECKED as u32 {
                        context.process_mask |= 1usize << cpu_index;
                    }
                }

                if context.process_mask == 0 {
                    let title_wide = to_wide_null(&page.strings.invalid_option);
                    let body_wide = to_wide_null(&page.strings.no_affinity_mask);
                    MessageBoxW(
                        hwnd,
                        body_wide.as_ptr(),
                        title_wide.as_ptr(),
                        MB_ICONERROR,
                    );
                    1
                } else {
                    EndDialog(hwnd, IDOK as isize);
                    1
                }
            }
            _ => 0,
        },
        _ => 0,
    }
}

unsafe fn apply_selected_columns(hwnd: HWND, options: &mut Options) {
    let mut existing_widths = HashMap::new();
    for (index, value) in options.active_process_columns.iter().copied().enumerate() {
        let Some(column) = column_id_from_i32(value) else {
            break;
        };
        existing_widths.insert(column as i32, options.column_widths[index]);
    }

    for value in options.active_process_columns.iter_mut() {
        *value = -1;
    }
    for value in options.column_widths.iter_mut() {
        *value = -1;
    }

    let mut next_index = 0usize;
    options.active_process_columns[next_index] = ColumnId::ImageName as i32;
    options.column_widths[next_index] = existing_widths
        .get(&(ColumnId::ImageName as i32))
        .copied()
        .filter(|width| *width > 0)
        .unwrap_or(PROCESS_COLUMNS[ColumnId::ImageName as usize].default_width);
    next_index += 1;

    for column_index in 1..NUM_COLUMN {
        if IsDlgButtonChecked(hwnd, COLUMN_DIALOG_IDS[column_index]) == BST_CHECKED as u32 {
            let column = column_id_from_i32(column_index as i32).unwrap_or(ColumnId::Pid);
            options.active_process_columns[next_index] = column as i32;
            options.column_widths[next_index] = existing_widths
                .get(&(column as i32))
                .copied()
                .filter(|width| *width > 0)
                .unwrap_or(PROCESS_COLUMNS[column as usize].default_width);
            next_index += 1;
        }
    }
}

unsafe fn load_debugger_path() -> Option<String> {
    let mut key: HKEY = null_mut();
    let key_name = to_wide_null("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\AeDebug");
    let value_name = to_wide_null("Debugger");
    if RegOpenKeyExW(HKEY_LOCAL_MACHINE, key_name.as_ptr(), 0, KEY_READ, &mut key) != 0 {
        return None;
    }

    let mut buffer = vec![0u16; 1024];
    let mut value_type = 0u32;
    let mut value_size = (buffer.len() * size_of::<u16>()) as u32;
    let status = RegQueryValueExW(
        key,
        value_name.as_ptr(),
        null_mut(),
        &mut value_type,
        buffer.as_mut_ptr() as *mut u8,
        &mut value_size,
    );
    RegCloseKey(key);

    if status != 0 || value_size == 0 {
        return None;
    }

    let length = buffer.iter().position(|value| *value == 0).unwrap_or(buffer.len());
    let raw = String::from_utf16_lossy(&buffer[..length]);
    let executable = extract_first_command_token(&raw);
    if executable.is_empty()
        || executable.eq_ignore_ascii_case("drwtsn32")
        || executable.eq_ignore_ascii_case("drwtsn32.exe")
    {
        None
    } else {
        Some(executable)
    }
}

fn extract_first_command_token(command_line: &str) -> String {
    let trimmed = command_line.trim();
    if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split('"').next().unwrap_or_default().to_string()
    } else {
        trimmed
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string()
    }
}

unsafe fn query_process_identity(process_handle: HANDLE) -> (String, u32) {
    let mut token: HANDLE = null_mut();
    if OpenProcessToken(process_handle, TOKEN_QUERY, &mut token) == 0 || token.is_null() {
        return (String::new(), 0);
    }

    let mut session_id = 0u32;
    let mut session_bytes = size_of::<u32>() as u32;
    let _ = GetTokenInformation(
        token,
        TokenSessionId,
        &mut session_id as *mut _ as *mut _,
        session_bytes,
        &mut session_bytes,
    );

    let mut required = 0u32;
    let _ = GetTokenInformation(token, TokenUser, null_mut(), 0, &mut required);
    if required == 0 {
        CloseHandle(token);
        return (String::new(), session_id);
    }

    let mut buffer = vec![0u8; required as usize];
    let mut user_name = String::new();
    if GetTokenInformation(
        token,
        TokenUser,
        buffer.as_mut_ptr() as *mut _,
        required,
        &mut required,
    ) != 0
    {
        let token_user = &*(buffer.as_ptr() as *const TOKEN_USER);
        user_name = lookup_account_name_from_sid(token_user.User.Sid);
    }

    CloseHandle(token);
    (user_name, session_id)
}

fn well_known_service_name(sid: *mut core::ffi::c_void) -> Option<String> {
    unsafe {
        if IsWellKnownSid(sid, WinLocalSystemSid) != 0 {
            Some("SYSTEM".to_string())
        } else if IsWellKnownSid(sid, WinLocalServiceSid) != 0 {
            Some("LOCAL SERVICE".to_string())
        } else if IsWellKnownSid(sid, WinNetworkServiceSid) != 0 {
            Some("NETWORK SERVICE".to_string())
        } else {
            None
        }
    }
}

unsafe fn lookup_account_name_from_sid(sid: *mut core::ffi::c_void) -> String {
    if sid.is_null() {
        return String::new();
    }

    let mut name_len = 0u32;
    let mut domain_len = 0u32;
    let mut sid_use = 0 as SID_NAME_USE;
    let _ = LookupAccountSidW(
        null_mut(),
        sid,
        null_mut(),
        &mut name_len,
        null_mut(),
        &mut domain_len,
        &mut sid_use,
    );

    if name_len != 0 {
        let mut name = vec![0u16; name_len as usize];
        let mut domain = vec![0u16; domain_len as usize];
        if LookupAccountSidW(
            null_mut(),
            sid,
            name.as_mut_ptr(),
            &mut name_len,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut sid_use,
        ) != 0
        {
            return String::from_utf16_lossy(&name[..name_len as usize]);
        }
    }

    well_known_service_name(sid).unwrap_or_default()
}

unsafe fn collect_process_identity_map() -> HashMap<u32, (String, u32)> {
    let mut identities = HashMap::new();
    let mut process_info = null_mut::<WTS_PROCESS_INFOW>();
    let mut count = 0u32;

    if WTSEnumerateProcessesW(
        WTS_CURRENT_SERVER_HANDLE,
        0,
        1,
        &mut process_info,
        &mut count,
    ) == 0
        || process_info.is_null()
    {
        return identities;
    }

    let processes = slice::from_raw_parts(process_info, count as usize);
    for process in processes {
        let pid = process.ProcessId;
        let user_name = if pid == 0 {
            "SYSTEM".to_string()
        } else {
            lookup_account_name_from_sid(process.pUserSid)
        };
        identities.insert(pid, (user_name, process.SessionId));
    }

    WTSFreeMemory(process_info as _);
    identities
}

fn columns_from_options(options: &Options) -> Vec<ColumnId> {
    options
        .active_process_columns
        .iter()
        .copied()
        .filter_map(column_id_from_i32)
        .collect()
}

fn column_id_from_i32(value: i32) -> Option<ColumnId> {
    match value {
        x if x == ColumnId::ImageName as i32 => Some(ColumnId::ImageName),
        x if x == ColumnId::Pid as i32 => Some(ColumnId::Pid),
        x if x == ColumnId::Username as i32 => Some(ColumnId::Username),
        x if x == ColumnId::SessionId as i32 => Some(ColumnId::SessionId),
        x if x == ColumnId::Cpu as i32 => Some(ColumnId::Cpu),
        x if x == ColumnId::CpuTime as i32 => Some(ColumnId::CpuTime),
        x if x == ColumnId::MemUsage as i32 => Some(ColumnId::MemUsage),
        x if x == ColumnId::MemUsageDiff as i32 => Some(ColumnId::MemUsageDiff),
        x if x == ColumnId::PageFaults as i32 => Some(ColumnId::PageFaults),
        x if x == ColumnId::PageFaultsDiff as i32 => Some(ColumnId::PageFaultsDiff),
        x if x == ColumnId::CommitCharge as i32 => Some(ColumnId::CommitCharge),
        x if x == ColumnId::PagedPool as i32 => Some(ColumnId::PagedPool),
        x if x == ColumnId::NonPagedPool as i32 => Some(ColumnId::NonPagedPool),
        x if x == ColumnId::BasePriority as i32 => Some(ColumnId::BasePriority),
        x if x == ColumnId::HandleCount as i32 => Some(ColumnId::HandleCount),
        x if x == ColumnId::ThreadCount as i32 => Some(ColumnId::ThreadCount),
        _ => None,
    }
}

fn build_sort_context(entries: &[ProcEntry]) -> HashMap<u32, ProcEntry> {
    entries
        .iter()
        .cloned()
        .map(|entry| (entry.pid, entry))
        .collect()
}

fn compare_entries(
    left: &ProcEntry,
    right: &ProcEntry,
    sort_context: &HashMap<u32, ProcEntry>,
    sort_column: ColumnId,
    sort_direction: i32,
) -> Ordering {
    let left_proxy = sort_proxy_entry(left, sort_context);
    let right_proxy = sort_proxy_entry(right, sort_context);

    if left_proxy.pid == right_proxy.pid {
        if left.is_wow_task {
            return if right.is_wow_task {
                left.image_name.to_lowercase().cmp(&right.image_name.to_lowercase())
            } else {
                Ordering::Greater
            };
        }

        if right.is_wow_task {
            return Ordering::Less;
        }
    }

    let ordering = match sort_column {
        ColumnId::ImageName => left_proxy
            .image_name
            .to_lowercase()
            .cmp(&right_proxy.image_name.to_lowercase()),
        ColumnId::Pid => left_proxy.pid.cmp(&right_proxy.pid),
        ColumnId::Username => left_proxy
            .user_name
            .to_lowercase()
            .cmp(&right_proxy.user_name.to_lowercase()),
        ColumnId::SessionId => left_proxy.session_id.cmp(&right_proxy.session_id),
        ColumnId::Cpu => left_proxy.cpu.cmp(&right_proxy.cpu),
        ColumnId::CpuTime => left_proxy.cpu_time_100ns.cmp(&right_proxy.cpu_time_100ns),
        ColumnId::MemUsage => left_proxy.mem_usage_kb.cmp(&right_proxy.mem_usage_kb),
        ColumnId::MemUsageDiff => left_proxy.mem_diff_kb.cmp(&right_proxy.mem_diff_kb),
        ColumnId::PageFaults => left_proxy.page_faults.cmp(&right_proxy.page_faults),
        ColumnId::PageFaultsDiff => left_proxy.page_faults_diff.cmp(&right_proxy.page_faults_diff),
        ColumnId::CommitCharge => left_proxy.commit_charge_kb.cmp(&right_proxy.commit_charge_kb),
        ColumnId::PagedPool => left_proxy.paged_pool_kb.cmp(&right_proxy.paged_pool_kb),
        ColumnId::NonPagedPool => left_proxy.nonpaged_pool_kb.cmp(&right_proxy.nonpaged_pool_kb),
        ColumnId::BasePriority => priority_rank(left_proxy.priority_class)
            .cmp(&priority_rank(right_proxy.priority_class)),
        ColumnId::HandleCount => left_proxy.handle_count.cmp(&right_proxy.handle_count),
        ColumnId::ThreadCount => left_proxy.thread_count.cmp(&right_proxy.thread_count),
    };

    let ordering = if ordering == Ordering::Equal {
        let tie_break = left_proxy.pid.cmp(&right_proxy.pid);
        if sort_direction < 0 {
            tie_break.reverse()
        } else {
            tie_break
        }
    } else if sort_direction < 0 {
        ordering.reverse()
    } else {
        ordering
    };

    ordering
}

fn priority_rank(priority_class: u32) -> u8 {
    match priority_class {
        REALTIME_PRIORITY_CLASS => 3,
        HIGH_PRIORITY_CLASS => 2,
        NORMAL_PRIORITY_CLASS => 1,
        _ => 0,
    }
}

fn sort_proxy_entry<'a>(entry: &'a ProcEntry, sort_context: &'a HashMap<u32, ProcEntry>) -> &'a ProcEntry {
    if entry.is_wow_task {
        sort_context.get(&entry.real_pid).unwrap_or(entry)
    } else {
        entry
    }
}

fn column_text(entry: &ProcEntry, column_id: ColumnId, strings: &ProcessStrings) -> String {
    match column_id {
        ColumnId::ImageName => append_32_bit_suffix(&entry.image_name, entry.is_32_bit),
        ColumnId::Pid => entry.pid.to_string(),
        ColumnId::Username => entry.user_name.clone(),
        ColumnId::SessionId => entry.session_id.to_string(),
        ColumnId::Cpu => format!("{:02} %", entry.cpu),
        ColumnId::CpuTime => format_elapsed_time(entry.display_cpu_time_100ns),
        ColumnId::MemUsage => format_kilobytes(entry.mem_usage_kb),
        ColumnId::MemUsageDiff => format_signed_kilobytes(entry.mem_diff_kb),
        ColumnId::PageFaults => entry.page_faults.to_string(),
        ColumnId::PageFaultsDiff => entry.page_faults_diff.to_string(),
        ColumnId::CommitCharge => format_kilobytes(entry.commit_charge_kb),
        ColumnId::PagedPool => format_kilobytes(entry.paged_pool_kb),
        ColumnId::NonPagedPool => format_kilobytes(entry.nonpaged_pool_kb),
        ColumnId::BasePriority => match entry.priority_class {
            value if value == IDLE_PRIORITY_CLASS => strings.priority_low.clone(),
            value if value == HIGH_PRIORITY_CLASS => strings.priority_high.clone(),
            value if value == REALTIME_PRIORITY_CLASS => strings.priority_realtime.clone(),
            value if value == NORMAL_PRIORITY_CLASS => strings.priority_normal.clone(),
            _ => strings.priority_unknown.clone(),
        },
        ColumnId::HandleCount => entry.handle_count.to_string(),
        ColumnId::ThreadCount => entry.thread_count.to_string(),
    }
}

fn format_elapsed_time(total_100ns: u64) -> String {
    let total_seconds = total_100ns / 10_000_000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{hours:2}:{minutes:02}:{seconds:02}")
}

fn format_kilobytes(value: u32) -> String {
    format!("{value} K")
}

fn format_signed_kilobytes(value: i64) -> String {
    format!("{value} K")
}

fn copy_text_to_callback_buffer(buffer: *mut u16, capacity: usize, text: &str) {
    if buffer.is_null() || capacity == 0 {
        return;
    }

    let max_len = capacity.saturating_sub(1);
    let encoded = text.encode_utf16().take(max_len).collect::<Vec<_>>();

    unsafe {
        std::ptr::copy_nonoverlapping(encoded.as_ptr(), buffer, encoded.len());
        *buffer.add(encoded.len()) = 0;
    }
}

unsafe fn collect_process_entries(
    previous_samples: &HashMap<u32, PreviousProcSample>,
    total_delta: u64,
    show_16bit: bool,
) -> (Vec<ProcEntry>, HashMap<u32, PreviousProcSample>) {
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snapshot == INVALID_HANDLE_VALUE {
        return (Vec::new(), HashMap::new());
    }

    let mut entries = Vec::new();
    let mut next_samples = HashMap::new();
    let identities = collect_process_identity_map();
    let mut process_entry = zeroed::<PROCESSENTRY32W>();
    process_entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

    if Process32FirstW(snapshot, &mut process_entry) != 0 {
        loop {
            let pid = process_entry.th32ProcessID;
            let thread_count = process_entry.cntThreads;
            let image_name = utf16_buffer_to_string(&process_entry.szExeFile);
            let mut entry = ProcEntry {
                pid,
                real_pid: pid,
                image_name,
                is_32_bit: false,
                user_name: String::new(),
                session_id: 0,
                cpu: 0,
                cpu_time_100ns: 0,
                display_cpu_time_100ns: 0,
                mem_usage_kb: 0,
                mem_diff_kb: 0,
                page_faults: 0,
                page_faults_diff: 0,
                commit_charge_kb: 0,
                paged_pool_kb: 0,
                nonpaged_pool_kb: 0,
                priority_class: NORMAL_PRIORITY_CLASS,
                handle_count: 0,
                thread_count,
                wow_task_handle: 0,
                is_wow_task: false,
                pass_count: 0,
                dirty_columns: DirtyColumns::default(),
            };
            let mut raw_cpu_time_100ns = 0u64;

            if let Some((user_name, session_id)) = identities.get(&pid) {
                entry.user_name = user_name.clone();
                entry.session_id = *session_id;
            }

            if pid == 0 {
                entry.user_name = "SYSTEM".to_string();
            } else {
                let identity_handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                if !identity_handle.is_null() {
                    let (user_name, session_id) = query_process_identity(identity_handle);
                    if !user_name.is_empty() {
                        entry.user_name = user_name;
                    }
                    if session_id != 0 || entry.session_id == 0 {
                        entry.session_id = session_id;
                    }
                    entry.is_32_bit = is_32_bit_process_handle(identity_handle);
                    CloseHandle(identity_handle);
                }
            }

            let process_handle =
                OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, 0, pid);
            if !process_handle.is_null() {
                entry.is_32_bit = is_32_bit_process_handle(process_handle);
                let mut creation = zeroed::<FILETIME>();
                let mut exit = zeroed::<FILETIME>();
                let mut kernel = zeroed::<FILETIME>();
                let mut user = zeroed::<FILETIME>();
                if GetProcessTimes(
                    process_handle,
                    &mut creation,
                    &mut exit,
                    &mut kernel,
                    &mut user,
                ) != 0
                {
                    let cpu_time_100ns = filetime_to_u64(kernel).saturating_add(filetime_to_u64(user));
                    let previous = previous_samples.get(&pid).cloned().unwrap_or_default();
                    let delta = cpu_time_100ns.saturating_sub(previous.raw_cpu_time_100ns);
                    raw_cpu_time_100ns = cpu_time_100ns;
                    entry.cpu_time_100ns = cpu_time_100ns;
                    entry.display_cpu_time_100ns = cpu_time_100ns;
                    entry.cpu = cpu_percent_from_delta(delta, total_delta);
                }

                let mut counters = PROCESS_MEMORY_COUNTERS_EX {
                    cb: size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
                    ..zeroed()
                };
                if K32GetProcessMemoryInfo(
                    process_handle,
                    &mut counters as *mut _ as *mut _,
                    size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
                ) != 0
                {
                    let previous = previous_samples.get(&pid).cloned().unwrap_or_default();
                    entry.mem_usage_kb = (counters.WorkingSetSize / 1024) as u32;
                    entry.mem_diff_kb = entry.mem_usage_kb as i64 - previous.mem_usage_kb as i64;
                    entry.page_faults = counters.PageFaultCount;
                    entry.page_faults_diff = entry.page_faults as i64 - previous.page_faults as i64;
                    entry.commit_charge_kb = (counters.PrivateUsage / 1024) as u32;
                    entry.paged_pool_kb = (counters.QuotaPagedPoolUsage / 1024) as u32;
                    entry.nonpaged_pool_kb = (counters.QuotaNonPagedPoolUsage / 1024) as u32;
                }

                let mut handle_count = 0u32;
                if GetProcessHandleCount(process_handle, &mut handle_count) != 0 {
                    entry.handle_count = handle_count;
                }

                let priority_class = GetPriorityClass(process_handle);
                if priority_class != 0 {
                    entry.priority_class = priority_class;
                }

                next_samples.insert(
                    pid,
                    PreviousProcSample {
                        raw_cpu_time_100ns,
                        display_cpu_time_100ns: entry.display_cpu_time_100ns,
                        mem_usage_kb: entry.mem_usage_kb,
                        page_faults: entry.page_faults,
                    },
                );

                CloseHandle(process_handle);
            }

            entries.push(entry);

            if Process32NextW(snapshot, &mut process_entry) == 0 {
                break;
            }
        }
    }

    CloseHandle(snapshot);

    if show_16bit {
        collect_wow_task_entries(previous_samples, total_delta, &mut entries, &mut next_samples);
    }

    (entries, next_samples)
}

unsafe fn collect_wow_task_entries(
    previous_samples: &HashMap<u32, PreviousProcSample>,
    total_delta: u64,
    entries: &mut Vec<ProcEntry>,
    next_samples: &mut HashMap<u32, PreviousProcSample>,
) {
    let Some(vdmdbg) = VdmDbgApi::load() else {
        return;
    };
    let Some(enum_tasks) = vdmdbg.enum_tasks else {
        return;
    };

    let ntvdm_parent_indexes: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| entry.image_name.eq_ignore_ascii_case("ntvdm.exe").then_some(index))
        .collect();

    for parent_index in ntvdm_parent_indexes {
        let parent_pid = entries[parent_index].pid;
        let parent_cpu_time = entries[parent_index].cpu_time_100ns;
        let (parent_display_cpu_time, parent_cpu_percent) = {
            let mut context = WowTaskEnumContext {
                previous_samples,
                next_samples,
                total_delta,
                parent_index,
                parent_pid,
                entries,
                time_left_100ns: parent_cpu_time,
            };
            enum_tasks(
                parent_pid,
                Some(enum_wow_task_proc),
                &mut context as *mut WowTaskEnumContext as LPARAM,
            );

            let previous_parent = previous_samples.get(&parent_pid).cloned().unwrap_or_default();
            let parent_delta = context
                .time_left_100ns
                .saturating_sub(previous_parent.display_cpu_time_100ns);
            (
                context.time_left_100ns,
                cpu_percent_from_delta(parent_delta, total_delta),
            )
        };

        let parent_entry = &mut entries[parent_index];
        parent_entry.display_cpu_time_100ns = parent_display_cpu_time;
        parent_entry.cpu = parent_cpu_percent;
        if let Some(sample) = next_samples.get_mut(&parent_pid) {
            sample.display_cpu_time_100ns = parent_entry.display_cpu_time_100ns;
        }
    }
}

unsafe fn current_system_time() -> u64 {
    let mut idle = zeroed::<FILETIME>();
    let mut kernel = zeroed::<FILETIME>();
    let mut user = zeroed::<FILETIME>();
    if GetSystemTimes(&mut idle, &mut kernel, &mut user) == 0 {
        0
    } else {
        filetime_to_u64(kernel).saturating_add(filetime_to_u64(user))
    }
}

fn filetime_to_u64(filetime: FILETIME) -> u64 {
    ((filetime.dwHighDateTime as u64) << 32) | filetime.dwLowDateTime as u64
}

unsafe fn read_thread_cpu_time_100ns(thread_id: u32) -> Option<u64> {
    let thread = OpenThread(THREAD_QUERY_INFORMATION, 0, thread_id);
    if thread.is_null() {
        return None;
    }

    let mut creation = zeroed::<FILETIME>();
    let mut exit = zeroed::<FILETIME>();
    let mut kernel = zeroed::<FILETIME>();
    let mut user = zeroed::<FILETIME>();
    let ok = GetThreadTimes(
        thread,
        &mut creation,
        &mut exit,
        &mut kernel,
        &mut user,
    );
    CloseHandle(thread);

    if ok == 0 {
        None
    } else {
        Some(filetime_to_u64(kernel).saturating_add(filetime_to_u64(user)))
    }
}

fn utf16_buffer_to_string(buffer: &[u16]) -> String {
    let length = buffer.iter().position(|value| *value == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..length])
}

fn basename_from_path(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

fn cstr_ptr_to_string(ptr: *mut i8) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
    }
}

unsafe fn load_popup_menu(hinstance: HINSTANCE, resource_id: u16) -> HMENU {
    let menu = LoadMenuW(hinstance, make_int_resource(resource_id));
    if menu.is_null() {
        return null_mut();
    }
    localize_menu(menu, resource_id);

    let popup = GetSubMenu(menu, 0);
    RemoveMenu(menu, 0, MF_BYPOSITION);
    DestroyMenu(menu);
    sanitize_task_manager_menu(popup, usize::MAX);
    popup
}

fn cpu_percent_from_delta(delta_100ns: u64, total_delta_100ns: u64) -> u8 {
    if total_delta_100ns == 0 {
        return 0;
    }

    let scaled_total = (total_delta_100ns / 1000).max(1);
    (((delta_100ns / scaled_total) + 5) / 10).min(99) as u8
}

unsafe fn window_rect_relative_to_page(hwnd: HWND, page_hwnd: HWND) -> RECT {
    let mut rect = zeroed::<RECT>();
    windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
    MapWindowPoints(null_mut(), page_hwnd, &mut rect as *mut _ as _, 2);
    rect
}

struct WowTaskEnumContext<'a> {
    previous_samples: &'a HashMap<u32, PreviousProcSample>,
    next_samples: &'a mut HashMap<u32, PreviousProcSample>,
    total_delta: u64,
    parent_index: usize,
    parent_pid: u32,
    entries: &'a mut Vec<ProcEntry>,
    time_left_100ns: u64,
}

unsafe extern "system" fn enum_wow_task_proc(
    thread_id: u32,
    _module16: u16,
    task16: u16,
    _module_name: *mut i8,
    file_name: *mut i8,
    lparam: LPARAM,
) -> i32 {
    let context = &mut *(lparam as *mut WowTaskEnumContext<'_>);
    let cpu_time_100ns = match read_thread_cpu_time_100ns(thread_id) {
        Some(value) => value,
        None => return 0,
    };

    let previous = context
        .previous_samples
        .get(&thread_id)
        .cloned()
        .unwrap_or_default();
    let mut delta_100ns = cpu_time_100ns.saturating_sub(previous.raw_cpu_time_100ns);
    if delta_100ns > context.time_left_100ns {
        delta_100ns = context.time_left_100ns;
    }
    context.time_left_100ns = context.time_left_100ns.saturating_sub(delta_100ns);

    let image_name = format!("  {}", basename_from_path(&cstr_ptr_to_string(file_name)));
    let cpu = cpu_percent_from_delta(delta_100ns, context.total_delta);
    let parent_priority = context.entries[context.parent_index].priority_class;
    let parent_user_name = context.entries[context.parent_index].user_name.clone();
    let parent_session_id = context.entries[context.parent_index].session_id;

    if let Some(existing) = context
        .entries
        .iter_mut()
        .find(|entry| entry.is_wow_task && entry.pid == thread_id)
    {
        existing.real_pid = context.parent_pid;
        existing.image_name = image_name;
        existing.cpu = cpu;
        existing.cpu_time_100ns = cpu_time_100ns;
        existing.display_cpu_time_100ns = cpu_time_100ns;
        existing.priority_class = parent_priority;
        existing.user_name.clone_from(&parent_user_name);
        existing.session_id = parent_session_id;
        existing.handle_count = 0;
        existing.thread_count = 1;
        existing.wow_task_handle = task16;
        existing.pass_count = 0;
        existing.dirty_columns = DirtyColumns::default();
    } else {
        context.entries.push(ProcEntry {
            pid: thread_id,
            real_pid: context.parent_pid,
            image_name,
            is_32_bit: false,
            user_name: parent_user_name,
            session_id: parent_session_id,
            cpu,
            cpu_time_100ns,
            display_cpu_time_100ns: cpu_time_100ns,
            mem_usage_kb: 0,
            mem_diff_kb: 0,
            page_faults: 0,
            page_faults_diff: 0,
            commit_charge_kb: 0,
            paged_pool_kb: 0,
            nonpaged_pool_kb: 0,
            priority_class: parent_priority,
            handle_count: 0,
            thread_count: 1,
            wow_task_handle: task16,
            is_wow_task: true,
            pass_count: 0,
            dirty_columns: DirtyColumns::default(),
        });
    }

    context.next_samples.insert(
        thread_id,
        PreviousProcSample {
            raw_cpu_time_100ns: cpu_time_100ns,
            display_cpu_time_100ns: cpu_time_100ns,
            mem_usage_kb: 0,
            page_faults: 0,
        },
    );
    0
}

impl ProcEntry {
    fn with_pass_count(mut self, pass_count: u64) -> Self {
        self.pass_count = pass_count;
        self.dirty_columns = DirtyColumns::all();
        self
    }
}

fn same_entry_identity(existing: &ProcEntry, snapshot: &ProcEntry) -> bool {
    existing.pid == snapshot.pid && existing.is_wow_task == snapshot.is_wow_task
}

fn update_process_entry(entry: &mut ProcEntry, snapshot: &ProcEntry, pass_count: u64) {
    entry.pass_count = pass_count;

    if entry.real_pid != snapshot.real_pid {
        entry.real_pid = snapshot.real_pid;
    }
    if entry.image_name != snapshot.image_name {
        entry.image_name.clone_from(&snapshot.image_name);
        entry.dirty_columns.mark(ColumnId::ImageName);
    }
    if entry.is_32_bit != snapshot.is_32_bit {
        entry.is_32_bit = snapshot.is_32_bit;
        entry.dirty_columns.mark(ColumnId::ImageName);
    }
    if entry.pid != snapshot.pid {
        entry.pid = snapshot.pid;
        entry.dirty_columns.mark(ColumnId::Pid);
    }
    if entry.user_name != snapshot.user_name {
        entry.user_name.clone_from(&snapshot.user_name);
        entry.dirty_columns.mark(ColumnId::Username);
    }
    if entry.session_id != snapshot.session_id {
        entry.session_id = snapshot.session_id;
        entry.dirty_columns.mark(ColumnId::SessionId);
    }
    if entry.cpu != snapshot.cpu {
        entry.cpu = snapshot.cpu;
        entry.dirty_columns.mark(ColumnId::Cpu);
    }
    if entry.cpu_time_100ns != snapshot.cpu_time_100ns {
        entry.cpu_time_100ns = snapshot.cpu_time_100ns;
    }
    if entry.display_cpu_time_100ns != snapshot.display_cpu_time_100ns {
        entry.display_cpu_time_100ns = snapshot.display_cpu_time_100ns;
        entry.dirty_columns.mark(ColumnId::CpuTime);
    }
    if entry.mem_usage_kb != snapshot.mem_usage_kb {
        entry.mem_usage_kb = snapshot.mem_usage_kb;
        entry.dirty_columns.mark(ColumnId::MemUsage);
    }
    if entry.mem_diff_kb != snapshot.mem_diff_kb {
        entry.mem_diff_kb = snapshot.mem_diff_kb;
        entry.dirty_columns.mark(ColumnId::MemUsageDiff);
    }
    if entry.page_faults != snapshot.page_faults {
        entry.page_faults = snapshot.page_faults;
        entry.dirty_columns.mark(ColumnId::PageFaults);
    }
    if entry.page_faults_diff != snapshot.page_faults_diff {
        entry.page_faults_diff = snapshot.page_faults_diff;
        entry.dirty_columns.mark(ColumnId::PageFaultsDiff);
    }
    if entry.commit_charge_kb != snapshot.commit_charge_kb {
        entry.commit_charge_kb = snapshot.commit_charge_kb;
        entry.dirty_columns.mark(ColumnId::CommitCharge);
    }
    if entry.paged_pool_kb != snapshot.paged_pool_kb {
        entry.paged_pool_kb = snapshot.paged_pool_kb;
        entry.dirty_columns.mark(ColumnId::PagedPool);
    }
    if entry.nonpaged_pool_kb != snapshot.nonpaged_pool_kb {
        entry.nonpaged_pool_kb = snapshot.nonpaged_pool_kb;
        entry.dirty_columns.mark(ColumnId::NonPagedPool);
    }
    if entry.priority_class != snapshot.priority_class {
        entry.priority_class = snapshot.priority_class;
        entry.dirty_columns.mark(ColumnId::BasePriority);
    }
    if entry.handle_count != snapshot.handle_count {
        entry.handle_count = snapshot.handle_count;
        entry.dirty_columns.mark(ColumnId::HandleCount);
    }
    if entry.thread_count != snapshot.thread_count {
        entry.thread_count = snapshot.thread_count;
        entry.dirty_columns.mark(ColumnId::ThreadCount);
    }

    entry.wow_task_handle = snapshot.wow_task_handle;
    entry.is_wow_task = snapshot.is_wow_task;
}

struct VdmDbgApi {
    module: HMODULE,
    enum_tasks: VDMENUMTASKWOWEXPROC,
    terminate_task: VDMTERMINATETASKINWOWPROC,
}

impl VdmDbgApi {
    unsafe fn load() -> Option<Self> {
        let module_name = to_wide_null("vdmdbg.dll");
        let module = LoadLibraryW(module_name.as_ptr());
        if module.is_null() {
            return None;
        }

        let enum_tasks = GetProcAddress(module, b"VDMEnumTaskWOWEx\0".as_ptr())
            .map(|proc_address| std::mem::transmute::<unsafe extern "system" fn() -> isize, VDMENUMTASKWOWEXPROC>(proc_address))
            .flatten();
        let terminate_task = GetProcAddress(module, b"VDMTerminateTaskWOW\0".as_ptr())
            .map(|proc_address| std::mem::transmute::<unsafe extern "system" fn() -> isize, VDMTERMINATETASKINWOWPROC>(proc_address))
            .flatten();

        if enum_tasks.is_none() && terminate_task.is_none() {
            FreeLibrary(module);
            None
        } else {
            Some(Self {
                module,
                enum_tasks,
                terminate_task,
            })
        }
    }
}

impl Drop for VdmDbgApi {
    fn drop(&mut self) {
        unsafe {
            if !self.module.is_null() {
                FreeLibrary(self.module);
            }
        }
    }
}

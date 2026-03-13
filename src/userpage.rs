use std::collections::HashMap;
use std::mem::zeroed;
use std::ptr::null_mut;
use std::slice;

use windows_sys::Win32::Foundation::{GetLastError, HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::System::RemoteDesktop::{
    WTSClientName, WTSConnectQuery, WTSConnected, WTSDisconnected, WTSDisconnectSession,
    WTSDomainName, WTSEnumerateSessionsW, WTSFreeMemory, WTSIdle, WTSInit, WTSListen,
    WTSLogoffSession, WTSQuerySessionInformationW, WTSReset, WTSSendMessageW, WTSShadow,
    WTSUserName, WTSActive, WTSDown, WTS_CONNECTSTATE_CLASS, WTS_CURRENT_SERVER_HANDLE,
    WTS_SESSION_INFOW,
};
use windows_sys::Win32::UI::Controls::{
    LVCFMT_LEFT, LVCFMT_RIGHT, LVCF_FMT, LVCF_SUBITEM, LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_PARAM,
    LVIF_STATE, LVIF_TEXT, LVIS_FOCUSED, LVIS_SELECTED, LVITEMW, LVN_COLUMNCLICK, LVN_ITEMCHANGED, LVNI_SELECTED,
    LVM_DELETECOLUMN, LVM_DELETEITEM, LVM_ENSUREVISIBLE, LVM_GETITEMCOUNT, LVM_GETITEMW,
    LVM_GETNEXTITEM, LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETITEMSTATE, LVM_SETITEMW,
    NMLISTVIEW,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::Graphics::Gdi::MapWindowPoints;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BeginDeferWindowPos, DeferWindowPos, DialogBoxParamW, EndDeferWindowPos, EndDialog,
    GetClientRect, GetDialogBaseUnits, GetDlgItem, GetWindowTextLengthW, GetWindowTextW, IDCANCEL,
    IDNO, IDOK, MB_DEFBUTTON2, MB_ICONEXCLAMATION, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
    MB_TOPMOST, MB_YESNO, MF_BYCOMMAND, MF_CHECKED, MF_DISABLED, MF_GRAYED, MF_BYPOSITION,
    MF_UNCHECKED, MessageBoxW,
    RemoveMenu, SendMessageW, TPM_RETURNCMD, TrackPopupMenuEx, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, WM_COMMAND, WM_INITDIALOG, WM_SETREDRAW,
    LoadMenuW, GetSubMenu, HMENU,
};

use crate::localization::{
    localize_dialog, localize_menu, session_state, text, user_column_titles,
    user_session_column_title, TextKey,
};
use crate::options::Options;
use crate::resource::{
    IDC_MESSAGE_MESSAGE, IDC_MESSAGE_TITLE, IDC_USERLIST, IDD_MESSAGE, IDM_DISCONNECT,
    IDM_LOGOFF, IDM_SENDMESSAGE, IDM_SHOWDOMAINNAMES, IDR_USER_CONTEXT, IDS_TASKMGR,
};
use crate::winutil::{
    get_window_userdata, load_string, loword, make_int_resource, set_window_userdata,
    subclass_list_view, to_wide_null,
};

const DEFSPACING_BASE: i32 = 3;
const DLG_SCALE_X: i32 = 4;

struct UserSessionEntry {
    session_id: u32,
    display_name: String,
    status: String,
    client_name: String,
    session_name: String,
    dirty: bool,
}

#[derive(Default)]
struct MessageDialogResult {
    title: String,
    body: String,
}

#[derive(Default)]
pub struct UserPageState {
    hinstance: isize,
    hwnd: HWND,
    no_title: bool,
    show_domain_names: bool,
    selected_session_id: Option<u32>,
    sessions: Vec<UserSessionEntry>,
    sort_column: usize,
    sort_ascending: bool,
}

impl UserPageState {
    pub fn new() -> Self {
        Self::default()
    }

    pub unsafe fn initialize(&mut self, hwnd: HWND) {
        self.hinstance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(null_mut()) as isize;
        self.hwnd = hwnd;
        let list = self.list_hwnd();
        if !list.is_null() {
            subclass_list_view(list);
        }
        self.configure_columns();
        self.refresh();
        self.size_page();
    }

    pub fn apply_options(&mut self, options: &Options) {
        self.no_title = options.no_title();
    }

    pub fn no_title(&self) -> bool {
        self.no_title
    }

    pub fn show_domain_names(&self) -> bool {
        self.show_domain_names
    }

    pub unsafe fn timer_event(&mut self) {
        self.refresh();
    }

    pub fn destroy(&mut self) {}

    pub unsafe fn size_page(&self) {
        if self.hwnd.is_null() {
            return;
        }
        let mut parent_rect = zeroed::<RECT>();
        GetClientRect(self.hwnd, &mut parent_rect);
        let units = GetDialogBaseUnits() as usize;
        let def_spacing = (DEFSPACING_BASE * loword(units) as i32) / DLG_SCALE_X;
        let mut hdwp = BeginDeferWindowPos(10);
        if hdwp.is_null() {
            return;
        }
        let master_hwnd = GetDlgItem(self.hwnd, IDM_SENDMESSAGE as i32);
        let list_hwnd = self.list_hwnd();
        if master_hwnd.is_null() || list_hwnd.is_null() {
            EndDeferWindowPos(hdwp);
            return;
        }
        let master_rect = window_rect_relative_to_page(master_hwnd, self.hwnd);
        let dx = (parent_rect.right - def_spacing * 2) - master_rect.right;
        let dy = (parent_rect.bottom - def_spacing * 2) - master_rect.bottom;
        let list_rect = window_rect_relative_to_page(list_hwnd, self.hwnd);
        hdwp = DeferWindowPos(
            hdwp,
            list_hwnd,
            null_mut(),
            0,
            0,
            (master_rect.right - list_rect.left + dx).max(0),
            (master_rect.top - list_rect.top + dy - def_spacing).max(0),
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        for control_id in [IDM_DISCONNECT as i32, IDM_LOGOFF as i32, IDM_SENDMESSAGE as i32] {
            let control_hwnd = GetDlgItem(self.hwnd, control_id);
            if control_hwnd.is_null() {
                continue;
            }
            let control_rect = window_rect_relative_to_page(control_hwnd, self.hwnd);
            hdwp = DeferWindowPos(
                hdwp,
                control_hwnd,
                null_mut(),
                control_rect.left + dx,
                control_rect.top + dy,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        EndDeferWindowPos(hdwp);
    }
    pub unsafe fn handle_notify(&mut self, lparam: isize) -> isize {
        let notify = &*(lparam as *const NMLISTVIEW);
        if notify.hdr.idFrom as i32 == IDC_USERLIST {
            if notify.hdr.code == LVN_ITEMCHANGED {
                self.selected_session_id = self.current_selected_session_id();
                self.update_ui_state();
                return 1;
            }
            if notify.hdr.code == LVN_COLUMNCLICK {
                let column = notify.iSubItem.max(0) as usize;
                if self.sort_column == column {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = column;
                    self.sort_ascending = true;
                }
                self.refresh();
                return 1;
            }
        }
        0
    }

    pub unsafe fn handle_command(&mut self, command_id: u16) -> bool {
        match command_id {
            IDM_SENDMESSAGE => {
                self.send_message();
                true
            }
            IDM_DISCONNECT => {
                self.change_session_state(command_id);
                true
            }
            IDM_LOGOFF => {
                self.change_session_state(command_id);
                true
            }
            IDM_SHOWDOMAINNAMES => {
                self.show_domain_names = !self.show_domain_names;
                self.refresh();
                true
            }
            _ => false,
        }
    }

    pub unsafe fn show_context_menu(&mut self, x: i32, y: i32) {
        let selected = self.selected_session_ids();
        if selected.is_empty() {
            return;
        }

        let menu = LoadMenuW(self.hinstance as _, make_int_resource(IDR_USER_CONTEXT));
        if menu.is_null() {
            return;
        }
        localize_menu(menu, IDR_USER_CONTEXT);
        let popup = GetSubMenu(menu, 0);
        RemoveMenu(menu, 0, MF_BYPOSITION);
        windows_sys::Win32::UI::WindowsAndMessaging::DestroyMenu(menu);
        if popup.is_null() {
            return;
        }

        self.update_menu_state(popup, &selected);
        let command = TrackPopupMenuEx(popup, TPM_RETURNCMD, x, y, self.hwnd, null_mut());
        windows_sys::Win32::UI::WindowsAndMessaging::DestroyMenu(popup);
        if command != 0 {
            self.handle_command(command as u16);
        }
    }

    unsafe fn configure_columns(&self) {
        let list = self.list_hwnd();
        if list.is_null() {
            return;
        }

        while SendMessageW(list, LVM_DELETECOLUMN, 0, 0) != 0 {}

        let titles = user_column_titles();
        let columns = [
            (titles[0], 160, LVCFMT_LEFT),
            (titles[1], 80, LVCFMT_RIGHT),
            (titles[2], 90, LVCFMT_LEFT),
            (titles[3], 120, LVCFMT_LEFT),
            (user_session_column_title(), 90, LVCFMT_LEFT),
        ];

        for (index, (title, width, fmt)) in columns.iter().enumerate() {
            let mut title_wide = to_wide_null(title);
            let mut column = LVCOLUMNW {
                mask: LVCF_FMT | LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM,
                fmt: *fmt,
                cx: *width,
                pszText: title_wide.as_mut_ptr(),
                cchTextMax: title_wide.len() as i32,
                iSubItem: index as i32,
                ..zeroed()
            };
            SendMessageW(list, LVM_INSERTCOLUMNW, index, &mut column as *mut _ as isize);
        }
    }

    unsafe fn refresh(&mut self) {
        let previous_selection = self.selected_session_id;
        let mut previous_sessions = HashMap::with_capacity(self.sessions.len());
        for session in self.sessions.drain(..) {
            previous_sessions.insert(session.session_id, session);
        }
        let mut sessions_ptr = null_mut::<WTS_SESSION_INFOW>();
        let mut session_count = 0u32;
        if WTSEnumerateSessionsW(
            WTS_CURRENT_SERVER_HANDLE,
            0,
            1,
            &mut sessions_ptr,
            &mut session_count,
        ) == 0
            || sessions_ptr.is_null()
        {
            self.sessions.clear();
            self.update_listview();
            self.update_ui_state();
            return;
        }

        let mut sessions = Vec::new();
        for session in slice::from_raw_parts(sessions_ptr, session_count as usize) {
            let user_name = query_session_string(session.SessionId, WTSUserName);
            if user_name.is_empty() {
                continue;
            }

            let domain_name = query_session_string(session.SessionId, WTSDomainName);
            let client_name = query_session_string(session.SessionId, WTSClientName);
            let display_name = if domain_name.is_empty() || !self.show_domain_names {
                user_name.clone()
            } else {
                format!("{domain_name}\\{user_name}")
            };

            let mut entry = UserSessionEntry {
                session_id: session.SessionId,
                display_name,
                status: session_state_text(session.State),
                client_name: if client_name.is_empty() {
                    "-".to_string()
                } else {
                    client_name
                },
                session_name: widestr_ptr_to_string(session.pWinStationName).replace("Console", "Console"),
                dirty: true,
            };
            if let Some(previous) = previous_sessions.remove(&entry.session_id) {
                entry.dirty = previous.display_name != entry.display_name
                    || previous.status != entry.status
                    || previous.client_name != entry.client_name
                    || previous.session_name != entry.session_name;
            }
            sessions.push(entry);
        }

        WTSFreeMemory(sessions_ptr as _);

        sessions.sort_by(|left, right| compare_user_sessions(left, right, self.sort_column, self.sort_ascending));
        self.sessions = sessions;
        self.update_listview();

        self.selected_session_id = previous_selection;
        if let Some(session_id) = previous_selection {
            self.restore_selection(session_id);
        } else {
            self.update_ui_state();
        }
    }

    unsafe fn update_listview(&self) {
        let list = self.list_hwnd();
        if list.is_null() {
            return;
        }

        SendMessageW(list, WM_SETREDRAW, 0, 0);

        let mut existing_count = SendMessageW(list, LVM_GETITEMCOUNT, 0, 0) as usize;
        let common_count = existing_count.min(self.sessions.len());

        for index in 0..common_count {
            let session = &self.sessions[index];
            let mut current_item = LVITEMW {
                mask: LVIF_PARAM,
                iItem: index as i32,
                ..zeroed()
            };
            let current_session_id =
                if SendMessageW(list, LVM_GETITEMW, 0, &mut current_item as *mut _ as isize) != 0 {
                    Some(current_item.lParam as u32)
                } else {
                    None
                };

            if current_session_id != Some(session.session_id) {
                self.replace_row(list, index, session);
            } else if session.dirty {
                self.update_row(list, index, session);
            }
        }

        while existing_count > self.sessions.len() {
            existing_count -= 1;
            SendMessageW(list, LVM_DELETEITEM, existing_count, 0);
        }

        for index in common_count..self.sessions.len() {
            self.insert_row(list, index, &self.sessions[index]);
        }

        SendMessageW(list, WM_SETREDRAW, 1, 0);
    }

    unsafe fn insert_row(&self, list: HWND, index: usize, session: &UserSessionEntry) {
        let mut user_name = to_wide_null(&session.display_name);
        let mut item = LVITEMW {
            mask: LVIF_TEXT | LVIF_PARAM,
            iItem: index as i32,
            iSubItem: 0,
            pszText: user_name.as_mut_ptr(),
            cchTextMax: user_name.len() as i32,
            lParam: session.session_id as isize,
            ..zeroed()
        };
        SendMessageW(list, LVM_INSERTITEMW, 0, &mut item as *mut _ as isize);
        self.update_row(list, index, session);
    }

    unsafe fn replace_row(&self, list: HWND, index: usize, session: &UserSessionEntry) {
        let mut user_name = to_wide_null(&session.display_name);
        let mut item = LVITEMW {
            mask: LVIF_TEXT | LVIF_PARAM,
            iItem: index as i32,
            iSubItem: 0,
            pszText: user_name.as_mut_ptr(),
            cchTextMax: user_name.len() as i32,
            lParam: session.session_id as isize,
            ..zeroed()
        };
        SendMessageW(list, LVM_SETITEMW, 0, &mut item as *mut _ as isize);
        self.update_row(list, index, session);
    }

    unsafe fn update_row(&self, list: HWND, index: usize, session: &UserSessionEntry) {
        let row = [
            session.display_name.as_str(),
            "",
            session.status.as_str(),
            session.client_name.as_str(),
            session.session_name.as_str(),
        ];
        for (subitem, text) in row.iter().enumerate() {
            let content = if subitem == 1 {
                session.session_id.to_string()
            } else {
                (*text).to_string()
            };
            let mut value = to_wide_null(&content);
            let mut subitem_item = LVITEMW {
                mask: LVIF_TEXT,
                iItem: index as i32,
                iSubItem: subitem as i32,
                pszText: value.as_mut_ptr(),
                cchTextMax: value.len() as i32,
                ..zeroed()
            };
            SendMessageW(list, LVM_SETITEMW, 0, &mut subitem_item as *mut _ as isize);
        }
    }

    unsafe fn restore_selection(&self, session_id: u32) {
        let list = self.list_hwnd();
        if list.is_null() {
            return;
        }

        for (index, session) in self.sessions.iter().enumerate() {
            if session.session_id != session_id {
                continue;
            }

            let mut item = LVITEMW {
                stateMask: LVIS_SELECTED | LVIS_FOCUSED,
                state: LVIS_SELECTED | LVIS_FOCUSED,
                ..zeroed()
            };
            SendMessageW(list, LVM_SETITEMSTATE, index, &mut item as *mut _ as isize);
            SendMessageW(list, LVM_ENSUREVISIBLE, index, 0);
            break;
        }

        self.update_ui_state();
    }

    unsafe fn current_selected_session_id(&self) -> Option<u32> {
        let list = self.list_hwnd();
        if list.is_null() {
            return None;
        }

        let index = SendMessageW(list, LVM_GETNEXTITEM, usize::MAX, LVNI_SELECTED as isize) as i32;
        if index < 0 {
            return None;
        }

        let mut item = LVITEMW {
            mask: LVIF_PARAM | LVIF_STATE,
            iItem: index,
            ..zeroed()
        };
        if SendMessageW(list, LVM_GETITEMW, 0, &mut item as *mut _ as isize) != 0 {
            Some(item.lParam as u32)
        } else {
            None
        }
    }

    unsafe fn update_ui_state(&self) {
        let selected = self.selected_session_ids();
        let mut send_enabled = !selected.is_empty();
        let mut disconnect_enabled = !selected.is_empty();
        let logoff_enabled = !selected.is_empty();

        for session_id in &selected {
            if let Some(session) = self.sessions.iter().find(|entry| entry.session_id == *session_id) {
                if session.status == session_state("Disconnected") {
                    disconnect_enabled = false;
                }
                if Some(*session_id) == self.selected_session_id && selected.len() == 1 {
                    send_enabled = false;
                }
            }
        }

        for control_id in [IDM_DISCONNECT, IDM_LOGOFF, IDM_SENDMESSAGE] {
            let control = GetDlgItem(self.hwnd, control_id as i32);
            if !control.is_null() {
                let enabled = match control_id {
                    IDM_DISCONNECT => disconnect_enabled,
                    IDM_LOGOFF => logoff_enabled,
                    IDM_SENDMESSAGE => send_enabled,
                    _ => false,
                };
                EnableWindow(control, enabled as i32);
            }
        }
    }

    unsafe fn selected_session_ids(&self) -> Vec<u32> {
        let list = self.list_hwnd();
        if list.is_null() {
            return Vec::new();
        }

        let mut selected = Vec::new();
        let mut index = -1;
        loop {
            index = SendMessageW(
                list,
                LVM_GETNEXTITEM,
                index.max(-1) as usize,
                LVNI_SELECTED as isize,
            ) as i32;
            if index < 0 {
                break;
            }

            let mut item = LVITEMW {
                mask: LVIF_PARAM,
                iItem: index,
                ..zeroed()
            };
            if SendMessageW(list, LVM_GETITEMW, 0, &mut item as *mut _ as isize) != 0 {
                selected.push(item.lParam as u32);
            }
        }
        selected
    }

    unsafe fn update_menu_state(&self, popup: HMENU, selected: &[u32]) {
        let mut send_enabled = !selected.is_empty();
        let mut disconnect_enabled = !selected.is_empty();
        let logoff_enabled = !selected.is_empty();

        for session_id in selected {
            if let Some(session) = self.sessions.iter().find(|entry| entry.session_id == *session_id) {
                if session.status == session_state("Disconnected") {
                    disconnect_enabled = false;
                }
                if Some(*session_id) == self.selected_session_id && selected.len() == 1 {
                    send_enabled = false;
                }
            }
        }

        if !send_enabled {
            windows_sys::Win32::UI::WindowsAndMessaging::EnableMenuItem(
                popup,
                IDM_SENDMESSAGE as u32,
                MF_BYCOMMAND | MF_GRAYED | MF_DISABLED,
            );
        }
        if !disconnect_enabled {
            windows_sys::Win32::UI::WindowsAndMessaging::EnableMenuItem(
                popup,
                IDM_DISCONNECT as u32,
                MF_BYCOMMAND | MF_GRAYED | MF_DISABLED,
            );
        }
        if !logoff_enabled {
            windows_sys::Win32::UI::WindowsAndMessaging::EnableMenuItem(
                popup,
                IDM_LOGOFF as u32,
                MF_BYCOMMAND | MF_GRAYED | MF_DISABLED,
            );
        }
        windows_sys::Win32::UI::WindowsAndMessaging::CheckMenuItem(
            popup,
            IDM_SHOWDOMAINNAMES as u32,
            MF_BYCOMMAND
                | if self.show_domain_names {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                },
        );
    }

    unsafe fn send_message(&mut self) {
        let selected = self.selected_session_ids();
        if selected.is_empty() {
            return;
        }

        let mut result = MessageDialogResult::default();
        if DialogBoxParamW(
            self.hinstance as _,
            make_int_resource(IDD_MESSAGE),
            self.hwnd,
            Some(message_dialog_proc),
            &mut result as *mut _ as LPARAM,
        ) != IDOK as isize
        {
            return;
        }

        let title = to_wide_null(&result.title);
        let body = to_wide_null(&result.body);
        for session_id in selected {
            let mut response = 0i32;
            if WTSSendMessageW(
                WTS_CURRENT_SERVER_HANDLE,
                session_id,
                title.as_ptr(),
                (result.title.encode_utf16().count() * 2) as u32,
                body.as_ptr(),
                (result.body.encode_utf16().count() * 2) as u32,
                MB_OK | MB_TOPMOST | MB_ICONINFORMATION,
                0,
                &mut response,
                0,
            ) == 0
            {
                self.show_command_failure(text(TextKey::MessageCouldNotBeSent));
                break;
            }
        }
    }

    unsafe fn change_session_state(&mut self, command_id: u16) {
        let selected = self.selected_session_ids();
        if selected.is_empty() {
            return;
        }

        let prompt = if command_id == IDM_LOGOFF {
            text(TextKey::ConfirmLogoffSelectedUsers)
        } else {
            text(TextKey::ConfirmDisconnectSelectedUsers)
        };
        let prompt_wide = to_wide_null(prompt);
        let caption_wide = to_wide_null(&load_string(self.hinstance as _, IDS_TASKMGR));
        if MessageBoxW(
            self.hwnd,
            prompt_wide.as_ptr(),
            caption_wide.as_ptr(),
            MB_YESNO | MB_DEFBUTTON2 | MB_ICONEXCLAMATION,
        ) == IDNO
        {
            return;
        }

        for session_id in selected {
            let succeeded = if command_id == IDM_LOGOFF {
                WTSLogoffSession(WTS_CURRENT_SERVER_HANDLE, session_id, 0) != 0
            } else {
                WTSDisconnectSession(WTS_CURRENT_SERVER_HANDLE, session_id, 0) != 0
            };
            if !succeeded {
                self.show_command_failure(if command_id == IDM_LOGOFF {
                    text(TextKey::SelectedUserCouldNotBeLoggedOff)
                } else {
                    text(TextKey::SelectedUserCouldNotBeDisconnected)
                });
                break;
            }
        }

        self.refresh();
    }

    unsafe fn show_command_failure(&self, message: &str) {
        let last_error = GetLastError();
        let body = if last_error == 0 {
            message.to_string()
        } else {
            format!(
                "{}\n\n{} {last_error}",
                message,
                text(TextKey::Win32ErrorPrefix)
            )
        };
        let body_wide = to_wide_null(&body);
        let caption_wide = to_wide_null(&load_string(self.hinstance as _, IDS_TASKMGR));
        MessageBoxW(
            self.hwnd,
            body_wide.as_ptr(),
            caption_wide.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }

    fn list_hwnd(&self) -> HWND {
        unsafe { GetDlgItem(self.hwnd, IDC_USERLIST) }
    }
}

unsafe fn window_rect_relative_to_page(hwnd: HWND, page_hwnd: HWND) -> RECT {
    let mut rect = zeroed::<RECT>();
    windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
    MapWindowPoints(null_mut(), page_hwnd, &mut rect as *mut _ as _, 2);
    rect
}
unsafe fn query_session_string(session_id: u32, info_class: i32) -> String {
    let mut buffer = null_mut();
    let mut bytes = 0u32;
    if WTSQuerySessionInformationW(
        WTS_CURRENT_SERVER_HANDLE,
        session_id,
        info_class,
        &mut buffer,
        &mut bytes,
    ) == 0
        || buffer.is_null()
        || bytes == 0
    {
        return String::new();
    }

    let len = (bytes as usize / 2).saturating_sub(1);
    let value = String::from_utf16_lossy(slice::from_raw_parts(buffer, len));
    WTSFreeMemory(buffer as _);
    value
}

fn compare_user_sessions(
    left: &UserSessionEntry,
    right: &UserSessionEntry,
    sort_column: usize,
    sort_ascending: bool,
) -> std::cmp::Ordering {
    let ordering = match sort_column {
        1 => left.session_id.cmp(&right.session_id),
        2 => left.status.to_lowercase().cmp(&right.status.to_lowercase()),
        3 => left
            .client_name
            .to_lowercase()
            .cmp(&right.client_name.to_lowercase()),
        4 => left
            .session_name
            .to_lowercase()
            .cmp(&right.session_name.to_lowercase()),
        _ => left
            .display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase()),
    };

    if sort_ascending {
        ordering
    } else {
        ordering.reverse()
    }
}

unsafe fn widestr_ptr_to_string(text: *const u16) -> String {
    if text.is_null() {
        return String::new();
    }

    let mut len = 0usize;
    while *text.add(len) != 0 {
        len += 1;
    }

    String::from_utf16_lossy(slice::from_raw_parts(text, len))
}

fn session_state_text(state: WTS_CONNECTSTATE_CLASS) -> String {
    if state == WTSActive {
        session_state("Active").to_string()
    } else if state == WTSConnected {
        session_state("Connected").to_string()
    } else if state == WTSConnectQuery {
        session_state("Connect Query").to_string()
    } else if state == WTSShadow {
        session_state("Shadow").to_string()
    } else if state == WTSDisconnected {
        session_state("Disconnected").to_string()
    } else if state == WTSIdle {
        session_state("Idle").to_string()
    } else if state == WTSListen {
        session_state("Listening").to_string()
    } else if state == WTSReset {
        session_state("Reset").to_string()
    } else if state == WTSDown {
        session_state("Down").to_string()
    } else if state == WTSInit {
        session_state("Init").to_string()
    } else {
        session_state("Unknown").to_string()
    }
}

unsafe extern "system" fn message_dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match msg {
        WM_INITDIALOG => {
            set_window_userdata(hwnd, lparam);
            localize_dialog(hwnd, IDD_MESSAGE);
            1
        }
        WM_COMMAND => match loword(wparam) as i32 {
            IDOK => {
                let result = &mut *(get_window_userdata(hwnd) as *mut MessageDialogResult);
                result.title = get_dialog_item_text(hwnd, IDC_MESSAGE_TITLE);
                result.body = get_dialog_item_text(hwnd, IDC_MESSAGE_MESSAGE);
                if result.body.trim().is_empty() {
                    return 1;
                }
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

unsafe fn get_dialog_item_text(hwnd: HWND, control_id: i32) -> String {
    let control = GetDlgItem(hwnd, control_id);
    if control.is_null() {
        return String::new();
    }

    let length = GetWindowTextLengthW(control);
    if length <= 0 {
        return String::new();
    }

    let mut buffer = vec![0u16; length as usize + 1];
    let actual = GetWindowTextW(control, buffer.as_mut_ptr(), buffer.len() as i32);
    String::from_utf16_lossy(&buffer[..actual as usize])
}

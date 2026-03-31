use std::mem::{size_of, zeroed};

// 界面本地化辅助模块。
// 这里负责在菜单和对话框资源创建后，按当前语言把可见文本替换成对应翻译。

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetDlgItem, GetSubMenu, HMENU, MENUITEMINFOW, MIIM_STRING, SetDlgItemTextW, SetMenuItemInfoW,
    SetWindowTextW, IDCANCEL, IDOK,
};

use crate::localization::{text, TextKey};
use crate::resource::*;
use crate::winutil::to_wide_null;

pub unsafe fn localize_dialog(hwnd: HWND, dialog_id: u16) {
    // 对话框本地化按资源 ID 分发，确保同一个模板在不同语言下仍复用相同控件编号。
    if hwnd.is_null() {
        return;
    }

    match dialog_id {
        IDD_TASKPAGE => {
            set_dialog_item_text(hwnd, IDC_SWITCHTO, TextKey::SwitchTo);
            set_dialog_item_text(hwnd, IDC_ENDTASK, TextKey::EndTask);
            set_dialog_item_text(hwnd, IDM_RUN as i32, TextKey::NewTaskButton);
        }
        IDD_PROCPAGE => {
            set_dialog_item_text(hwnd, IDC_TERMINATE, TextKey::EndProcess);
        }
        IDD_NETPAGE => {
            set_dialog_item_text(hwnd, IDC_NOADAPTERS, TextKey::NoActiveNetworkAdaptersFound);
        }
        IDD_USERSPAGE => {
            set_dialog_item_text(hwnd, IDM_DISCONNECT as i32, TextKey::Disconnect);
            set_dialog_item_text(hwnd, IDM_LOGOFF as i32, TextKey::Logoff);
            set_dialog_item_text(hwnd, IDM_SENDMESSAGE as i32, TextKey::SendMessage);
        }
        IDD_PERFPAGE => {
            set_dialog_item_text(hwnd, IDC_STATIC14, TextKey::Handles);
            set_dialog_item_text(hwnd, IDC_STATIC15, TextKey::Threads);
            set_dialog_item_text(hwnd, IDC_STATIC16, TextKey::ProcessesLabel);
            set_dialog_item_text(hwnd, IDC_STATIC2, TextKey::Total);
            set_dialog_item_text(hwnd, IDC_STATIC3, TextKey::Available);
            set_dialog_item_text(hwnd, IDC_STATIC4, TextKey::FileCache);
            set_dialog_item_text(hwnd, IDC_STATIC6, TextKey::Total);
            set_dialog_item_text(hwnd, IDC_STATIC8, TextKey::Limit);
            set_dialog_item_text(hwnd, IDC_STATIC9, TextKey::Peak);
            set_dialog_item_text(hwnd, IDC_STATIC11, TextKey::Total);
            set_dialog_item_text(hwnd, IDC_STATIC12, TextKey::Paged);
            set_dialog_item_text(hwnd, IDC_STATIC17, TextKey::Nonpaged);
            set_control_text(GetDlgItem(hwnd, IDC_CPUFRAME), TextKey::CpuUsageHistory);
            set_control_text(GetDlgItem(hwnd, IDC_CPUUSAGEFRAME), TextKey::CpuUsage);
            set_control_text(GetDlgItem(hwnd, IDC_MEMBARFRAME), TextKey::MemUsage);
            set_control_text(GetDlgItem(hwnd, IDC_MEMFRAME), TextKey::MemoryUsageHistory);
            set_control_text(GetDlgItem(hwnd, IDC_STATIC1), TextKey::PhysicalMemoryK);
            set_control_text(GetDlgItem(hwnd, IDC_STATIC5), TextKey::CommitChargeK);
            set_control_text(GetDlgItem(hwnd, IDC_STATIC10), TextKey::KernelMemoryK);
            set_control_text(GetDlgItem(hwnd, IDC_STATIC13), TextKey::Totals);
        }
        IDD_SELECTPROCCOLS => {
            set_window_text(hwnd, TextKey::SelectColumnsTitle);
            set_dialog_item_text(hwnd, IDOK, TextKey::Ok);
            set_dialog_item_text(hwnd, IDCANCEL, TextKey::Cancel);
            set_dialog_item_text(hwnd, IDC_SELECTPROCCOLS_DESC, TextKey::SelectProcessColumnsDescription);
            set_dialog_item_text(hwnd, IDC_IMAGENAME, TextKey::ImageName);
            set_dialog_item_text(hwnd, IDC_PID, TextKey::PidProcessIdentifier);
            set_dialog_item_text(hwnd, IDC_USERNAME, TextKey::UserName);
            set_dialog_item_text(hwnd, IDC_SESSIONID, TextKey::SessionId);
            set_dialog_item_text(hwnd, IDC_CPU, TextKey::CpuUsage);
            set_dialog_item_text(hwnd, IDC_CPUTIME, TextKey::CpuTime);
            set_dialog_item_text(hwnd, IDC_MEMUSAGE, TextKey::MemoryUsage);
            set_dialog_item_text(hwnd, IDC_MEMUSAGEDIFF, TextKey::MemoryUsageDelta);
            set_dialog_item_text(hwnd, IDC_PAGEFAULTS, TextKey::PageFaults);
            set_dialog_item_text(hwnd, IDC_PAGEFAULTSDIFF, TextKey::PageFaultsDelta);
            set_dialog_item_text(hwnd, IDC_COMMITCHARGE, TextKey::VirtualMemorySize);
            set_dialog_item_text(hwnd, IDC_PAGEDPOOL, TextKey::PagedPool);
            set_dialog_item_text(hwnd, IDC_NONPAGEDPOOL, TextKey::NonPagedPool);
            set_dialog_item_text(hwnd, IDC_BASEPRIORITY, TextKey::BasePriority);
            set_dialog_item_text(hwnd, IDC_HANDLECOUNT, TextKey::HandleCount);
            set_dialog_item_text(hwnd, IDC_THREADCOUNT, TextKey::ThreadCount);
        }
        IDD_AFFINITY => {
            set_window_text(hwnd, TextKey::ProcessorAffinity);
            set_dialog_item_text(hwnd, IDOK, TextKey::Ok);
            set_dialog_item_text(hwnd, IDCANCEL, TextKey::Cancel);
            set_dialog_item_text(hwnd, IDC_AFFINITY_GROUP, TextKey::Processors);
            set_dialog_item_text(hwnd, IDC_AFFINITY_DESC, TextKey::ProcessorAffinityDescription);
        }
        IDD_MESSAGE => {
            set_window_text(hwnd, TextKey::SendMessageTitle);
            set_dialog_item_text(hwnd, IDOK, TextKey::Ok);
            set_dialog_item_text(hwnd, IDCANCEL, TextKey::Cancel);
            set_dialog_item_text(hwnd, IDC_MESSAGE_TITLE_LABEL, TextKey::MessageTitleLabel);
            set_dialog_item_text(hwnd, IDC_MESSAGE_BODY_LABEL, TextKey::MessageLabel);
        }
        _ => {}
    }
}

pub unsafe fn localize_menu(menu: HMENU, resource_id: u16) {
    // 菜单本地化分两层：
    // 先替换所有命令项文本，再按资源结构修正各级子菜单标题。
    if menu.is_null() {
        return;
    }

    for &(command_id, text_key) in MENU_TEXTS {
        set_menu_by_command(menu, command_id, text_key);
    }

    match resource_id {
        IDR_MAINMENU_TASK => {
            set_submenu(menu, &[0], TextKey::File);
            set_submenu(menu, &[1], TextKey::Options);
            set_submenu(menu, &[2], TextKey::View);
            set_submenu(menu, &[3], TextKey::Windows);
            set_submenu(menu, &[4], TextKey::Help);
            set_submenu(menu, &[2, 1], TextKey::UpdateSpeed);
        }
        IDR_MAINMENU_PROC => {
            set_submenu(menu, &[0], TextKey::File);
            set_submenu(menu, &[1], TextKey::Options);
            set_submenu(menu, &[2], TextKey::View);
            set_submenu(menu, &[3], TextKey::Help);
            set_submenu(menu, &[2, 1], TextKey::UpdateSpeed);
        }
        IDR_MAINMENU_PERF => {
            set_submenu(menu, &[0], TextKey::File);
            set_submenu(menu, &[1], TextKey::Options);
            set_submenu(menu, &[2], TextKey::View);
            set_submenu(menu, &[3], TextKey::Help);
            set_submenu(menu, &[2, 1], TextKey::UpdateSpeed);
            set_submenu(menu, &[2, 3], TextKey::CpuHistory);
        }
        IDR_MAINMENU_NET | IDR_MAINMENU_USER => {
            set_submenu(menu, &[0], TextKey::File);
            set_submenu(menu, &[1], TextKey::Options);
            set_submenu(menu, &[2], TextKey::View);
            set_submenu(menu, &[3], TextKey::Help);
            set_submenu(menu, &[2, 1], TextKey::UpdateSpeed);
        }
        IDR_TASK_CONTEXT | IDR_TASKVIEW => set_submenu(menu, &[0], TextKey::Tasks),
        IDR_PROC_CONTEXT => {
            set_submenu(menu, &[0], TextKey::Processes);
            set_submenu(menu, &[0, 3], TextKey::SetPriority);
        }
        IDR_USER_CONTEXT => set_submenu(menu, &[0], TextKey::Users),
        IDR_TRAYMENU => set_submenu(menu, &[0], TextKey::TaskManager),
        _ => {}
    }
}

unsafe fn set_window_text(hwnd: HWND, text_key: TextKey) {
    let wide = to_wide_null(text(text_key));
    SetWindowTextW(hwnd, wide.as_ptr());
}

unsafe fn set_dialog_item_text(hwnd: HWND, control_id: i32, text_key: TextKey) {
    let wide = to_wide_null(text(text_key));
    SetDlgItemTextW(hwnd, control_id, wide.as_ptr());
}

unsafe fn set_control_text(hwnd: HWND, text_key: TextKey) {
    if hwnd.is_null() {
        return;
    }
    let wide = to_wide_null(text(text_key));
    SetWindowTextW(hwnd, wide.as_ptr());
}

unsafe fn set_submenu(menu: HMENU, path: &[i32], text_key: TextKey) {
    // `path` 表示一条“第几个子菜单 -> 其下第几个子菜单”的路径，
    // 这样不同资源结构可以复用同一个小工具函数。
    if path.is_empty() {
        return;
    }

    let mut parent = menu;
    for position in &path[..path.len() - 1] {
        parent = GetSubMenu(parent, *position);
        if parent.is_null() {
            return;
        }
    }

    let mut wide = to_wide_null(text(text_key));
    let mut info = MENUITEMINFOW {
        cbSize: size_of::<MENUITEMINFOW>() as u32,
        fMask: MIIM_STRING,
        dwTypeData: wide.as_mut_ptr(),
        cch: wide.len() as u32,
        ..zeroed()
    };
    if let Some(last) = path.last() {
        let _ = SetMenuItemInfoW(parent, *last as u32, 1, &mut info);
    }
}

unsafe fn set_menu_by_command(menu: HMENU, command_id: u16, text_key: TextKey) {
    // 命令项通过 command id 直接定位，适合不同菜单资源里复用的同名命令。
    let mut wide = to_wide_null(text(text_key));
    let mut info = MENUITEMINFOW {
        cbSize: size_of::<MENUITEMINFOW>() as u32,
        fMask: MIIM_STRING,
        dwTypeData: wide.as_mut_ptr(),
        cch: wide.len() as u32,
        ..zeroed()
    };
    let _ = SetMenuItemInfoW(menu, command_id as u32, 0, &mut info);
}

const MENU_TEXTS: &[(u16, TextKey)] = &[
    (IDM_RUN, TextKey::NewTaskMenu),
    (IDM_EXIT, TextKey::ExitTaskManager),
    (IDM_ALWAYSONTOP, TextKey::AlwaysOnTop),
    (IDM_MINIMIZEONUSE, TextKey::MinimizeOnUse),
    (IDM_LARGEICONS, TextKey::LargeIcons),
    (IDM_SMALLICONS, TextKey::SmallIcons),
    (IDM_DETAILS, TextKey::Details),
    (IDM_ALLCPUS, TextKey::OneGraphAllCpus),
    (IDM_MULTIGRAPH, TextKey::OneGraphPerCpu),
    (IDM_ABOUT, TextKey::AboutTaskManager),
    (IDM_HIGH, TextKey::High),
    (IDM_NORMAL, TextKey::Normal),
    (IDM_LOW, TextKey::Low),
    (IDM_PAUSED, TextKey::Paused),
    (IDM_CONFIRMATIONS, TextKey::Confirmations),
    (IDM_PROC_DEBUG, TextKey::Debug),
    (IDM_PROC_TERMINATE, TextKey::EndProcess),
    (IDM_PROC_REALTIME, TextKey::Realtime),
    (IDM_PROC_HIGH, TextKey::High),
    (IDM_PROC_NORMAL, TextKey::Normal),
    (IDM_PROC_LOW, TextKey::Low),
    (IDM_HELP, TextKey::HelpTopics),
    (IDM_PROCCOLS, TextKey::SelectColumnsMenu),
    (IDM_USERCOLS, TextKey::SelectColumnsMenu),
    (IDM_REFRESH, TextKey::RefreshNow),
    (IDM_AFFINITY, TextKey::SetAffinity),
    (IDM_KERNELTIMES, TextKey::ShowKernelTimes),
    (IDM_HIDEWHENMIN, TextKey::HideWhenMinimized),
    (IDM_NOTITLE, TextKey::NoTitle),
    (IDM_SENDMESSAGE, TextKey::SendMessage),
    (IDM_DISCONNECT, TextKey::Disconnect),
    (IDM_LOGOFF, TextKey::Logoff),
    (IDM_SHOWDOMAINNAMES, TextKey::ShowFullAccountName),
    (IDM_TASK_MINIMIZE, TextKey::Minimize),
    (IDM_TASK_MAXIMIZE, TextKey::Maximize),
    (IDM_TASK_CASCADE, TextKey::Cascade),
    (IDM_TASK_TILEHORZ, TextKey::TileHorizontally),
    (IDM_TASK_TILEVERT, TextKey::TileVertically),
    (IDM_TASK_SWITCHTO, TextKey::SwitchTo),
    (IDM_TASK_BRINGTOFRONT, TextKey::BringToFront),
    (IDM_TASK_ENDTASK, TextKey::EndTask),
    (IDM_TASK_FINDPROCESS, TextKey::GoToProcess),
    (IDM_RESTORETASKMAN, TextKey::RestoreTaskManager),
];

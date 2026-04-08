use std::sync::OnceLock;

// 语言入口模块。
// 其它模块只通过这里按资源 ID 或文本键取字符串，不必关心当前语言表
// 实际来自哪个语言文件。

use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

#[path = "localization/de.rs"]
mod de;
#[path = "localization/en_us.rs"]
mod en_us;
#[path = "localization/es.rs"]
mod es;
#[path = "localization/fr.rs"]
mod fr;
#[path = "localization_terms.rs"]
mod language_terms;
#[path = "localization_ui.rs"]
mod language_ui;
#[path = "localization/pt.rs"]
mod pt;
#[path = "localization/ru.rs"]
mod ru;
#[path = "localization/text_key.rs"]
mod text_key;
#[path = "localization/zh_cn.rs"]
mod zh_cn;
#[path = "localization/zh_tw.rs"]
mod zh_tw;

pub use language_terms::{
    adapter_state, network_column_titles, session_state, user_column_titles,
    user_session_column_title,
};
pub use language_ui::localize_dialog;
pub use text_key::TextKey;

const LANG_CHINESE: u16 = 0x04;
const LANG_GERMAN: u16 = 0x07;
const LANG_SPANISH: u16 = 0x0a;
const LANG_FRENCH: u16 = 0x0c;
const LANG_PORTUGUESE: u16 = 0x16;
const LANG_RUSSIAN: u16 = 0x19;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiLanguage {
    EnUs,
    ZhCn,
    ZhTw,
    Ru,
    De,
    Fr,
    Pt,
    Es,
}

static UI_LANGUAGE: OnceLock<UiLanguage> = OnceLock::new();

pub fn current_language() -> UiLanguage {
    // 语言探测只做一次，后续都走缓存，避免每次查字符串都调用系统 API。
    *UI_LANGUAGE.get_or_init(|| unsafe {
        let lang_id = GetUserDefaultUILanguage();
        let primary = lang_id & 0x03ff;
        let sub = (lang_id >> 10) & 0x003f;
        match primary {
            LANG_CHINESE => match sub {
                0x02 | 0x04 => UiLanguage::ZhCn,
                0x01 | 0x03 | 0x05 => UiLanguage::ZhTw,
                _ => UiLanguage::ZhCn,
            },
            LANG_RUSSIAN => UiLanguage::Ru,
            LANG_GERMAN => UiLanguage::De,
            LANG_FRENCH => UiLanguage::Fr,
            LANG_PORTUGUESE => UiLanguage::Pt,
            LANG_SPANISH => UiLanguage::Es,
            _ => UiLanguage::EnUs,
        }
    })
}

pub fn localized_string(id: u32) -> Option<&'static str> {
    // 先查当前语言，缺失时回退到英文，保证 UI 至少有稳定文本可显示。
    let text = match current_language() {
        UiLanguage::EnUs => en_us::resource(id),
        UiLanguage::ZhCn => zh_cn::resource(id),
        UiLanguage::ZhTw => zh_tw::resource(id),
        UiLanguage::Ru => ru::resource(id),
        UiLanguage::De => de::resource(id),
        UiLanguage::Fr => fr::resource(id),
        UiLanguage::Pt => pt::resource(id),
        UiLanguage::Es => es::resource(id),
    };
    if !text.is_empty() {
        Some(text)
    } else {
        let fallback = en_us::resource(id);
        if fallback.is_empty() {
            None
        } else {
            Some(fallback)
        }
    }
}

pub fn text(key: TextKey) -> &'static str {
    // 文本键是新的首选入口；它比裸资源 ID 更类型安全、更容易维护。
    match current_language() {
        UiLanguage::EnUs => en_us::text(key),
        UiLanguage::ZhCn => zh_cn::text(key),
        UiLanguage::ZhTw => zh_tw::text(key),
        UiLanguage::Ru => ru::text(key),
        UiLanguage::De => de::text(key),
        UiLanguage::Fr => fr::text(key),
        UiLanguage::Pt => pt::text(key),
        UiLanguage::Es => es::text(key),
    }
}

pub fn menu_status_help(command_id: u16) -> Option<&'static str> {
    // 这部分帮助文本暂时仍以英文为主，作用是兼容旧版状态栏提示行为。
    match command_id {
        crate::resource::IDM_RUN => Some("Runs a new program"),
        crate::resource::IDM_EXIT => Some("Exits the Task Manager application"),
        crate::resource::IDM_ALWAYSONTOP => {
            Some("Task Manager remains in front of all other windows unless minimized")
        }
        crate::resource::IDM_MINIMIZEONUSE => {
            Some("Task Manager is minimized when a SwitchTo operation is performed")
        }
        crate::resource::IDM_LARGEICONS => Some("Displays tasks by using large icons"),
        crate::resource::IDM_SMALLICONS => Some("Displays tasks by using small icons"),
        crate::resource::IDM_DETAILS => Some("Displays information about each task"),
        crate::resource::IDM_ALLCPUS => Some("A single history graph shows total CPU usage"),
        crate::resource::IDM_MULTIGRAPH => Some("Each CPU has its own history graph"),
        crate::resource::IDM_ABOUT => {
            Some("Displays program information, version number, and copyright")
        }
        crate::resource::IDM_HIGH => Some("Updates the display twice per second"),
        crate::resource::IDM_NORMAL => Some("Updates the display every two seconds"),
        crate::resource::IDM_LOW => Some("Updates the display every four seconds"),
        crate::resource::IDM_PAUSED => Some("Display does not automatically update"),
        crate::resource::IDM_CONFIRMATIONS => {
            Some("Task Manager will prompt before modifying processes")
        }
        crate::resource::IDM_PROC_DEBUG => Some("Attaches the debugger to this process"),
        crate::resource::IDM_PROC_TERMINATE => Some("Removes the process from the system"),
        crate::resource::IDM_PROC_ENDTREE => {
            Some("Removes the process and any child processes from the system")
        }
        crate::resource::IDM_HELP => Some("Displays Task Manager help topics"),
        crate::resource::IDM_PROCCOLS => {
            Some("Select which columns will be visible on the Process page")
        }
        crate::resource::IDM_REFRESH => {
            Some("Force Task Manager to update now, regardless of Update Speed setting")
        }
        crate::resource::IDM_AFFINITY => {
            Some("Controls which processors the process will be allowed to run on")
        }
        crate::resource::IDM_KERNELTIMES => {
            Some("Displays kernel timings on the CPU graphs in red.")
        }
        crate::resource::IDM_TASK_MINIMIZE => Some("Minimizes the windows"),
        crate::resource::IDM_TASK_MAXIMIZE => Some("Maximizes the windows"),
        crate::resource::IDM_TASK_CASCADE => Some("Cascades the windows diagonally on the desktop"),
        crate::resource::IDM_TASK_TILEHORZ => Some("Tiles the windows horizontally on the desktop"),
        crate::resource::IDM_TASK_TILEVERT => Some("Tiles the windows vertically on the desktop"),
        crate::resource::IDM_TASK_BRINGTOFRONT => {
            Some("Brings the window front, but does not switch to it")
        }
        _ => None,
    }
}

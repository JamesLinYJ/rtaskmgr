use std::sync::OnceLock;

// 本地化入口模块。
// 其它模块只通过这里按资源 ID 或文本键取字符串，不必关心当前语言表
// 实际来自哪个语言文件。

use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

#[path = "localization_terms.rs"]
mod localization_terms;
#[path = "localization_ui.rs"]
mod localization_ui;
mod de;
mod en_us;
mod es;
mod fr;
mod pt;
mod ru;
mod text_key;
mod zh_cn;
mod zh_tw;

pub use localization_terms::{
    adapter_state, network_column_titles, network_graph_labels, session_state, user_column_titles,
    user_session_column_title,
};
pub use localization_ui::{localize_dialog, localize_menu};
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
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

pub fn text(key: TextKey) -> &'static str {
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

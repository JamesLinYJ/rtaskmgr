use std::mem::{size_of, zeroed};
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{ERROR_SUCCESS, RECT};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_BINARY, REG_OPTION_NON_VOLATILE,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SystemParametersInfoW, SM_CXMAXIMIZED, SM_CYMAXIMIZED, SPI_GETSCREENREADER,
};

use crate::resource::{NUM_COLUMN, NUM_PAGES};
use crate::winutil::to_wide_null;

const TASKMAN_KEY: &str = "Software\\Microsoft\\Windows NT\\CurrentVersion\\TaskManager";
const OPTIONS_KEY: &str = "Preferences";

const FLAG_MINIMIZE_ON_USE: u32 = 1 << 0;
const FLAG_CONFIRMATIONS: u32 = 1 << 1;
const FLAG_ALWAYS_ON_TOP: u32 = 1 << 2;
const FLAG_KERNEL_TIMES: u32 = 1 << 3;
const FLAG_NO_TITLE: u32 = 1 << 4;
const FLAG_HIDE_WHEN_MIN: u32 = 1 << 5;
const FLAG_SHOW_16BIT: u32 = 1 << 6;

#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    LargeIcon = 0,
    SmallIcon = 1,
    Details = 2,
}

#[repr(i32)]
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CpuHistoryMode {
    Sum = 0,
    Panes = 1,
}

#[repr(i32)]
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UpdateSpeed {
    High = 0,
    Normal = 1,
    Low = 2,
    Paused = 3,
}

#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColumnId {
    ImageName = 0,
    Pid = 1,
    Username = 2,
    SessionId = 3,
    Cpu = 4,
    CpuTime = 5,
    MemUsage = 6,
    MemUsageDiff = 7,
    PageFaults = 8,
    PageFaultsDiff = 9,
    CommitCharge = 10,
    PagedPool = 11,
    NonPagedPool = 12,
    BasePriority = 13,
    HandleCount = 14,
    ThreadCount = 15,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Options {
    pub cb_size: u32,
    pub timer_interval: u32,
    pub view_mode: i32,
    pub cpu_history_mode: i32,
    pub update_speed: i32,
    pub window_rect: RECT,
    pub current_page: i32,
    pub active_process_columns: [i32; NUM_COLUMN + 1],
    pub column_widths: [i32; NUM_COLUMN + 1],
    flags: u32,
    pub unused: i32,
    pub unused2: i32,
}

impl Default for Options {
    fn default() -> Self {
        let mut options = Self {
            cb_size: size_of::<Self>() as u32,
            timer_interval: 1000,
            view_mode: ViewMode::Details as i32,
            cpu_history_mode: CpuHistoryMode::Panes as i32,
            update_speed: UpdateSpeed::Normal as i32,
            window_rect: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            current_page: -1,
            active_process_columns: [-1; NUM_COLUMN + 1],
            column_widths: [-1; NUM_COLUMN + 1],
            flags: 0,
            unused: 0,
            unused2: 0,
        };

        options.set_minimize_on_use(true);
        options.set_confirmations(true);
        options.set_always_on_top(true);
        options.set_show_16bit(true);

        options.active_process_columns[0] = ColumnId::ImageName as i32;
        options.active_process_columns[1] = ColumnId::Username as i32;
        options.active_process_columns[2] = ColumnId::SessionId as i32;
        options.active_process_columns[3] = ColumnId::Cpu as i32;
        options.active_process_columns[4] = ColumnId::MemUsage as i32;

        options
    }
}

impl Options {
    pub fn set_default_values(&mut self, min_width: i32, min_height: i32) {
        *self = Self::default();

        if screen_reader_enabled() {
            self.timer_interval = 0;
        }

        let screen_width = unsafe { GetSystemMetrics(SM_CXMAXIMIZED) };
        let screen_height = unsafe { GetSystemMetrics(SM_CYMAXIMIZED) };

        self.window_rect.left = (screen_width - min_width) / 2;
        self.window_rect.top = (screen_height - min_height) / 2;
        self.window_rect.right = self.window_rect.left + min_width;
        self.window_rect.bottom = self.window_rect.top + min_height;
    }

    pub fn load(&mut self, min_width: i32, min_height: i32) -> bool {
        if modifiers_force_defaults() {
            self.set_default_values(min_width, min_height);
            return false;
        }

        unsafe {
            let key_name = to_wide_null(TASKMAN_KEY);
            let value_name = to_wide_null(OPTIONS_KEY);
            let mut key: HKEY = null_mut();
            if RegOpenKeyExW(HKEY_CURRENT_USER, key_name.as_ptr(), 0, KEY_READ, &mut key) != ERROR_SUCCESS {
                self.set_default_values(min_width, min_height);
                return false;
            }

            let mut loaded = zeroed::<Options>();
            let mut value_type = 0u32;
            let mut value_size = size_of::<Options>() as u32;
            let status = RegQueryValueExW(
                key,
                value_name.as_ptr(),
                null_mut(),
                &mut value_type,
                &mut loaded as *mut Options as *mut u8,
                &mut value_size,
            );
            RegCloseKey(key);

            if status != ERROR_SUCCESS
                || value_type != REG_BINARY
                || value_size != size_of::<Options>() as u32
                || !loaded.is_valid()
            {
                self.set_default_values(min_width, min_height);
                return false;
            }

            *self = loaded;
            true
        }
    }

    pub fn save(&self) -> Result<(), u32> {
        unsafe {
            let key_name = to_wide_null(TASKMAN_KEY);
            let value_name = to_wide_null(OPTIONS_KEY);
            let mut key: HKEY = null_mut();
            let mut disposition = 0u32;

            let create_status = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                key_name.as_ptr(),
                0,
                null_mut(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                null_mut(),
                &mut key,
                &mut disposition,
            );
            if create_status != ERROR_SUCCESS {
                return Err(create_status);
            }

            let set_status = RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_BINARY,
                self as *const Options as *const u8,
                size_of::<Options>() as u32,
            );
            RegCloseKey(key);

            if set_status == ERROR_SUCCESS {
                Ok(())
            } else {
                Err(set_status)
            }
        }
    }

    pub fn minimize_on_use(&self) -> bool {
        self.flags & FLAG_MINIMIZE_ON_USE != 0
    }

    pub fn set_minimize_on_use(&mut self, value: bool) {
        self.set_flag(FLAG_MINIMIZE_ON_USE, value);
    }

    pub fn confirmations(&self) -> bool {
        self.flags & FLAG_CONFIRMATIONS != 0
    }

    pub fn set_confirmations(&mut self, value: bool) {
        self.set_flag(FLAG_CONFIRMATIONS, value);
    }

    pub fn always_on_top(&self) -> bool {
        self.flags & FLAG_ALWAYS_ON_TOP != 0
    }

    pub fn set_always_on_top(&mut self, value: bool) {
        self.set_flag(FLAG_ALWAYS_ON_TOP, value);
    }

    pub fn kernel_times(&self) -> bool {
        self.flags & FLAG_KERNEL_TIMES != 0
    }

    pub fn set_kernel_times(&mut self, value: bool) {
        self.set_flag(FLAG_KERNEL_TIMES, value);
    }

    pub fn no_title(&self) -> bool {
        self.flags & FLAG_NO_TITLE != 0
    }

    pub fn set_no_title(&mut self, value: bool) {
        self.set_flag(FLAG_NO_TITLE, value);
    }

    pub fn hide_when_minimized(&self) -> bool {
        self.flags & FLAG_HIDE_WHEN_MIN != 0
    }

    pub fn set_hide_when_minimized(&mut self, value: bool) {
        self.set_flag(FLAG_HIDE_WHEN_MIN, value);
    }

    pub fn show_16bit(&self) -> bool {
        self.flags & FLAG_SHOW_16BIT != 0
    }

    pub fn set_show_16bit(&mut self, value: bool) {
        self.set_flag(FLAG_SHOW_16BIT, value);
    }

    fn is_valid(&self) -> bool {
        let max_width = unsafe { GetSystemMetrics(SM_CXMAXIMIZED) };
        let max_height = unsafe { GetSystemMetrics(SM_CYMAXIMIZED) };

        self.window_rect.left <= max_width
            && self.window_rect.top <= max_height
            && self.window_rect.right >= 0
            && self.window_rect.bottom >= 0
            && self.current_page < NUM_PAGES as i32
    }

    fn set_flag(&mut self, mask: u32, value: bool) {
        if value {
            self.flags |= mask;
        } else {
            self.flags &= !mask;
        }
    }
}

fn modifiers_force_defaults() -> bool {
    unsafe {
        GetKeyState(VK_SHIFT as i32) < 0
            && GetKeyState(VK_MENU as i32) < 0
            && GetKeyState(VK_CONTROL as i32) < 0
    }
}

fn screen_reader_enabled() -> bool {
    let mut enabled = 0i32;
    unsafe {
        SystemParametersInfoW(
            SPI_GETSCREENREADER,
            0,
            &mut enabled as *mut i32 as *mut _,
            0,
        ) != 0
            && enabled != 0
    }
}

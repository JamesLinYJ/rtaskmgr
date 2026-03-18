use std::ffi::c_void;

// 性能页实现。
// 该模块负责采样系统级 CPU/内存指标，并绘制经典任务管理器里的折线图、
// 数值面板和状态快照。
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HINSTANCE, HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen, CreateSolidBrush, DeleteDC,
    DeleteObject, DrawTextW, FillRect, GetCurrentObject, GetDC, GetObjectW, GetStockObject,
    InvalidateRect, LineTo, LoadBitmapW, LOGFONTW, MapWindowPoints, MoveToEx, Rectangle,
    ReleaseDC, SelectObject, SetBkMode, SetTextColor, UpdateWindow, BLACK_BRUSH, DT_BOTTOM,
    DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, HBITMAP, HBRUSH, HDC, HGDIOBJ, OBJ_FONT,
    PS_SOLID, SRCCOPY, TRANSPARENT,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetPerformanceInfo, PERFORMANCE_INFORMATION};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BeginDeferWindowPos, DeferWindowPos, EndDeferWindowPos, GetClientRect, GetDialogBaseUnits,
    GetDlgItem, GetWindowRect, HDWP, IsIconic, SetDlgItemTextW, ShowWindow, SW_HIDE, SW_SHOW,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
};

use crate::options::{CpuHistoryMode, Options};
use crate::resource::{
    IDC_AVAIL_PHYSICAL, IDC_COMMIT_LIMIT, IDC_COMMIT_PEAK, IDC_COMMIT_TOTAL, IDC_CPUGRAPH,
    IDC_CPUMETER, IDC_CPUUSAGEFRAME, IDC_FILE_CACHE, IDC_KERNEL_NONPAGED, IDC_KERNEL_PAGED,
    IDC_KERNEL_TOTAL,
    IDC_LAST_CPUGRAPH, IDC_MEMBARFRAME, IDC_MEMFRAME, IDC_MEMGRAPH, IDC_MEMMETER, IDC_STATIC1,
    IDC_STATIC2, IDC_STATIC3, IDC_STATIC4, IDC_STATIC5, IDC_STATIC6, IDC_STATIC8, IDC_STATIC9,
    IDC_STATIC10, IDC_STATIC11, IDC_STATIC12, IDC_STATIC13, IDC_STATIC14, IDC_STATIC15, IDC_STATIC16,
    IDC_STATIC17, IDC_TOTAL_HANDLES, IDC_TOTAL_PHYSICAL, IDC_TOTAL_PROCESSES, IDC_TOTAL_THREADS,
    LED_STRIP_LIT, LED_STRIP_LIT_RED, LED_STRIP_UNLIT, STATIC_CPU_GRAPH_COUNT,
};
use crate::winutil::{hiword, loword, make_int_resource, to_wide_null};

#[link(name = "shlwapi")]
unsafe extern "system" {
    fn StrFormatByteSizeW(qw: i64, pszbuf: *mut u16, cchbuf: u32) -> *mut u16;
}

const HIST_SIZE: usize = 2000;
const GRAPH_GRID: i32 = 12;
const STRIP_HEIGHT: i32 = 75;
const STRIP_WIDTH: i32 = 33;
const DEFSPACING_BASE: i32 = 3;
const INNERSPACING_BASE: i32 = 2;
const TOPSPACING_BASE: i32 = 10;
const DLG_SCALE_X: i32 = 4;
const DLG_SCALE_Y: i32 = 8;
const CPU_USAGE_FRAME_ID: i32 = IDC_CPUUSAGEFRAME;

const PERF_TEXT_CONTROLS: [i32; 28] = [
    IDC_STATIC1,
    IDC_STATIC2,
    IDC_STATIC3,
    IDC_STATIC4,
    IDC_STATIC5,
    IDC_STATIC6,
    IDC_STATIC8,
    IDC_STATIC9,
    IDC_STATIC10,
    IDC_STATIC11,
    IDC_STATIC12,
    IDC_STATIC13,
    IDC_STATIC14,
    IDC_STATIC15,
    IDC_STATIC16,
    IDC_STATIC17,
    IDC_TOTAL_PHYSICAL,
    IDC_AVAIL_PHYSICAL,
    IDC_FILE_CACHE,
    IDC_COMMIT_TOTAL,
    IDC_COMMIT_LIMIT,
    IDC_COMMIT_PEAK,
    IDC_KERNEL_TOTAL,
    IDC_KERNEL_PAGED,
    IDC_KERNEL_NONPAGED,
    IDC_TOTAL_HANDLES,
    IDC_TOTAL_THREADS,
    IDC_TOTAL_PROCESSES,
];

const PERF_LAYOUT_CONTROLS: [i32; 28] = PERF_TEXT_CONTROLS;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SystemProcessorPerformanceInformation {
    idle_time: i64,
    kernel_time: i64,
    user_time: i64,
    dpc_time: i64,
    interrupt_time: i64,
    interrupt_count: u32,
}

#[repr(i32)]
enum SystemInformationClass {
    ProcessorPerformanceInformation = 8,
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQuerySystemInformation(
        system_information_class: i32,
        system_information: *mut c_void,
        system_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

#[derive(Clone, Copy, Default)]
pub struct PerformanceSnapshot {
    pub cpu_usage: u8,
    pub mem_usage_kb: u32,
    pub mem_limit_kb: u32,
    pub process_count: u32,
}

#[derive(Default)]
pub struct PerformancePageState {
    // 页面级缓存包含采样结果、图表历史和绘制时会复用的 GDI 资源句柄。
    hinstance: HINSTANCE,
    processor_count: usize,
    cpu_usage: u8,
    kernel_usage: u8,
    physical_mem_usage_kb: u32,
    physical_mem_limit_kb: u32,
    commit_total_kb: u32,
    commit_limit_kb: u32,
    commit_peak_kb: u32,
    total_physical_kb: u32,
    avail_physical_kb: u32,
    file_cache_kb: u32,
    kernel_total_kb: u32,
    kernel_paged_kb: u32,
    kernel_nonpaged_kb: u32,
    handle_count: u32,
    thread_count: u32,
    process_count: u32,
    cpu_history_mode: i32,
    show_kernel_times: bool,
    no_title: bool,
    scroll_offset: i32,
    previous_idle_times: Vec<i64>,
    previous_total_times: Vec<i64>,
    previous_kernel_times: Vec<i64>,
    cpu_history: Vec<Vec<u8>>,
    kernel_history: Vec<Vec<u8>>,
    mem_history: Vec<u8>,
    strip_lit_bitmap: HBITMAP,
    strip_lit_red_bitmap: HBITMAP,
    strip_unlit_bitmap: HBITMAP,
    graph_dc: HDC,
    graph_bitmap: HBITMAP,
    graph_bitmap_old: HGDIOBJ,
    graph_bitmap_width: i32,
    graph_bitmap_height: i32,
}

impl PerformancePageState {
    pub fn new() -> Self {
        Self::default()
    }

    pub unsafe fn initialize(&mut self, hinstance: HINSTANCE, processor_count: usize) {
        // 性能页启动时先准备采样缓冲和仪表位图；
        // 真正依赖窗口尺寸的离屏表面会在布局完成后再创建。
        self.hinstance = hinstance;
        self.ensure_history_capacity(processor_count.max(1));
        self.load_meter_bitmaps();
    }

    pub unsafe fn apply_options(&mut self, hwnd_page: HWND, options: &Options, processor_count: usize) {
        // 配置变化会同时影响图表数量、是否叠加内核时间，以及文字区是否折叠。
        self.ensure_history_capacity(processor_count.max(1));
        self.cpu_history_mode = options.cpu_history_mode;
        self.show_kernel_times = options.kernel_times();
        self.no_title = options.no_title();

        let pane_count = self.visible_cpu_graph_count();

        for index in 0..self.cpu_graph_slot_count() {
            let control = self.cpu_graph_hwnd(hwnd_page, index);
            if !control.is_null() {
                ShowWindow(control, if index < pane_count { SW_SHOW } else { SW_HIDE });
            }
        }

        let detail_state = if self.no_title { SW_HIDE } else { SW_SHOW };
        for control_id in PERF_TEXT_CONTROLS {
            let control = GetDlgItem(hwnd_page, control_id);
            if !control.is_null() {
                ShowWindow(control, detail_state);
            }
        }

        for control_id in [IDC_MEMGRAPH, IDC_MEMFRAME, IDC_MEMBARFRAME, IDC_MEMMETER] {
            let control = GetDlgItem(hwnd_page, control_id);
            if !control.is_null() {
                ShowWindow(control, detail_state);
            }
        }

        InvalidateRect(hwnd_page, null(), 0);
    }

    pub unsafe fn timer_event(&mut self, hwnd_page: HWND, main_hwnd: HWND) {
        // 定时器事件先刷新底层采样，再推动图表滚动与数值文本更新。
        self.refresh_measurements(hwnd_page);
        self.scroll_offset = (self.scroll_offset + 2) % GRAPH_GRID;

        if IsIconic(main_hwnd) == 0 {
            for control_id in [IDC_CPUMETER, IDC_MEMMETER, IDC_MEMGRAPH] {
                let control = GetDlgItem(hwnd_page, control_id);
                if !control.is_null() {
                    InvalidateRect(control, null(), 0);
                    UpdateWindow(control);
                }
            }

            let pane_count = if self.cpu_history_mode == CpuHistoryMode::Panes as i32 {
                self.processor_count.max(1)
            } else {
                1
            };
            for pane_index in 0..pane_count {
                let control = self.cpu_graph_hwnd(hwnd_page, pane_index);
                if !control.is_null() {
                    InvalidateRect(control, null(), 0);
                    UpdateWindow(control);
                }
            }
        }
    }

    pub fn snapshot(&self) -> PerformanceSnapshot {
        PerformanceSnapshot {
            cpu_usage: self.cpu_usage,
            mem_usage_kb: self.physical_mem_usage_kb,
            mem_limit_kb: self.physical_mem_limit_kb,
            process_count: self.process_count,
        }
    }

    pub fn no_title(&self) -> bool {
        self.no_title
    }

    pub fn is_graph_control(&self, control_id: i32) -> bool {
        matches!(control_id, IDC_MEMGRAPH | IDC_MEMMETER | IDC_CPUMETER)
            || self.cpu_graph_pane_index(control_id).is_some()
    }

    pub fn cpu_graph_pane_index(&self, control_id: i32) -> Option<usize> {
        if (IDC_CPUGRAPH..=IDC_LAST_CPUGRAPH).contains(&control_id) {
            let pane_index = (control_id - IDC_CPUGRAPH) as usize;
            if pane_index < self.cpu_graph_slot_count() {
                Some(pane_index)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub unsafe fn draw_cpu_graph(&self, hdc: HDC, rect: RECT, pane_index: usize) {
        // CPU 图优先绘制到离屏 DC，再一次性拷回目标 DC，
        // 这样网格线和曲线更新时不会在前台逐步闪出来。
        if pane_index >= self.cpu_history.len() {
            return;
        }

        let width = (rect.right - rect.left).max(1);
        let height = (rect.bottom - rect.top).max(1);
        let use_backbuffer = !self.graph_dc.is_null()
            && self.graph_bitmap_width >= width
            && self.graph_bitmap_height >= height;
        let target_hdc = if use_backbuffer { self.graph_dc } else { hdc };
        let target_rect = if use_backbuffer {
            RECT {
                left: 0,
                top: 0,
                right: self.graph_bitmap_width,
                bottom: height,
            }
        } else {
            rect
        };

        fill_black(target_hdc, &target_rect);
        draw_grid_width(target_hdc, &target_rect, width, self.scroll_offset);

        let graph_height = (target_rect.bottom - target_rect.top - 1).max(1);
        let scale = ((width - 1) / HIST_SIZE as i32).max(0);
        let scale = if scale == 0 { 2 } else { scale } as usize;

        if self.show_kernel_times {
            if self.cpu_history_mode == CpuHistoryMode::Panes as i32 {
                draw_history_series(
                    target_hdc,
                    &target_rect,
                    graph_height,
                    width,
                    scale,
                    &self.kernel_history[pane_index],
                    rgb(255, 0, 0),
                    false,
                );
            } else {
                let averaged_kernel = average_history(&self.kernel_history);
                draw_history_series(
                    target_hdc,
                    &target_rect,
                    graph_height,
                    width,
                    scale,
                    &averaged_kernel,
                    rgb(255, 0, 0),
                    false,
                );
            }
        }

        if self.cpu_history_mode == CpuHistoryMode::Panes as i32 {
            draw_history_series(
                target_hdc,
                &target_rect,
                graph_height,
                width,
                scale,
                &self.cpu_history[pane_index],
                rgb(0, 255, 0),
                false,
            );
        } else {
            let averaged_cpu = average_history(&self.cpu_history);
            draw_history_series(
                target_hdc,
                &target_rect,
                graph_height,
                width,
                scale,
                &averaged_cpu,
                rgb(0, 255, 0),
                false,
            );
        }

        if use_backbuffer {
            let x_diff = (self.graph_bitmap_width - width).max(0);
            BitBlt(
                hdc,
                rect.left,
                rect.top,
                width,
                height,
                self.graph_dc,
                x_diff,
                0,
                SRCCOPY,
            );
        }
    }

    pub unsafe fn draw_mem_graph(&self, hdc: HDC, rect: RECT) {
        // 内存历史图复用 CPU 图的绘制策略，只是数据源和颜色不同。
        let width = (rect.right - rect.left).max(1);
        let height = (rect.bottom - rect.top).max(1);
        let use_backbuffer = !self.graph_dc.is_null()
            && self.graph_bitmap_width >= width
            && self.graph_bitmap_height >= height;
        let target_hdc = if use_backbuffer { self.graph_dc } else { hdc };
        let target_rect = if use_backbuffer {
            RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            }
        } else {
            rect
        };

        fill_black(target_hdc, &target_rect);
        draw_grid_width(target_hdc, &target_rect, width, self.scroll_offset);
        let scale = ((width - 1) / HIST_SIZE as i32).max(0);
        let scale = if scale == 0 { 2 } else { scale } as usize;
        draw_history_series(
            target_hdc,
            &target_rect,
            (target_rect.bottom - target_rect.top - 1).max(1),
            width,
            scale,
            &self.mem_history,
            rgb(255, 255, 0),
            true,
        );

        if use_backbuffer {
            BitBlt(
                hdc,
                rect.left,
                rect.top,
                width,
                height,
                self.graph_dc,
                0,
                0,
                SRCCOPY,
            );
        }
    }

    pub unsafe fn draw_cpu_meter(&self, hdc: HDC, rect: RECT) {
        if self.draw_strip_meter(
            hdc,
            rect,
            &format!("{} %", self.cpu_usage),
            self.cpu_usage,
            if self.show_kernel_times {
                self.kernel_usage.min(self.cpu_usage)
            } else {
                0
            },
        ) {
            return;
        }

        draw_meter(hdc, rect, &format!("{} %", self.cpu_usage), self.cpu_usage, if self.show_kernel_times {
            self.kernel_usage.min(self.cpu_usage)
        } else {
            0
        }, rgb(0, 255, 0), rgb(255, 0, 0));
    }

    pub unsafe fn draw_mem_meter(&self, hdc: HDC, rect: RECT) {
        let mem_percent = if self.physical_mem_limit_kb == 0 {
            0
        } else {
            ((self.physical_mem_usage_kb.saturating_mul(100)) / self.physical_mem_limit_kb).min(100)
                as u8
        };
        let mem_usage_text = format_mem_meter_text(self.physical_mem_usage_kb);
        if self.draw_strip_meter(
            hdc,
            rect,
            &mem_usage_text,
            mem_percent,
            0,
        ) {
            return;
        }

        draw_meter(
            hdc,
            rect,
            &mem_usage_text,
            mem_percent,
            0,
            rgb(255, 255, 0),
            rgb(255, 255, 0),
        );
    }

    pub unsafe fn size_page(&mut self, hwnd_page: HWND, main_hwnd: HWND) {
        // 布局逻辑尽量贴近经典 Task Manager：
        // 先算整体可用高度，再分配图表、仪表和底部统计区的位置。
        if hwnd_page.is_null() {
            return;
        }


        let mut parent_rect = zeroed::<RECT>();
        if self.no_title {
            // C++ uses GetClientRect(g_hMainWnd) directly — no mapping
            GetClientRect(main_hwnd, &mut parent_rect);
        } else {
            GetClientRect(hwnd_page, &mut parent_rect);
        }

        let pane_count = self.visible_cpu_graph_count();

        let units = GetDialogBaseUnits() as usize;
        let def_spacing = (DEFSPACING_BASE * loword(units) as i32) / DLG_SCALE_X;
        let inner_spacing = (INNERSPACING_BASE * loword(units) as i32) / DLG_SCALE_X;
        let top_spacing = (TOPSPACING_BASE * hiword(units) as i32) / DLG_SCALE_Y;

        let defer_hint = (PERF_LAYOUT_CONTROLS.len() + self.cpu_graph_slot_count() + 6) as i32;
        let mut hdwp: HDWP = BeginDeferWindowPos(defer_hint);
        if hdwp.is_null() {
            return;
        }

        let master_rect = window_rect_relative_to_page(GetDlgItem(hwnd_page, IDC_STATIC5), hwnd_page);
        let dy = ((parent_rect.bottom - def_spacing * 2) - master_rect.bottom).max(-master_rect.bottom);

        for control_id in PERF_LAYOUT_CONTROLS {
            let hwnd_ctrl = GetDlgItem(hwnd_page, control_id);
            if hwnd_ctrl.is_null() {
                continue;
            }
            let rect = window_rect_relative_to_page(hwnd_ctrl, hwnd_page);
            hdwp = DeferWindowPos(
                hdwp,
                hwnd_ctrl,
                null_mut(),
                rect.left,
                rect.top + dy,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }

        let top_frame = window_rect_relative_to_page(GetDlgItem(hwnd_page, IDC_STATIC13), hwnd_page);
        let y_top = top_frame.top + dy;
        let y_hist = if self.no_title {
            parent_rect.bottom - parent_rect.top - def_spacing * 2
        } else {
            (y_top - def_spacing * 3) / 2
        };

        let cpu_history_frame =
            window_rect_relative_to_page(GetDlgItem(hwnd_page, crate::resource::IDC_CPUFRAME), hwnd_page);
        hdwp = defer_resize(
            hdwp,
            GetDlgItem(hwnd_page, crate::resource::IDC_CPUFRAME),
            (parent_rect.right - cpu_history_frame.left) - def_spacing * 2,
            y_hist,
        );

        let cpu_usage_frame =
            window_rect_relative_to_page(GetDlgItem(hwnd_page, CPU_USAGE_FRAME_ID), hwnd_page);
        hdwp = defer_resize(
            hdwp,
            GetDlgItem(hwnd_page, CPU_USAGE_FRAME_ID),
            cpu_usage_frame.right - cpu_usage_frame.left,
            y_hist,
        );

        let cpu_meter = window_rect_relative_to_page(GetDlgItem(hwnd_page, IDC_CPUMETER), hwnd_page);
        hdwp = DeferWindowPos(
            hdwp,
            GetDlgItem(hwnd_page, IDC_CPUMETER),
            null_mut(),
            cpu_usage_frame.left + inner_spacing * 2,
            cpu_usage_frame.top + top_spacing,
            cpu_meter.right - cpu_meter.left,
            y_hist - top_spacing - inner_spacing * 2,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        let mem_bar_frame = window_rect_relative_to_page(GetDlgItem(hwnd_page, IDC_MEMBARFRAME), hwnd_page);
        hdwp = DeferWindowPos(
            hdwp,
            GetDlgItem(hwnd_page, IDC_MEMBARFRAME),
            null_mut(),
            mem_bar_frame.left,
            y_hist + def_spacing * 2,
            mem_bar_frame.right - mem_bar_frame.left,
            y_hist,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        let mem_meter = window_rect_relative_to_page(GetDlgItem(hwnd_page, IDC_MEMMETER), hwnd_page);
        hdwp = DeferWindowPos(
            hdwp,
            GetDlgItem(hwnd_page, IDC_MEMMETER),
            null_mut(),
            mem_meter.left,
            y_hist + def_spacing * 2 + top_spacing,
            mem_meter.right - mem_meter.left,
            y_hist - inner_spacing * 2 - top_spacing,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        let mem_frame = window_rect_relative_to_page(GetDlgItem(hwnd_page, IDC_MEMFRAME), hwnd_page);
        hdwp = DeferWindowPos(
            hdwp,
            GetDlgItem(hwnd_page, IDC_MEMFRAME),
            null_mut(),
            mem_frame.left,
            y_hist + def_spacing * 2,
            (parent_rect.right - mem_frame.left) - def_spacing * 2,
            y_hist,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        let mut pane_width = (parent_rect.right - parent_rect.left)
            - (cpu_history_frame.left - parent_rect.left)
            - def_spacing * 2
            - inner_spacing * 3;
        hdwp = DeferWindowPos(
            hdwp,
            GetDlgItem(hwnd_page, IDC_MEMGRAPH),
            null_mut(),
            cpu_history_frame.left + inner_spacing * 2,
            y_hist + def_spacing * 2 + top_spacing,
            pane_width - inner_spacing,
            y_hist - inner_spacing * 2 - top_spacing,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        pane_width -= pane_count as i32 * inner_spacing;
        pane_width = (pane_width / pane_count as i32).max(0);
        for pane_index in 0..pane_count {
            let left = cpu_history_frame.left
                + inner_spacing * (pane_index as i32 + 2)
                + pane_width * pane_index as i32;
            let cpu_graph = self.cpu_graph_hwnd(hwnd_page, pane_index);
            if cpu_graph.is_null() {
                continue;
            }
            hdwp = DeferWindowPos(
                hdwp,
                cpu_graph,
                null_mut(),
                left,
                cpu_history_frame.top + top_spacing,
                pane_width,
                y_hist - inner_spacing * 2 - top_spacing,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }

        EndDeferWindowPos(hdwp);
        self.recreate_graph_surface(hwnd_page);
    }

    pub unsafe fn destroy(&mut self) {
        self.destroy_graph_surface();
        if !self.strip_lit_bitmap.is_null() {
            DeleteObject(self.strip_lit_bitmap as _);
            self.strip_lit_bitmap = null_mut();
        }
        if !self.strip_lit_red_bitmap.is_null() {
            DeleteObject(self.strip_lit_red_bitmap as _);
            self.strip_lit_red_bitmap = null_mut();
        }
        if !self.strip_unlit_bitmap.is_null() {
            DeleteObject(self.strip_unlit_bitmap as _);
            self.strip_unlit_bitmap = null_mut();
        }
    }

    fn ensure_history_capacity(&mut self, processor_count: usize) {
        // 核心数变化时，所有按 CPU 维度分片的历史数组都需要一起重建，
        // 否则“每核图”和“汇总图”会看到不一致的采样长度。
        if self.processor_count == processor_count
            && self.cpu_history.len() == processor_count
            && self.mem_history.len() == HIST_SIZE
        {
            return;
        }

        self.processor_count = processor_count;
        self.previous_idle_times.resize(processor_count, 0);
        self.previous_total_times.resize(processor_count, 0);
        self.previous_kernel_times.resize(processor_count, 0);
        self.cpu_history = vec![vec![0; HIST_SIZE]; processor_count];
        self.kernel_history = vec![vec![0; HIST_SIZE]; processor_count];
        self.mem_history = vec![0; HIST_SIZE];
    }

    unsafe fn refresh_measurements(&mut self, hwnd_page: HWND) {
        // 这里集中采集所有性能相关数据，确保一次刷新内各图表看到的是同一时刻的快照。
        if self.processor_count == 0 {
            self.ensure_history_capacity(1);
        }

        self.refresh_cpu_histories();
        self.refresh_system_info(hwnd_page);
    }

    unsafe fn refresh_cpu_histories(&mut self) {
        // 内核返回的是累积 CPU 时间，所以这里必须与上一轮做差，
        // 再换算成本轮使用率和内核时间占比。
        let mut processor_info = vec![SystemProcessorPerformanceInformation::default(); self.processor_count];
        let status = NtQuerySystemInformation(
            SystemInformationClass::ProcessorPerformanceInformation as i32,
            processor_info.as_mut_ptr() as *mut c_void,
            (processor_info.len() * size_of::<SystemProcessorPerformanceInformation>()) as u32,
            null_mut(),
        );
        if status < 0 {
            return;
        }

        let mut sum_idle = 0i64;
        let mut sum_total = 0i64;
        let mut sum_kernel = 0i64;

        for (index, entry) in processor_info.iter().enumerate() {
            let idle_time = entry.idle_time;
            let kernel_time = entry.kernel_time.saturating_sub(entry.idle_time);
            let total_time = entry.kernel_time.saturating_add(entry.user_time);

            let delta_idle = idle_time.saturating_sub(self.previous_idle_times[index]);
            let delta_kernel = kernel_time.saturating_sub(self.previous_kernel_times[index]);
            let delta_total = total_time.saturating_sub(self.previous_total_times[index]);

            sum_idle = sum_idle.saturating_add(delta_idle);
            sum_kernel = sum_kernel.saturating_add(delta_kernel);
            sum_total = sum_total.saturating_add(delta_total);

            let cpu_percent = if delta_total > 0 {
                (100 - ((delta_idle * 100) / delta_total)).clamp(0, 100) as u8
            } else {
                0
            };
            let kernel_percent = if delta_total > 0 {
                ((delta_kernel * 100) / delta_total).clamp(0, 100) as u8
            } else {
                0
            };

            push_history(&mut self.cpu_history[index], cpu_percent);
            push_history(&mut self.kernel_history[index], kernel_percent);

            self.previous_idle_times[index] = idle_time;
            self.previous_total_times[index] = total_time;
            self.previous_kernel_times[index] = kernel_time;
        }

        self.cpu_usage = if sum_total > 0 {
            (100 - ((sum_idle * 100) / sum_total)).clamp(0, 100) as u8
        } else {
            0
        };
        self.kernel_usage = if sum_total > 0 {
            ((sum_kernel * 100) / sum_total).clamp(0, 100) as u8
        } else {
            0
        };
    }

    unsafe fn refresh_system_info(&mut self, hwnd_page: HWND) {
        // 系统级内存、Commit、句柄、线程、进程总数都来源于同一份快照，
        // 统一在这里采样可以保证页面上的数字属于同一个刷新时刻。
        let mut perf = zeroed::<PERFORMANCE_INFORMATION>();
        perf.cb = size_of::<PERFORMANCE_INFORMATION>() as u32;
        if K32GetPerformanceInfo(&mut perf, perf.cb) == 0 {
            return;
        }

        let page_kb = (perf.PageSize / 1024).max(1);
        let pages_to_kb = |page_count: usize| -> u32 {
            page_count
                .saturating_mul(page_kb)
                .min(u32::MAX as usize) as u32
        };

        self.total_physical_kb = pages_to_kb(perf.PhysicalTotal);
        self.avail_physical_kb = pages_to_kb(perf.PhysicalAvailable);
        self.file_cache_kb = pages_to_kb(perf.SystemCache);
        self.physical_mem_limit_kb = self.total_physical_kb;
        self.physical_mem_usage_kb = self
            .total_physical_kb
            .saturating_sub(self.avail_physical_kb);
        self.commit_total_kb = pages_to_kb(perf.CommitTotal);
        self.commit_limit_kb = pages_to_kb(perf.CommitLimit);
        self.commit_peak_kb = pages_to_kb(perf.CommitPeak);
        self.kernel_total_kb = pages_to_kb(perf.KernelTotal);
        self.kernel_paged_kb = pages_to_kb(perf.KernelPaged);
        self.kernel_nonpaged_kb = pages_to_kb(perf.KernelNonpaged);
        self.handle_count = perf.HandleCount;
        self.process_count = perf.ProcessCount;
        self.thread_count = perf.ThreadCount;

        let mem_percent = if self.physical_mem_limit_kb == 0 {
            0
        } else {
            ((self.physical_mem_usage_kb.saturating_mul(100)) / self.physical_mem_limit_kb).min(100)
                as u8
        };
        push_history(&mut self.mem_history, mem_percent);

        set_numeric_text(hwnd_page, IDC_TOTAL_PHYSICAL, self.total_physical_kb);
        set_numeric_text(hwnd_page, IDC_AVAIL_PHYSICAL, self.avail_physical_kb);
        set_numeric_text(hwnd_page, IDC_FILE_CACHE, self.file_cache_kb);
        set_numeric_text(hwnd_page, IDC_COMMIT_TOTAL, self.commit_total_kb);
        set_numeric_text(hwnd_page, IDC_COMMIT_LIMIT, self.commit_limit_kb);
        set_numeric_text(hwnd_page, IDC_COMMIT_PEAK, self.commit_peak_kb);
        set_numeric_text(hwnd_page, IDC_KERNEL_TOTAL, self.kernel_total_kb);
        set_numeric_text(hwnd_page, IDC_KERNEL_PAGED, self.kernel_paged_kb);
        set_numeric_text(hwnd_page, IDC_KERNEL_NONPAGED, self.kernel_nonpaged_kb);
        set_numeric_text(hwnd_page, IDC_TOTAL_HANDLES, self.handle_count);
        set_numeric_text(hwnd_page, IDC_TOTAL_THREADS, self.thread_count);
        set_numeric_text(hwnd_page, IDC_TOTAL_PROCESSES, self.process_count);
    }

    unsafe fn load_meter_bitmaps(&mut self) {
        // 条形仪表优先复用资源位图；如果资源已经加载过，就不再重复创建 GDI 对象。
        if !self.strip_lit_bitmap.is_null() {
            return;
        }

        self.strip_lit_bitmap = LoadBitmapW(self.hinstance, make_int_resource(LED_STRIP_LIT));
        self.strip_lit_red_bitmap = LoadBitmapW(self.hinstance, make_int_resource(LED_STRIP_LIT_RED));
        self.strip_unlit_bitmap = LoadBitmapW(self.hinstance, make_int_resource(LED_STRIP_UNLIT));
    }

    unsafe fn draw_strip_meter(
        &self,
        hdc: HDC,
        rect: RECT,
        label: &str,
        lit_percent: u8,
        red_percent: u8,
    ) -> bool {
        if self.strip_lit_bitmap.is_null()
            || self.strip_unlit_bitmap.is_null()
            || (red_percent != 0 && self.strip_lit_red_bitmap.is_null())
        {
            return false;
        }

        let black = GetStockObject(BLACK_BRUSH) as HBRUSH;
        let old = SelectObject(hdc, black as HGDIOBJ);
        Rectangle(hdc, rect.left, rect.top, rect.right, rect.bottom);

        let units = GetDialogBaseUnits() as usize;
        let def_spacing = (DEFSPACING_BASE * loword(units) as i32) / DLG_SCALE_X;
        let x_bar_offset = ((rect.right - rect.left) - STRIP_WIDTH) / 2;
        let bar_height = rect.bottom - rect.top - (current_font_height(hdc) + def_spacing * 3);
        if bar_height <= 0 {
            SelectObject(hdc, old);
            return true;
        }

        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextColor(hdc, rgb(0, 255, 0));
        let mut label_rect = rect;
        label_rect.bottom -= 4;
        let mut label_wide = to_wide_null(label);
        DrawTextW(
            hdc,
            label_wide.as_mut_ptr(),
            -1,
            &mut label_rect,
            DT_SINGLELINE | DT_CENTER | DT_BOTTOM,
        );

        let hdc_mem = CreateCompatibleDC(hdc);
        if hdc_mem.is_null() {
            SelectObject(hdc, old);
            return true;
        }

        let target_lit = ((lit_percent as i32 * bar_height) / 100).max(0);
        let target_red = ((red_percent as i32 * bar_height) / 100).clamp(0, target_lit);
        let unlit_pixels = ((bar_height - target_lit) / 3) * 3;
        let lit_pixels = bar_height - unlit_pixels;
        let lit_only_pixels = (lit_pixels - target_red).max(0);

        self.blit_meter_strip(
            hdc,
            hdc_mem,
            self.strip_unlit_bitmap,
            x_bar_offset,
            def_spacing,
            bar_height - lit_pixels,
        );
        if lit_only_pixels > 0 {
            self.blit_meter_strip(
                hdc,
                hdc_mem,
                self.strip_lit_bitmap,
                x_bar_offset,
                def_spacing + (bar_height - lit_pixels),
                lit_only_pixels,
            );
        }
        if target_red > 0 {
            self.blit_meter_strip(
                hdc,
                hdc_mem,
                self.strip_lit_red_bitmap,
                x_bar_offset,
                def_spacing + (bar_height - target_red),
                target_red,
            );
        }

        DeleteDC(hdc_mem);
        SelectObject(hdc, old);
        true
    }

    unsafe fn blit_meter_strip(
        &self,
        hdc: HDC,
        hdc_mem: HDC,
        bitmap: HBITMAP,
        x: i32,
        start_y: i32,
        height: i32,
    ) {
        if bitmap.is_null() || height <= 0 {
            return;
        }

        let old_bitmap = SelectObject(hdc_mem, bitmap as HGDIOBJ);
        let mut remaining = height;
        let mut offset = 0;
        while remaining > 0 {
            let chunk = remaining.min(STRIP_HEIGHT);
            BitBlt(hdc, x, start_y + offset, STRIP_WIDTH, chunk, hdc_mem, 0, 0, SRCCOPY);
            remaining -= chunk;
            offset += chunk;
        }
        SelectObject(hdc_mem, old_bitmap);
    }

    unsafe fn recreate_graph_surface(&mut self, hwnd_page: HWND) {
        // 离屏表面尺寸取当前最大的 CPU / 内存图区域，
        // 这样单份缓冲就能复用于多个图表控件。
        let mut graph_rect = zeroed::<RECT>();
        let mut mem_rect = zeroed::<RECT>();
        let cpu_graph = self.cpu_graph_hwnd(hwnd_page, 0);
        let mem_graph = GetDlgItem(hwnd_page, IDC_MEMGRAPH);

        let mut width = 0;
        let mut height = 0;
        if !cpu_graph.is_null() {
            GetClientRect(cpu_graph, &mut graph_rect);
            width = width.max(graph_rect.right - graph_rect.left);
            height = height.max(graph_rect.bottom - graph_rect.top);
        }
        if !mem_graph.is_null() {
            GetClientRect(mem_graph, &mut mem_rect);
            width = width.max(mem_rect.right - mem_rect.left);
            height = height.max(mem_rect.bottom - mem_rect.top);
        }

        if width <= 0 || height <= 0 {
            self.destroy_graph_surface();
            return;
        }

        if self.graph_bitmap_width == width
            && self.graph_bitmap_height == height
            && !self.graph_dc.is_null()
            && !self.graph_bitmap.is_null()
        {
            return;
        }

        self.destroy_graph_surface();

        let page_dc = GetDC(hwnd_page);
        if page_dc.is_null() {
            return;
        }

        let graph_dc = CreateCompatibleDC(page_dc);
        if graph_dc.is_null() {
            ReleaseDC(hwnd_page, page_dc);
            return;
        }

        let graph_bitmap = CreateCompatibleBitmap(page_dc, width, height);
        ReleaseDC(hwnd_page, page_dc);
        if graph_bitmap.is_null() {
            DeleteDC(graph_dc);
            return;
        }

        let old_bitmap = SelectObject(graph_dc, graph_bitmap as HGDIOBJ);
        self.graph_dc = graph_dc;
        self.graph_bitmap = graph_bitmap;
        self.graph_bitmap_old = old_bitmap;
        self.graph_bitmap_width = width;
        self.graph_bitmap_height = height;
    }

    unsafe fn destroy_graph_surface(&mut self) {
        if !self.graph_dc.is_null() {
            if !self.graph_bitmap_old.is_null() {
                SelectObject(self.graph_dc, self.graph_bitmap_old);
                self.graph_bitmap_old = null_mut();
            }
            DeleteDC(self.graph_dc);
            self.graph_dc = null_mut();
        }
        if !self.graph_bitmap.is_null() {
            DeleteObject(self.graph_bitmap as _);
            self.graph_bitmap = null_mut();
        }
        self.graph_bitmap_width = 0;
        self.graph_bitmap_height = 0;
    }

    fn cpu_graph_slot_count(&self) -> usize {
        STATIC_CPU_GRAPH_COUNT
    }

    fn cpu_graph_control_id(&self, pane_index: usize) -> i32 {
        IDC_CPUGRAPH + pane_index as i32
    }

    unsafe fn cpu_graph_hwnd(&self, hwnd_page: HWND, pane_index: usize) -> HWND {
        if pane_index < self.cpu_graph_slot_count() {
            GetDlgItem(hwnd_page, self.cpu_graph_control_id(pane_index))
        } else {
            null_mut()
        }
    }

    fn visible_cpu_graph_count(&self) -> usize {
        if self.cpu_history_mode == CpuHistoryMode::Panes as i32 {
            self.processor_count.max(1).min(self.cpu_graph_slot_count())
        } else {
            1
        }
    }
}

unsafe fn window_rect_relative_to_page(hwnd: HWND, page_hwnd: HWND) -> RECT {
    let mut rect = zeroed::<RECT>();
    GetWindowRect(hwnd, &mut rect);
    MapWindowPoints(null_mut(), page_hwnd, &mut rect as *mut _ as _, 2);
    rect
}

unsafe fn defer_resize(hdwp: HDWP, hwnd: HWND, width: i32, height: i32) -> HDWP {
    if hwnd.is_null() {
        return hdwp;
    }
    DeferWindowPos(
        hdwp,
        hwnd,
        null_mut(),
        0,
        0,
        width,
        height,
        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
    )
}

fn push_history(history: &mut [u8], value: u8) {
    // 历史值按“最新在前”滚动，绘图时就可以直接从右向左连接。
    if history.is_empty() {
        return;
    }
    history.copy_within(..history.len() - 1, 1);
    history[0] = value;
}

unsafe fn set_numeric_text(hwnd_page: HWND, control_id: i32, value: u32) {
    let text = to_wide_null(&value.to_string());
    SetDlgItemTextW(hwnd_page, control_id, text.as_ptr());
}

unsafe fn format_mem_meter_text(mem_usage_kb: u32) -> String {
    let mut buffer = [0u16; 32];
    if !StrFormatByteSizeW((mem_usage_kb as i64) * 1024, buffer.as_mut_ptr(), buffer.len() as u32).is_null() {
        let len = buffer.iter().position(|&ch| ch == 0).unwrap_or(buffer.len());
        return String::from_utf16_lossy(&buffer[..len]);
    }

    // Match XP intent: prefer compact byte-size text over raw kilobytes.
    let mem_usage_bytes = (mem_usage_kb as u64) * 1024;
    let gib = 1024_u64 * 1024 * 1024;
    let mib = 1024_u64 * 1024;
    if mem_usage_bytes >= gib {
        format!("{:.1} GB", mem_usage_bytes as f64 / gib as f64)
    } else if mem_usage_bytes >= mib {
        format!("{:.1} MB", mem_usage_bytes as f64 / mib as f64)
    } else {
        format!("{mem_usage_kb} KB")
    }
}

unsafe fn fill_black(hdc: HDC, rect: &RECT) {
    FillRect(hdc, rect, GetStockObject(BLACK_BRUSH) as HBRUSH);
}

unsafe fn draw_grid_width(hdc: HDC, rect: &RECT, width: i32, scroll_offset: i32) {
    // 网格会跟着 scroll_offset 横向平移，视觉上形成持续向左滚动的历史时间轴。
    let pen = CreatePen(PS_SOLID, 1, rgb(0, 128, 64));
    if pen.is_null() {
        return;
    }

    let old_pen = SelectObject(hdc, pen as _);
    let left = rect.right - width.max(0);
    let right = rect.right;
    let top = rect.top;
    let bottom = rect.bottom;

    let mut y = top + GRAPH_GRID - 1;
    while y < bottom {
        MoveToEx(hdc, left, y, null_mut());
        LineTo(hdc, right, y);
        y += GRAPH_GRID;
    }

    let mut x = right - scroll_offset;
    while x > left {
        MoveToEx(hdc, x, top, null_mut());
        LineTo(hdc, x, bottom);
        x -= GRAPH_GRID;
    }

    SelectObject(hdc, old_pen);
    DeleteObject(pen as _);
}

unsafe fn draw_history_series(
    hdc: HDC,
    rect: &RECT,
    graph_height: i32,
    width: i32,
    scale: usize,
    history: &[u8],
    color: u32,
    stop_on_zero: bool,
) {
    // 同一套折线绘制既服务 CPU 曲线，也服务内存曲线；
    // `stop_on_zero` 用来阻止内存图在历史尚未填满时画出一条贴底长线。
    if history.is_empty() {
        return;
    }

    let pen = CreatePen(PS_SOLID, 1, color);
    if pen.is_null() {
        return;
    }

    let old_pen = SelectObject(hdc, pen as _);
    MoveToEx(
        hdc,
        rect.right,
        rect.bottom - (history[0] as i32 * graph_height) / 100,
        null_mut(),
    );

    for (index, value) in history.iter().enumerate() {
        if index * scale >= width as usize {
            break;
        }
        if stop_on_zero && *value == 0 {
            break;
        }

        LineTo(
            hdc,
            rect.right - (scale * index) as i32,
            rect.bottom - (*value as i32 * graph_height) / 100,
        );
    }

    SelectObject(hdc, old_pen);
    DeleteObject(pen as _);
}

unsafe fn draw_meter(
    hdc: HDC,
    rect: RECT,
    label: &str,
    fill_percent: u8,
    red_percent: u8,
    main_color: u32,
    red_color: u32,
) {
    fill_black(hdc, &rect);

    let mut text_rect = rect;
    text_rect.top = rect.bottom - 18;

    let graph_top = rect.top + 4;
    let graph_bottom = (text_rect.top - 4).max(graph_top);
    let graph_height = (graph_bottom - graph_top).max(1);
    let bar_width = 20;
    let bar_left = rect.left + ((rect.right - rect.left - bar_width) / 2).max(0);
    let bar_right = bar_left + bar_width;

    let lit_pixels = ((graph_height * fill_percent as i32) / 100).clamp(0, graph_height);
    let red_pixels = ((graph_height * red_percent as i32) / 100).clamp(0, lit_pixels);

    if lit_pixels < graph_height {
        let unlit_rect = RECT {
            left: bar_left,
            top: graph_top,
            right: bar_right,
            bottom: graph_bottom - lit_pixels,
        };
        fill_rect_color(hdc, &unlit_rect, rgb(32, 32, 32));
    }

    if lit_pixels > red_pixels {
        let lit_rect = RECT {
            left: bar_left,
            top: graph_bottom - lit_pixels,
            right: bar_right,
            bottom: graph_bottom - red_pixels,
        };
        fill_rect_color(hdc, &lit_rect, main_color);
    }

    if red_pixels > 0 {
        let red_rect = RECT {
            left: bar_left,
            top: graph_bottom - red_pixels,
            right: bar_right,
            bottom: graph_bottom,
        };
        fill_rect_color(hdc, &red_rect, red_color);
    }

    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, rgb(0, 255, 0));
    let mut label_wide = to_wide_null(label);
    DrawTextW(
        hdc,
        label_wide.as_mut_ptr(),
        -1,
        &mut text_rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
}

unsafe fn fill_rect_color(hdc: HDC, rect: &RECT, color: u32) {
    let brush = CreateSolidBrush(color);
    if brush.is_null() {
        return;
    }
    FillRect(hdc, rect, brush);
    DeleteObject(brush as _);
}

fn average_history(history_sets: &[Vec<u8>]) -> Vec<u8> {
    // “所有 CPU 合并图”不是重新采样，而是把同一时间点上的每核数据做平均。
    let Some(first_history) = history_sets.first() else {
        return Vec::new();
    };

    let mut averaged = vec![0u8; first_history.len()];
    for index in 0..first_history.len() {
        let sum = history_sets
            .iter()
            .map(|history| history.get(index).copied().unwrap_or_default() as u32)
            .sum::<u32>();
        averaged[index] = (sum / history_sets.len() as u32).min(100) as u8;
    }

    averaged
}

unsafe fn current_font_height(hdc: HDC) -> i32 {
    let font = GetCurrentObject(hdc, OBJ_FONT as u32);
    if font.is_null() {
        return 0;
    }

    let mut font_info = zeroed::<LOGFONTW>();
    if GetObjectW(
        font,
        size_of::<LOGFONTW>() as i32,
        &mut font_info as *mut _ as *mut c_void,
    ) == 0
    {
        return 0;
    }

    font_info.lfHeight.abs()
}

const fn rgb(red: u8, green: u8, blue: u8) -> u32 {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}

use std::collections::HashMap;

// 网络页实现。
// 该模块轮询网卡统计信息，维护历史曲线，并同步底部列表与顶部图表区域。
use std::mem::zeroed;
use std::ptr::null_mut;
use std::slice;
use std::time::Instant;

use windows_sys::Win32::Foundation::{HWND, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreatePen, DeleteObject, DrawTextW, FillRect, GetStockObject, HBRUSH, HDC, InvalidateRect,
    LineTo, MapWindowPoints, MoveToEx, SelectObject, SetBkMode, SetTextColor, UpdateWindow,
    BLACK_BRUSH, DT_CALCRECT, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_TOP, PS_SOLID,
    TRANSPARENT,
};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetIfTable2, IF_TYPE_SOFTWARE_LOOPBACK, IF_TYPE_TUNNEL, MIB_IF_ROW2, MIB_IF_TABLE2,
};
use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows_sys::Win32::UI::Controls::{
    LVCFMT_LEFT, LVCFMT_RIGHT, LVCF_FMT, LVCF_SUBITEM, LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_PARAM,
    LVIF_TEXT, LVITEMW, LVM_DELETEITEM, LVM_GETITEMCOUNT, LVM_GETITEMW, LVM_INSERTITEMW,
    LVM_SETEXTENDEDLISTVIEWSTYLE, LVS_EX_FULLROWSELECT, LVS_EX_HEADERDRAGDROP, SetScrollInfo, TCM_ADJUSTRECT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BeginDeferWindowPos, CreateWindowExW, DeferWindowPos, DestroyWindow, EndDeferWindowPos,
    GetClientRect, GetDialogBaseUnits, GetDlgItem, GetScrollInfo, HDWP, HMENU, SCROLLINFO, SendMessageW,
    SetWindowTextW, ShowWindow, BS_OWNERDRAW, SB_BOTTOM, SB_CTL, SB_LINEDOWN,
    SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION, SB_THUMBTRACK, SB_TOP, SIF_ALL,
    SIF_PAGE, SIF_POS, SIF_RANGE, SW_HIDE, SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOZORDER,
    SWP_SHOWWINDOW, WM_GETFONT, WM_SETFONT, WM_SETREDRAW, WS_CHILD, WS_DISABLED, WS_EX_CLIENTEDGE,
    WS_EX_NOPARENTNOTIFY,
};

use crate::localization::{adapter_state, network_column_titles, network_graph_labels};
use crate::options::Options;
use crate::resource::{IDC_GRAPHSCROLLVERT, IDC_NICGRAPH, IDC_NICTOTALS, IDC_NOADAPTERS};
use crate::winutil::{finish_list_view_update, hiword, loword, subclass_list_view, to_wide_null};

const HIST_SIZE: usize = 2000;
const GRAPH_GRID: i32 = 12;
const MIN_GRAPH_HEIGHT: i32 = 120;
const SCROLLBAR_WIDTH: i32 = 17;
const DEFSPACING_BASE: i32 = 3;
const TOPSPACING_BASE: i32 = 10;
const DLG_SCALE_X: i32 = 4;
const DLG_SCALE_Y: i32 = 8;
const FRAME_CLASS_NAME: &str = "TaskManagerFrame";
const BUTTON_CLASS_NAME: &str = "Button";
const LVM_SETBKCOLOR: u32 = 0x1001;
const LVM_SETTEXTCOLOR: u32 = 0x1024;
const LVM_SETTEXTBKCOLOR: u32 = 0x1026;

struct RawAdapterEntry {
    interface_index: u32,
    key: String,
    name: String,
    state: String,
    link_speed_bps: u64,
    bytes_sent: u64,
    bytes_received: u64,
}

struct NetworkAdapterEntry {
    interface_index: u32,
    key: String,
    name: String,
    state: String,
    link_speed: String,
    utilization: String,
    bytes_sent: String,
    bytes_received: String,
    bytes_total: String,
    current_sent: u64,
    current_received: u64,
    sent_history: Vec<u8>,
    received_history: Vec<u8>,
    total_history: Vec<u8>,
    dirty: bool,
}

struct NetworkGraphControl {
    frame_hwnd: HWND,
    graph_hwnd: HWND,
}

#[derive(Default)]
pub struct NetworkPageState {
    // 网络页状态对象维护网卡采样缓存、图表窗口以及滚动位置。
    hwnd: HWND,
    main_hwnd: HWND,
    hwnd_tabs: HWND,
    no_title: bool,
    adapters: Vec<NetworkAdapterEntry>,
    graphs: Vec<NetworkGraphControl>,
    graphs_per_page: usize,
    first_visible_adapter: usize,
    scroll_offset: i32,
    last_sample_time: Option<Instant>,
}

impl NetworkPageState {
    pub fn new() -> Self {
        Self::default()
    }

    pub unsafe fn initialize(&mut self, hwnd: HWND, main_hwnd: HWND, hwnd_tabs: HWND) {
        // 网络页初始化时就先建列并做一次刷新，
        // 这样页面首次显示时不会先看到空壳列表和空白图表框。
        self.hwnd = hwnd;
        self.main_hwnd = main_hwnd;
        self.hwnd_tabs = hwnd_tabs;
        let list = self.list_hwnd();
        if !list.is_null() {
            subclass_list_view(list);
        }
        self.configure_columns();
        self.refresh();
    }


    pub fn apply_options(&mut self, options: &Options) {
        // 网络页当前只有无标题布局依赖全局选项，因此这里比较轻量。
        let previous = self.no_title;
        self.no_title = options.no_title();
        if self.hwnd.is_null() || previous == self.no_title {
            return;
        }

        unsafe {
            self.size_page();
        }
    }

    pub fn no_title(&self) -> bool {
        self.no_title
    }

    pub unsafe fn timer_event(&mut self) {
        // 每轮刷新都先推动网格滚动，再采样并重绘当前可见图表。
        self.scroll_offset = (self.scroll_offset + 2) % GRAPH_GRID;
        self.refresh();
        self.update_graphs();
    }

    pub unsafe fn destroy(&mut self) {
        self.destroy_graphs();
    }

    pub fn graph_pane_index(&self, control_id: i32) -> Option<usize> {
        let pane_index = control_id.saturating_sub(IDC_NICGRAPH) as usize;
        if control_id >= IDC_NICGRAPH && pane_index < self.graphs.len() {
            Some(pane_index)
        } else {
            None
        }
    }

    pub unsafe fn draw_graph(&self, hdc: HDC, rect: RECT, pane_index: usize) {
        // 每个图表面板都根据当前适配器的历史数据独立绘制，
        // 但缩放规则保持一致，便于横向比较。
        let adapter_index = pane_index.saturating_add(self.first_visible_adapter());
        let Some(adapter) = self.adapters.get(adapter_index) else {
            return;
        };

        let scale_max = adapter
            .sent_history
            .iter()
            .chain(adapter.received_history.iter())
            .chain(adapter.total_history.iter())
            .copied()
            .max()
            .unwrap_or(0);
        let zoom = graph_zoom(scale_max);
        fill_black(hdc, &rect);
        let plot_left = draw_scale(hdc, &rect, 100 / zoom);
        let plot_rect = RECT {
            left: (rect.left + plot_left).min(rect.right),
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        };

        if plot_rect.right > plot_rect.left {
            draw_grid(hdc, &plot_rect, self.scroll_offset, zoom);
            draw_history(hdc, &plot_rect, &adapter.total_history, rgb(0, 255, 0), zoom);
            draw_history(hdc, &plot_rect, &adapter.received_history, rgb(255, 255, 0), zoom);
            draw_history(hdc, &plot_rect, &adapter.sent_history, rgb(255, 0, 0), zoom);
        }

        let legend_top = rect.top + 4;
        draw_graph_text(
            hdc,
            RECT {
                left: plot_rect.left + 4,
                top: legend_top,
                right: (plot_rect.left + 52).min(plot_rect.right),
                bottom: legend_top + 12,
            },
            &adapter.utilization,
            rgb(0, 255, 0),
            DT_LEFT,
        );
        let labels = network_graph_labels();
        draw_graph_text(
            hdc,
            RECT {
                left: (plot_rect.right - 108).max(plot_rect.left),
                top: legend_top,
                right: (plot_rect.right - 72).max(plot_rect.left),
                bottom: legend_top + 12,
            },
            labels[0],
            rgb(0, 255, 0),
            DT_RIGHT,
        );
        draw_graph_text(
            hdc,
            RECT {
                left: (plot_rect.right - 72).max(plot_rect.left),
                top: legend_top,
                right: (plot_rect.right - 36).max(plot_rect.left),
                bottom: legend_top + 12,
            },
            labels[1],
            rgb(255, 255, 0),
            DT_RIGHT,
        );
        draw_graph_text(
            hdc,
            RECT {
                left: (plot_rect.right - 36).max(plot_rect.left),
                top: legend_top,
                right: plot_rect.right - 4,
                bottom: legend_top + 12,
            },
            labels[2],
            rgb(255, 0, 0),
            DT_RIGHT,
        );
        draw_graph_text(
            hdc,
            RECT {
                left: plot_rect.left + 4,
                top: rect.bottom - 14,
                right: rect.right - 4,
                bottom: rect.bottom - 2,
            },
            &adapter.link_speed,
            rgb(0, 255, 0),
            DT_RIGHT,
        );
    }

    pub unsafe fn size_page(&mut self) {
        // 网络页需要同时布局“多块图表 + 滚动条 + 底部列表”，
        // 因此会先算出一页能显示多少图，再决定是否出现滚动条。
        if self.hwnd.is_null() {
            return;
        }

        let list = self.list_hwnd();
        let label = GetDlgItem(self.hwnd, IDC_NOADAPTERS);
        let scrollbar = GetDlgItem(self.hwnd, IDC_GRAPHSCROLLVERT);
        let adapter_count = self.adapters.len();

        let mut parent_rect = zeroed::<RECT>();
        let (def_spacing, top_spacing) = layout_spacing();
        let mut graph_rect = zeroed::<RECT>();
        let mut graph_dim_rect = zeroed::<RECT>();
        let graph_history_height;
        let mut need_scrollbar = false;

        self.graphs_per_page = 0;

        if self.no_title {
            if self.main_hwnd.is_null() {
                return;
            }
            GetClientRect(self.main_hwnd, &mut parent_rect);
            graph_history_height = (parent_rect.bottom - parent_rect.top - def_spacing).max(0);
        } else {
            if self.hwnd_tabs.is_null() {
                return;
            }
            GetClientRect(self.hwnd_tabs, &mut parent_rect);
            MapWindowPoints(self.hwnd_tabs, self.hwnd, &mut parent_rect as *mut _ as _, 2);
            SendMessageW(self.hwnd_tabs, TCM_ADJUSTRECT, 0, &mut parent_rect as *mut _ as isize);
            graph_history_height = ((parent_rect.bottom - parent_rect.top - def_spacing) * 3 / 4).max(0);
        }

        if adapter_count != 0 {
            self.graphs_per_page = graphs_per_page(graph_history_height, adapter_count);
            self.ensure_graphs(self.graphs_per_page);
            need_scrollbar = adapter_count > self.graphs_per_page;
            graph_rect.left = parent_rect.left + def_spacing;
            graph_rect.right = (parent_rect.right - parent_rect.left)
                - def_spacing * 2
                - if need_scrollbar {
                    SCROLLBAR_WIDTH + def_spacing
                } else {
                    0
                };
            graph_rect.top = parent_rect.top + def_spacing;
            graph_rect.bottom = if self.graphs_per_page > 0 {
                graph_history_height / self.graphs_per_page as i32
            } else {
                0
            };
        }

        let mut hdwp = BeginDeferWindowPos(10);
        if hdwp.is_null() {
            return;
        }

        if !scrollbar.is_null() {
            hdwp = DeferWindowPos(
                hdwp,
                scrollbar,
                null_mut(),
                parent_rect.right - def_spacing - SCROLLBAR_WIDTH,
                parent_rect.top + def_spacing,
                SCROLLBAR_WIDTH,
                graph_rect.bottom * self.graphs_per_page as i32,
                if need_scrollbar {
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW
                } else {
                    SWP_HIDEWINDOW
                },
            );
        }

        for index in 0..self.graphs.len() {
            if index < self.graphs_per_page {
                let frame = &self.graphs[index];
                hdwp = size_graph(hdwp, frame, &graph_rect, &mut graph_dim_rect, def_spacing, top_spacing);
                graph_rect.top += graph_rect.bottom;
            } else {
                let frame = &self.graphs[index];
                hdwp = hide_graph(hdwp, frame);
            }
        }

        if !list.is_null() {
            hdwp = DeferWindowPos(
                hdwp,
                list,
                null_mut(),
                graph_rect.left,
                graph_rect.top + def_spacing,
                (parent_rect.right - parent_rect.left - graph_rect.left - def_spacing).max(0),
                (parent_rect.bottom - graph_rect.top - def_spacing).max(0),
                if adapter_count == 0 {
                    SWP_HIDEWINDOW
                } else {
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW
                },
            );
        }

        if !label.is_null() {
            hdwp = DeferWindowPos(
                hdwp,
                label,
                null_mut(),
                parent_rect.left,
                parent_rect.top + ((parent_rect.bottom - parent_rect.top) / 2) - 40,
                (parent_rect.right - parent_rect.left).max(0),
                (parent_rect.bottom - parent_rect.top).max(0),
                if adapter_count == 0 {
                    SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW
                } else {
                    SWP_HIDEWINDOW
                },
            );
        }

        EndDeferWindowPos(hdwp);

        if need_scrollbar && !scrollbar.is_null() {
            let scroll_info = SCROLLINFO {
                cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                fMask: SIF_RANGE | SIF_PAGE,
                nMin: 0,
                nMax: adapter_count.saturating_sub(self.graphs_per_page) as i32,
                nPage: 1,
                ..zeroed()
            };
            SetScrollInfo(scrollbar, SB_CTL, &scroll_info, 1);
        }

        let _ = graph_dim_rect;
        self.label_graphs();
    }

    pub unsafe fn handle_vscroll(&mut self, wparam: WPARAM) -> isize {
        let scrollbar = GetDlgItem(self.hwnd, IDC_GRAPHSCROLLVERT);
        if scrollbar.is_null() {
            return 0;
        }

        let mut scroll_info = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_ALL,
            ..zeroed()
        };
        if GetScrollInfo(scrollbar, SB_CTL, &mut scroll_info) == 0 {
            return 0;
        }

        match loword(wparam) as i32 {
            SB_BOTTOM => scroll_info.nPos = scroll_info.nMax,
            SB_TOP => scroll_info.nPos = scroll_info.nMin,
            SB_LINEDOWN => scroll_info.nPos += 1,
            SB_LINEUP => scroll_info.nPos -= 1,
            SB_PAGEDOWN => scroll_info.nPos += self.graphs_per_page as i32,
            SB_PAGEUP => scroll_info.nPos -= self.graphs_per_page as i32,
            SB_THUMBTRACK | SB_THUMBPOSITION => scroll_info.nPos = hiword(wparam) as i32,
            _ => {}
        }

        if scroll_info.nPos < scroll_info.nMin {
            scroll_info.nPos = scroll_info.nMin;
        }
        if scroll_info.nPos > scroll_info.nMax {
            scroll_info.nPos = scroll_info.nMax;
        }

        self.first_visible_adapter = scroll_info.nPos.max(0) as usize;
        scroll_info.fMask = SIF_POS;
        SetScrollInfo(scrollbar, SB_CTL, &scroll_info, 1);
        self.label_graphs();
        1
    }

    unsafe fn refresh(&mut self) {
        // 网络页刷新会把原始计数器转换为“总量 + 利用率 + 历史曲线”三类数据，
        // 这样图表和列表可以共享同一份采样结果。
        let raw_adapters = self.collect_adapters();
        let now = Instant::now();
        let elapsed_secs = self
            .last_sample_time
            .replace(now)
            .map(|previous| now.duration_since(previous).as_secs_f64())
            .unwrap_or(0.0);

        let mut previous_by_key = HashMap::with_capacity(self.adapters.len());
        for adapter in self.adapters.drain(..) {
            previous_by_key.insert(adapter.key.clone(), adapter);
        }

        let mut adapters = Vec::with_capacity(raw_adapters.len());
        for raw in raw_adapters {
            let previous = previous_by_key.remove(&raw.key);
            let (sent_delta, received_delta) = if let Some(previous_adapter) = previous.as_ref() {
                (
                    raw.bytes_sent.saturating_sub(previous_adapter.current_sent),
                    raw.bytes_received.saturating_sub(previous_adapter.current_received),
                )
            } else {
                (0, 0)
            };
            let total_delta = sent_delta.saturating_add(received_delta);

            let sent_util = utilization_percent(sent_delta, raw.link_speed_bps, elapsed_secs);
            let received_util = utilization_percent(received_delta, raw.link_speed_bps, elapsed_secs);
            let total_util = utilization_percent(total_delta, raw.link_speed_bps, elapsed_secs);

            let mut sent_history = previous
                .as_ref()
                .map(|adapter| adapter.sent_history.clone())
                .unwrap_or_else(|| vec![0; HIST_SIZE]);
            let mut received_history = previous
                .as_ref()
                .map(|adapter| adapter.received_history.clone())
                .unwrap_or_else(|| vec![0; HIST_SIZE]);
            let mut total_history = previous
                .as_ref()
                .map(|adapter| adapter.total_history.clone())
                .unwrap_or_else(|| vec![0; HIST_SIZE]);

            push_history(&mut sent_history, sent_util);
            push_history(&mut received_history, received_util);
            push_history(&mut total_history, total_util);

            let bytes_total = raw.bytes_sent.saturating_add(raw.bytes_received);
            let mut adapter = NetworkAdapterEntry {
                interface_index: raw.interface_index,
                key: raw.key,
                name: raw.name,
                state: raw.state,
                link_speed: format_link_speed(raw.link_speed_bps),
                utilization: format!("{}%", total_util),
                bytes_sent: format_counter(raw.bytes_sent),
                bytes_received: format_counter(raw.bytes_received),
                bytes_total: format_counter(bytes_total),
                current_sent: raw.bytes_sent,
                current_received: raw.bytes_received,
                sent_history,
                received_history,
                total_history,
                dirty: true,
            };
            if let Some(previous_adapter) = previous.as_ref() {
                adapter.dirty = previous_adapter.name != adapter.name
                    || previous_adapter.state != adapter.state
                    || previous_adapter.link_speed != adapter.link_speed
                    || previous_adapter.utilization != adapter.utilization
                    || previous_adapter.bytes_sent != adapter.bytes_sent
                    || previous_adapter.bytes_received != adapter.bytes_received
                    || previous_adapter.bytes_total != adapter.bytes_total;
            }
            adapters.push(adapter);
        }

        adapters.sort_by(|left, right| left.name.cmp(&right.name));
        self.adapters = adapters;
        self.update_listview();
        self.size_page();
    }

    unsafe fn collect_adapters(&self) -> Vec<RawAdapterEntry> {
        let mut table = null_mut::<MIB_IF_TABLE2>();
        if GetIfTable2(&mut table) != 0 || table.is_null() {
            return Vec::new();
        }

        let mut adapters = Vec::new();
        let count = (*table).NumEntries as usize;
        let rows = slice::from_raw_parts((*table).Table.as_ptr(), count);
        for row in rows {
            if !include_adapter(row) {
                continue;
            }

            let mut name = wide_array_to_string(&row.Alias);
            if name.is_empty() {
                name = wide_array_to_string(&row.Description);
            }
            let description = wide_array_to_string(&row.Description);
            let key = format!("{}|{}|{}", row.InterfaceIndex, name, description);

            adapters.push(RawAdapterEntry {
                interface_index: row.InterfaceIndex,
                key,
                name,
                state: adapter_state_text(row.OperStatus),
                link_speed_bps: row.ReceiveLinkSpeed.max(row.TransmitLinkSpeed),
                bytes_sent: row.OutOctets,
                bytes_received: row.InOctets,
            });
        }

        FreeMibTable(table as _);
        adapters.sort_by(|left, right| left.name.cmp(&right.name));
        adapters
    }

    unsafe fn configure_columns(&self) {
        let list = self.list_hwnd();
        if list.is_null() {
            return;
        }

        SendMessageW(
            list,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            (LVS_EX_HEADERDRAGDROP | LVS_EX_FULLROWSELECT) as usize,
            (LVS_EX_HEADERDRAGDROP | LVS_EX_FULLROWSELECT) as isize,
        );
        SendMessageW(list, LVM_SETBKCOLOR, 0, rgb(0, 0, 0) as isize);
        SendMessageW(list, LVM_SETTEXTBKCOLOR, 0, rgb(0, 0, 0) as isize);
        SendMessageW(list, LVM_SETTEXTCOLOR, 0, rgb(0, 255, 0) as isize);

        while SendMessageW(list, 0x101C, 0, 0) != 0 {}

        let titles = network_column_titles();
        let columns = [
            (titles[0], 150, LVCFMT_LEFT),
            (titles[1], 96, LVCFMT_RIGHT),
            (titles[2], 90, LVCFMT_RIGHT),
            (titles[3], 90, LVCFMT_LEFT),
            (titles[4], 90, LVCFMT_RIGHT),
            (titles[5], 96, LVCFMT_RIGHT),
            (titles[6], 90, LVCFMT_RIGHT),
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
            SendMessageW(list, 0x1061, index, &mut column as *mut _ as isize);
        }
    }

    unsafe fn update_listview(&self) {
        // 列表只在适配器身份变化时替换整行，普通数值更新尽量走原位写回。
        let list = self.list_hwnd();
        if list.is_null() {
            return;
        }

        SendMessageW(list, WM_SETREDRAW, 0, 0);

        let mut existing_count = SendMessageW(list, LVM_GETITEMCOUNT, 0, 0) as usize;
        let common_count = existing_count.min(self.adapters.len());

        for index in 0..common_count {
            let adapter = &self.adapters[index];
            let mut current_item = LVITEMW {
                mask: LVIF_PARAM,
                iItem: index as i32,
                ..zeroed()
            };
            let current_interface_index =
                if SendMessageW(list, LVM_GETITEMW, 0, &mut current_item as *mut _ as isize) != 0 {
                    Some(current_item.lParam as u32)
                } else {
                    None
                };

            if current_interface_index != Some(adapter.interface_index) {
                self.replace_row(list, index, adapter);
            } else if adapter.dirty {
                self.update_row(list, index, adapter);
            }
        }

        while existing_count > self.adapters.len() {
            existing_count -= 1;
            SendMessageW(list, LVM_DELETEITEM, existing_count, 0);
        }

        for index in common_count..self.adapters.len() {
            self.insert_row(list, index, &self.adapters[index]);
        }

        finish_list_view_update(list);
    }

    unsafe fn insert_row(&self, list: HWND, index: usize, adapter: &NetworkAdapterEntry) {
        let mut name = to_wide_null(&adapter.name);
        let mut item = LVITEMW {
            mask: LVIF_TEXT | LVIF_PARAM,
            iItem: index as i32,
            iSubItem: 0,
            pszText: name.as_mut_ptr(),
            cchTextMax: name.len() as i32,
            lParam: adapter.interface_index as isize,
            ..zeroed()
        };
        SendMessageW(list, LVM_INSERTITEMW, 0, &mut item as *mut _ as isize);
        self.update_row(list, index, adapter);
    }

    unsafe fn replace_row(&self, list: HWND, index: usize, adapter: &NetworkAdapterEntry) {
        let mut name = to_wide_null(&adapter.name);
        let mut item = LVITEMW {
            mask: LVIF_TEXT | LVIF_PARAM,
            iItem: index as i32,
            iSubItem: 0,
            pszText: name.as_mut_ptr(),
            cchTextMax: name.len() as i32,
            lParam: adapter.interface_index as isize,
            ..zeroed()
        };
        SendMessageW(list, 0x1076, 0, &mut item as *mut _ as isize);
        self.update_row(list, index, adapter);
    }

    unsafe fn update_row(&self, list: HWND, index: usize, adapter: &NetworkAdapterEntry) {
        for (subitem, text) in [
            &adapter.utilization,
            &adapter.link_speed,
            &adapter.state,
            &adapter.bytes_sent,
            &adapter.bytes_received,
            &adapter.bytes_total,
        ]
        .iter()
        .enumerate()
        {
            let mut value = to_wide_null(text);
            let mut subitem_item = LVITEMW {
                mask: LVIF_TEXT,
                iItem: index as i32,
                iSubItem: (subitem + 1) as i32,
                pszText: value.as_mut_ptr(),
                cchTextMax: value.len() as i32,
                ..zeroed()
            };
            SendMessageW(list, 0x1076, 0, &mut subitem_item as *mut _ as isize);
        }
    }

    unsafe fn ensure_graphs(&mut self, required: usize) {
        if required <= self.graphs.len() || self.hwnd.is_null() {
            return;
        }

        let frame_class = to_wide_null(FRAME_CLASS_NAME);
        let button_class = to_wide_null(BUTTON_CLASS_NAME);
        let empty_text = to_wide_null("");
        let font = SendMessageW(self.hwnd, WM_GETFONT, 0, 0);

        while self.graphs.len() < required {
            let graph_id = IDC_NICGRAPH + self.graphs.len() as i32;
            let graph_hwnd = CreateWindowExW(
                WS_EX_CLIENTEDGE,
                button_class.as_ptr(),
                empty_text.as_ptr(),
                WS_CHILD | WS_DISABLED | BS_OWNERDRAW as u32,
                0,
                0,
                0,
                0,
                self.hwnd,
                graph_id as usize as HMENU,
                null_mut(),
                null_mut(),
            );
            let frame_hwnd = CreateWindowExW(
                WS_EX_NOPARENTNOTIFY,
                frame_class.as_ptr(),
                empty_text.as_ptr(),
                0x0000_0007 | WS_CHILD,
                0,
                0,
                0,
                0,
                self.hwnd,
                null_mut(),
                null_mut(),
                null_mut(),
            );
            if graph_hwnd.is_null() || frame_hwnd.is_null() {
                if !graph_hwnd.is_null() {
                    DestroyWindow(graph_hwnd);
                }
                if !frame_hwnd.is_null() {
                    DestroyWindow(frame_hwnd);
                }
                break;
            }

            SendMessageW(frame_hwnd, WM_SETFONT, font as usize, 0);
            SendMessageW(graph_hwnd, WM_SETFONT, font as usize, 0);
            ShowWindow(frame_hwnd, SW_HIDE);
            ShowWindow(graph_hwnd, SW_HIDE);
            self.graphs.push(NetworkGraphControl {
                frame_hwnd,
                graph_hwnd,
            });
        }
    }

    unsafe fn destroy_graphs(&mut self) {
        for graph in self.graphs.drain(..) {
            if !graph.graph_hwnd.is_null() {
                DestroyWindow(graph.graph_hwnd);
            }
            if !graph.frame_hwnd.is_null() {
                DestroyWindow(graph.frame_hwnd);
            }
        }
        self.graphs_per_page = 0;
        self.first_visible_adapter = 0;
    }

    unsafe fn update_graphs(&self) {
        // 只重绘当前一页真正可见的图表，避免隐藏面板也跟着刷新。
        for pane_index in 0..self.graphs_per_page {
            let Some(graph) = self.graphs.get(pane_index) else {
                break;
            };
            InvalidateRect(graph.graph_hwnd, null_mut(), 0);
            UpdateWindow(graph.graph_hwnd);
        }
    }

    unsafe fn label_graphs(&mut self) {
        // 图表标题始终绑定当前可见适配器切片，滚动后要一起更新标题文字。
        let first_visible = self.first_visible_adapter();
        for pane_index in 0..self.graphs_per_page {
            let Some(graph) = self.graphs.get(pane_index) else {
                break;
            };
            if let Some(adapter) = self.adapters.get(first_visible + pane_index) {
                let title = to_wide_null(&adapter.name);
                SetWindowTextW(graph.frame_hwnd, title.as_ptr());
            } else {
                let title = to_wide_null("");
                SetWindowTextW(graph.frame_hwnd, title.as_ptr());
            }
        }
        self.update_graphs();
    }

    fn first_visible_adapter(&self) -> usize {
        if self.adapters.is_empty() || self.graphs_per_page == 0 {
            return 0;
        }

        self.first_visible_adapter
            .min(self.adapters.len().saturating_sub(self.graphs_per_page.min(self.adapters.len())))
    }

    fn list_hwnd(&self) -> HWND {
        unsafe { GetDlgItem(self.hwnd, IDC_NICTOTALS) }
    }
}

unsafe fn size_graph(
    mut hdwp: HDWP,
    graph: &NetworkGraphControl,
    rect: &RECT,
    dim_rect: &mut RECT,
    def_spacing: i32,
    top_spacing: i32,
) -> HDWP {
    // 单个网络图由“外层 frame + 内层 owner-draw graph”两层控件组成，这里一次性定位它们。
    let graph_width = (rect.right - def_spacing * 2).max(0);
    let graph_height = (rect.bottom - top_spacing - def_spacing).max(0);

    hdwp = DeferWindowPos(
        hdwp,
        graph.frame_hwnd,
        null_mut(),
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );

    let left = rect.left + def_spacing;
    let top = rect.top + top_spacing;
    let right = left + graph_width;
    let bottom = top + graph_height;
    *dim_rect = RECT { left, top, right, bottom };

    DeferWindowPos(
        hdwp,
        graph.graph_hwnd,
        null_mut(),
        left,
        top,
        right - left,
        bottom - top,
        SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
    )
}

unsafe fn hide_graph(mut hdwp: HDWP, graph: &NetworkGraphControl) -> HDWP {
    hdwp = DeferWindowPos(
        hdwp,
        graph.frame_hwnd,
        null_mut(),
        0,
        0,
        0,
        0,
        SWP_NOZORDER | SWP_NOACTIVATE | SWP_HIDEWINDOW,
    );

    DeferWindowPos(
        hdwp,
        graph.graph_hwnd,
        null_mut(),
        0,
        0,
        0,
        0,
        SWP_NOZORDER | SWP_NOACTIVATE | SWP_HIDEWINDOW,
    )
}

fn graphs_per_page(graph_height: i32, adapter_count: usize) -> usize {
    // 每页图表数量基于当前可用高度动态决定，但不会低于最小可读高度。
    if graph_height <= 0 || adapter_count == 0 {
        return 0;
    }

    let average_height = (graph_height / adapter_count as i32).max(MIN_GRAPH_HEIGHT);
    if graph_height > average_height {
        (graph_height / average_height).max(1) as usize
    } else {
        1
    }
}

fn push_history(history: &mut [u8], value: u8) {
    // 与性能页一致，网络历史按“最新样本在前”滚动。
    if history.is_empty() {
        return;
    }

    history.copy_within(..history.len() - 1, 1);
    history[0] = value;
}

fn include_adapter(row: &MIB_IF_ROW2) -> bool {
    // 经典任务管理器不显示 loopback / tunnel，这里保持相同过滤策略。
    row.Type != IF_TYPE_SOFTWARE_LOOPBACK && row.Type != IF_TYPE_TUNNEL
}

fn wide_array_to_string(value: &[u16]) -> String {
    let end = value.iter().position(|&ch| ch == 0).unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end]).trim().to_string()
}

fn adapter_state_text(oper_status: i32) -> String {
    // 操作状态来自 IP Helper API 的枚举值，这里映射成 UI 层展示文案。
    if oper_status == IfOperStatusUp {
        adapter_state("Connected").to_string()
    } else {
        match oper_status {
            2 => adapter_state("Disconnected").to_string(),
            3 => adapter_state("Connecting").to_string(),
            4 => adapter_state("Disconnecting").to_string(),
            5 => adapter_state("Hardware Missing").to_string(),
            6 => adapter_state("Hardware Disabled").to_string(),
            7 => adapter_state("Hardware Malfunction").to_string(),
            _ => adapter_state("Unknown").to_string(),
        }
    }
}

fn utilization_percent(bytes_per_interval: u64, link_speed_bps: u64, elapsed_secs: f64) -> u8 {
    // 利用率按“本轮字节数 -> bit/s -> 除以链路速率”计算，并限制在 0-100。
    if bytes_per_interval == 0 || link_speed_bps == 0 || elapsed_secs <= 0.0 {
        return 0;
    }

    let bits_per_second = (bytes_per_interval as f64 * 8.0) / elapsed_secs;
    ((bits_per_second * 100.0) / link_speed_bps as f64)
        .round()
        .clamp(0.0, 100.0) as u8
}

fn format_link_speed(bits_per_second: u64) -> String {
    // 链路速率采用十进制网络单位显示，更符合网卡/交换机常见标注方式。
    if bits_per_second == 0 {
        return "-".to_string();
    }

    let units = ["bps", "Kbps", "Mbps", "Gbps", "Tbps"];
    let mut value = bits_per_second as f64;
    let mut unit = 0usize;
    while value >= 1000.0 && unit + 1 < units.len() {
        value /= 1000.0;
        unit += 1;
    }

    if value >= 100.0 || unit == 0 {
        format!("{value:.0} {}", units[unit])
    } else {
        format!("{value:.1} {}", units[unit])
    }
}

fn format_counter(value: u64) -> String {
    if value == 0 {
        return "0".to_string();
    }

    let digits = value.to_string();
    let mut output = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index != 0 && (digits.len() - index) % 3 == 0 {
            output.push(',');
        }
        output.push(ch);
    }
    output
}

unsafe fn fill_black(hdc: HDC, rect: &RECT) {
    FillRect(hdc, rect, GetStockObject(BLACK_BRUSH) as HBRUSH);
}

unsafe fn draw_scale(hdc: HDC, rect: &RECT, max_scale_value: u32) -> i32 {
    // 刻度区单独占据左侧一列，返回值是后续真正绘图区域的左边界偏移。
    let top_text = format!("{max_scale_value} %");
    let middle_text = format!("{} %", max_scale_value / 2);
    let bottom_text = "0 %";
    let (sample_width, sample_height) = measure_graph_text(hdc, " 22.5 %");
    let (top_width, top_height) = measure_graph_text(hdc, &top_text);
    let (middle_width, middle_height) = measure_graph_text(hdc, &middle_text);
    let (bottom_width, bottom_height) = measure_graph_text(hdc, bottom_text);
    let scale_width = sample_width.max(top_width).max(middle_width).max(bottom_width);
    let scale_height = sample_height.max(top_height).max(middle_height).max(bottom_height);
    let divider_x = rect.left + scale_width;

    draw_graph_text(
        hdc,
        RECT {
            left: rect.left,
            top: rect.top,
            right: divider_x - 3,
            bottom: rect.top + scale_height,
        },
        &top_text,
        rgb(255, 255, 0),
        DT_RIGHT,
    );
    draw_graph_text(
        hdc,
        RECT {
            left: rect.left,
            top: rect.top + ((rect.bottom - rect.top - scale_height) / 2),
            right: divider_x - 3,
            bottom: rect.top + ((rect.bottom - rect.top + scale_height) / 2),
        },
        &middle_text,
        rgb(255, 255, 0),
        DT_RIGHT,
    );
    draw_graph_text(
        hdc,
        RECT {
            left: rect.left,
            top: rect.bottom - scale_height,
            right: divider_x - 3,
            bottom: rect.bottom,
        },
        bottom_text,
        rgb(255, 255, 0),
        DT_RIGHT,
    );

    let pen = CreatePen(PS_SOLID, 1, rgb(255, 255, 0));
    if !pen.is_null() {
        let old_pen = SelectObject(hdc, pen as _);
        MoveToEx(hdc, divider_x, rect.top, null_mut());
        LineTo(hdc, divider_x, rect.bottom);
        SelectObject(hdc, old_pen);
        DeleteObject(pen as _);
    }

    scale_width + 3
}

unsafe fn measure_graph_text(hdc: HDC, text: &str) -> (i32, i32) {
    let mut text_wide = to_wide_null(text);
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    DrawTextW(
        hdc,
        text_wide.as_mut_ptr(),
        -1,
        &mut rect,
        DT_CALCRECT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX,
    );
    ((rect.right - rect.left).max(1), (rect.bottom - rect.top).max(1))
}

unsafe fn draw_grid(hdc: HDC, rect: &RECT, scroll_offset: i32, zoom: u32) {
    // 网格密度会随着 zoom 调整，避免在低利用率场景下曲线长期贴底看不清。
    let pen = CreatePen(PS_SOLID, 1, rgb(0, 128, 64));
    if pen.is_null() {
        return;
    }

    let old_pen = SelectObject(hdc, pen as _);
    let square_height = GRAPH_GRID + ((20 * (100 - (100 / zoom.max(1) as i32))) / 100);

    let mut y = rect.bottom - square_height - 1;
    while y > rect.top {
        MoveToEx(hdc, rect.left, y, null_mut());
        LineTo(hdc, rect.right, y);
        y -= square_height.max(1);
    }

    let mut x = rect.right - scroll_offset;
    while x > rect.left {
        MoveToEx(hdc, x, rect.top, null_mut());
        LineTo(hdc, x, rect.bottom);
        x -= GRAPH_GRID;
    }

    SelectObject(hdc, old_pen);
    DeleteObject(pen as _);
}

unsafe fn draw_history(hdc: HDC, rect: &RECT, history: &[u8], color: u32, zoom: u32) {
    // 三条折线都共用这套绘制函数，只靠颜色区分 total / received / sent。
    if history.is_empty() {
        return;
    }

    let width = (rect.right - rect.left).max(1) as usize;
    let graph_height = (rect.bottom - rect.top).max(1);
    let scale = ((width - 1) / HIST_SIZE).max(1);

    let pen = CreatePen(PS_SOLID, 1, color);
    if pen.is_null() {
        return;
    }

    let old_pen = SelectObject(hdc, pen as _);
    MoveToEx(
        hdc,
        rect.right,
        scaled_history_y(rect, graph_height, history[0], zoom),
        null_mut(),
    );

    for (index, value) in history.iter().enumerate() {
        if index * scale >= width {
            break;
        }
        LineTo(
            hdc,
            rect.right - (scale * index) as i32,
            scaled_history_y(rect, graph_height, *value, zoom),
        );
    }

    SelectObject(hdc, old_pen);
    DeleteObject(pen as _);
}

fn scaled_history_y(rect: &RECT, graph_height: i32, value: u8, zoom: u32) -> i32 {
    if value == 0 {
        return rect.bottom - 1;
    }

    let scaled_value = ((value as i32 * graph_height * zoom as i32) / 100).clamp(1, graph_height);
    rect.bottom - scaled_value
}

fn graph_zoom(scale_max: u8) -> u32 {
    // 当前量级较小时自动放大坐标，让低流量波动也能在图上看出来。
    match scale_max {
        0 => 100,
        1..=4 => 20,
        5..=24 => 4,
        25..=49 => 2,
        _ => 1,
    }
}

unsafe fn draw_graph_text(hdc: HDC, mut rect: RECT, text: &str, color: u32, align: u32) {
    let mut text_wide = to_wide_null(text);
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, color);
    DrawTextW(
        hdc,
        text_wide.as_mut_ptr(),
        -1,
        &mut rect,
        align | DT_TOP | DT_SINGLELINE | DT_NOPREFIX,
    );
}

unsafe fn layout_spacing() -> (i32, i32) {
    let units = GetDialogBaseUnits() as usize;
    let def_spacing = (DEFSPACING_BASE * loword(units) as i32) / DLG_SCALE_X;
    let top_spacing = (TOPSPACING_BASE * hiword(units) as i32) / DLG_SCALE_Y;
    (def_spacing, top_spacing)
}

const fn rgb(red: u8, green: u8, blue: u8) -> u32 {
    red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
}











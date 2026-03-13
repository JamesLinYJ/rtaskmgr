# Rust Migration Notes

当前仓库已经新增一套纯 Rust 的 Win32 工程骨架，目标是用更安全、命名更清晰的 Rust 代码逐步替代原来的 C++ Task Manager 工程。

本轮已经完成：

- `Cargo.toml` / `.cargo/config.toml` / `build.rs`
  - 默认目标改为 `x86_64-pc-windows-msvc`
  - 资源脚本优先走 `rc.exe`，保留 `windres` 兜底
- 纯 Rust 入口与宿主层
  - 单实例互斥与激活
  - 主对话框创建
  - 状态栏与 Tab 初始化
  - 托盘图标与基础刷新
  - 注册表选项二进制读写骨架
- 三个页面的 Rust 对话框包装
  - `Applications`
  - `Processes`
  - `Performance`
- `Performance` 页的第一批 Rust 迁移
  - 使用 `NtQuerySystemInformation` 维护每 CPU 历史样本
  - 使用 `K32GetPerformanceInfo` 刷新提交内存、物理内存、内核内存、句柄/线程/进程计数
  - owner-draw CPU/内存图表与基础仪表绘制
  - 已补上更接近原版的离屏历史图位图复用、滚动网格和 CPU/内存历史缩放策略
  - `All CPUs` / `One Graph Per CPU`、`Kernel Times`、`No Title` 选项联动
  - 状态栏/托盘优先复用性能页采样结果
- `Applications` 页的第一批 Rust 迁移
  - 顶层可见窗口枚举与标题过滤
  - 当前窗口站下的多桌面窗口枚举与任务去重
  - 新工作线程上的跨窗口站枚举，避免主 UI 线程切换窗口站上下文
  - listview 列初始化、排序、增量刷新与图标列表
  - 刷新模型已改回更接近原版 `CTaskPage::TimerEvent` / `UpdateTaskListview`
  - 任务对象现在按 `pass count` 持久保留、原地更新、仅在表头点击时重排
  - stale 任务删除时会同步 `ImageList_Remove` 并修正剩余 icon index
  - `LVN_GETDISPINFOW` 现在按行 `lParam` 取数据，减少刷新期文本回写和额外闪烁
  - 双击切换、切换到前台、结束任务、最小化/最大化、层叠/平铺、置前
  - 右键菜单与视图模式联动
- `Processes` 页的第一批 Rust 迁移
  - Toolhelp 进程枚举与按 PID 保持选择
  - CPU 时间 / 工作集 / 页错误 / 提交大小 / 句柄数等基础采样
  - listview 列配置、排序、刷新与“选择列”对话框
  - 刷新模型已继续向原版 `UpdateProcInfoArray` / `UpdateProcListview` 靠拢
  - 进程条目现在按 `pass count` 持久保留、原地更新、按 dirty 位触发单行重绘
  - 列表显示回调现在按行 `lParam` 取 entry，减少刷新期整行文本回写
  - 结束进程、优先级调整、亲和性对话框、AeDebug 调试器启动
  - `Applications -> Go To Process` 已切换到 Rust 进程页定位
  - `WM_FINDPROC` 现在会先按 WOW 任务线程 ID 查找，再回退到真实 PID
  - `Show 16-bit Tasks` 会动态加载 `vdmdbg.dll`，枚举 WOW/16-bit 伪进程并支持终止 WOW 任务
  - WOW 任务右键菜单会按原版禁用 `Debug` / 优先级，移除 `Affinity`
  - `LVN_ITEMCHANGED` 和 `LVN_GETDISPINFOW` 已继续向原版靠拢，只在状态变化时刷新选择状态，并按行 `lParam` 提供显示文本
- 宿主层菜单行为继续向原版靠拢
  - `IDM_RUN` 改为动态调用 `shell32!RunFileDlg(ordinal 61)`
  - `IDM_HELP` 改为调用 `WinHelpW("taskmgr.hlp", HELP_FINDER)`
  - `IDM_SHOW16BIT` / CPU 图历史模式 / 任务视图模式切换后立即刷新对应页面
  - `WM_MENUSELECT` 状态栏菜单帮助文本恢复为按资源 ID 动态加载
  - 置顶窗口的右键临时隐藏/松开恢复逻辑已接回宿主层
  - `Applications` / `Processes` / 托盘弹出菜单都会同步宿主 `in popup` 状态

还没完成一比一迁移的部分：

- `taskpage.cpp` 还差少量旧菜单限制策略和个别视图边角
- `procpage.cpp` 的主体刷新模型已经切成持久 entry / `pass count` / dirty 位，但 WOW 任务更老的历史兼容细节、PID 复用边角和极旧系统分支还没完全照搬
- `perfpage.cpp` 的图例文字和极细的像素级布局对齐还没完全做完
- 一些老代码里的帮助文件/旧资源可用性，以及更细的托盘边角行为

当前环境状态：

- 本机已确认可用 `cargo` / `rustc`
- 本机已确认可用 `rc.exe` / `link.exe`
- 已能编译出 `x86_64-pc-windows-msvc` 的 `dev` / `release` 构建

后续建议顺序：

1. 继续补齐 `perfpage.cpp` 的图例文字和最后一层像素级布局
2. 收紧 `procpage.cpp` 的 WOW 细枝末节和错误提示细节
3. 回头继续打磨 `taskpage.cpp` 的剩余菜单/视图边角

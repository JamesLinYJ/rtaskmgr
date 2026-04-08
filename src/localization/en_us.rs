//! 英文（美国）语言表。

use super::text_key::TextKey;

pub fn resource(_id: u32) -> &'static str {
    // 旧式资源 ID 到英文字符串的映射表。
    match _id {
        crate::resource::IDS_PERFPAGETITLE => "Performance",
        crate::resource::IDS_NETPAGETITLE => "Networking",
        crate::resource::IDS_RUNTITLE => "Run",
        crate::resource::IDS_RUNTEXT => {
            "Type the name of a program, folder, document, or Internet resource, and Windows will open it for you."
        }
        crate::resource::IDS_APPTITLE | crate::resource::IDS_TASKMGR => "Windows NT Task Manager",
        crate::resource::IDS_PROCPAGETITLE => "Processes",
        crate::resource::IDS_TASKPAGETITLE => "Applications",
        crate::resource::IDS_USERPAGETITLE => "Users",
        crate::resource::IDS_TASKMGRDISABLED => "Task Manager has been disabled by your administrator.",
        crate::resource::IDS_WARNING => "Task Manager Warning",
        crate::resource::IDS_PRICHANGE => {
            "WARNING: Changing the priority class of this process may\ncause undesired results including system instability.  Are you\nsure you want to change the priority class?"
        }
        crate::resource::IDS_KILL => {
            "WARNING: Terminating a process can cause undesired\nresults including loss of data and system instability.  The\nprocess will not be given the chance to save its state or\ndata before it is terminated.  Are you sure you want to\nterminate the process?"
        }
        crate::resource::IDS_DEBUG => {
            "WARNING: Debugging this process may result in loss of data.\nAre you sure you wish to attach the debugger?"
        }
        crate::resource::IDS_LOW => "Low",
        crate::resource::IDS_HIGH => "High",
        crate::resource::IDS_REALTIME => "Realtime",
        crate::resource::IDS_NORMAL => "Normal",
        crate::resource::IDS_UNKNOWN => "Unknown",
        crate::resource::IDS_CANTSETAFFINITY => "The operation could not be completed.\n\n",
        crate::resource::IDS_CANTKILL => "Unable to Terminate Process",
        crate::resource::IDS_CANTDEBUG => "Unable to Attach Debugger",
        crate::resource::IDS_CANTCHANGEPRI => "Unable to Change Priority",
        crate::resource::IDS_FMTPROCS => "Processes: %d",
        crate::resource::IDS_FMTCPU => "CPU Usage: %d%%",
        crate::resource::IDS_FMTMEM => "Mem Usage: %dK / %dK",
        crate::resource::IDS_COL_TASKNAME => "Task",
        crate::resource::IDS_COL_TASKSTATUS => "Status",
        crate::resource::IDS_COL_TASKWINSTATION => "WinStation",
        crate::resource::IDS_COL_TASKDESKTOP => "Desktop",
        crate::resource::IDS_INVALIDOPTION => "Invalid Option",
        crate::resource::IDS_NOAFFINITYMASK => {
            "The process must have affinity with at least one processor."
        }
        crate::resource::IDS_FMTCPUNUM => "CPU %d",
        crate::resource::IDS_TOTALTIME => "Total CPU",
        crate::resource::IDS_KERNELTIME => "Kernel CPU",
        crate::resource::IDS_MEMUSAGE => "Memory Usage",
        crate::resource::IDS_COL_IMAGENAME => "Image Name",
        crate::resource::IDS_COL_PID => "PID",
        crate::resource::IDS_COL_CPU => "CPU",
        crate::resource::IDS_COL_CPUTIME => "CPU Time",
        crate::resource::IDS_COL_MEMUSAGE => "Mem Usage",
        crate::resource::IDS_COL_MEMUSAGEDIFF => "Mem Delta",
        crate::resource::IDS_COL_PAGEFAULTS => "Page Faults",
        crate::resource::IDS_COL_PAGEFAULTSDIFF => "PF Delta",
        crate::resource::IDS_COL_COMMITCHARGE => "VM Size",
        crate::resource::IDS_COL_PAGEDPOOL => "Paged Pool",
        crate::resource::IDS_COL_NONPAGEDPOOL => "NP Pool",
        crate::resource::IDS_COL_BASEPRIORITY => "Base Pri",
        crate::resource::IDS_COL_HANDLECOUNT => "Handles",
        crate::resource::IDS_COL_THREADCOUNT => "Threads",
        crate::resource::IDS_COL_SESSIONID => "Session ID",
        crate::resource::IDS_COL_USERNAME => "User Name",
        _ => "",
    }
}

pub fn text(key: TextKey) -> &'static str {
    // 新的声明式文本键到英文字符串的映射表。
    match key {
        TextKey::File => "&File",
        TextKey::Options => "&Options",
        TextKey::View => "&View",
        TextKey::Windows => "&Windows",
        TextKey::Help => "&Help",
        TextKey::UpdateSpeed => "&Update Speed",
        TextKey::CpuHistory => "&CPU History",
        TextKey::NewTaskMenu => "&Run...",
        TextKey::NewTaskButton => "&Run...",
        TextKey::ExitTaskManager => "E&xit Task Manager",
        TextKey::AlwaysOnTop => "&Always On Top",
        TextKey::MinimizeOnUse => "&Minimize On Use",
        TextKey::Confirmations => "&Confirmations",
        TextKey::HideWhenMinimized => "&Hide When Minimized",
        TextKey::RefreshNow => "&Refresh Now",
        TextKey::High => "&High",
        TextKey::Normal => "&Normal",
        TextKey::Low => "&Low",
        TextKey::Paused => "&Paused",
        TextKey::LargeIcons => "Lar&ge Icons",
        TextKey::SmallIcons => "S&mall Icons",
        TextKey::Details => "&Details",
        TextKey::TileHorizontally => "Tile &Horizontally",
        TextKey::TileVertically => "Tile &Vertically",
        TextKey::Minimize => "&Minimize",
        TextKey::Maximize => "Ma&ximize",
        TextKey::Cascade => "&Cascade",
        TextKey::BringToFront => "&Bring to Front",
        TextKey::HelpTopics => "Task Manager &Help Topics",
        TextKey::AboutTaskManager => "&About Task Manager",
        TextKey::OneGraphAllCpus => "One Graph, &All CPUs",
        TextKey::OneGraphPerCpu => "One Graph &Per CPU",
        TextKey::SelectColumnsMenu => "Select &Columns...",
        TextKey::SelectColumnsTitle => "Select Columns",
        TextKey::SelectProcessColumnsDescription => {
            "Select the columns that will appear on the Process page of the Task Manager."
        }
        TextKey::ShowKernelTimes => "Show &Kernel Times",
        TextKey::RestoreTaskManager => "&Restore Task Manager",
        TextKey::EndProcess => "&End Process",
        TextKey::EndProcessTree => "End Process &Tree",
        TextKey::OpenFileLocation => "Open File &Location",
        TextKey::Debug => "&Debug",
        TextKey::SetPriority => "Set &Priority",
        TextKey::Realtime => "&Realtime",
        TextKey::AboveNormal => "&Above Normal",
        TextKey::BelowNormal => "&Below Normal",
        TextKey::SetAffinity => "Set &Affinity...",
        TextKey::SwitchTo => "&Switch To",
        TextKey::EndTask => "&End Task",
        TextKey::GoToProcess => "&Go To Process",
        TextKey::Disconnect => "&Disconnect",
        TextKey::Logoff => "&Logoff",
        TextKey::SendMessage => "&Send Message...",
        TextKey::SendMessageTitle => "Send Message",
        TextKey::TaskManager => "Task Manager",
        TextKey::Handles => "Handles",
        TextKey::Threads => "Threads",
        TextKey::ProcessesLabel => "Processes",
        TextKey::CpuUsageHistory => "CPU Usage History",
        TextKey::CpuUsage => "CPU Usage",
        TextKey::MemUsage => "MEM Usage",
        TextKey::MemoryUsageHistory => "Memory Usage History",
        TextKey::PhysicalMemoryK => "Physical Memory (K)",
        TextKey::CommitChargeK => "Commit Charge (K)",
        TextKey::KernelMemoryK => "Kernel Memory (K)",
        TextKey::Totals => "Totals",
        TextKey::Total => "Total",
        TextKey::Available => "Available",
        TextKey::FileCache => "File Cache",
        TextKey::Paged => "Paged",
        TextKey::Nonpaged => "Nonpaged",
        TextKey::Limit => "Limit",
        TextKey::Peak => "Peak",
        TextKey::NoActiveNetworkAdaptersFound => "No Active Network Adapters Found.",
        TextKey::Ok => "OK",
        TextKey::Cancel => "Cancel",
        TextKey::ImageName => "&Image Name",
        TextKey::PidProcessIdentifier => "PID (Process Identifier)",
        TextKey::UserName => "User Name",
        TextKey::SessionId => "Session ID",
        TextKey::CpuTime => "CPU Time",
        TextKey::MemoryUsage => "Memory Usage",
        TextKey::MemoryUsageDelta => "Memory Usage Delta",
        TextKey::PageFaults => "Page Faults",
        TextKey::PageFaultsDelta => "Page Faults Delta",
        TextKey::VirtualMemorySize => "Virtual Memory Size",
        TextKey::PagedPool => "Paged Pool",
        TextKey::NonPagedPool => "Non-paged Pool",
        TextKey::BasePriority => "Base Priority",
        TextKey::HandleCount => "Handle Count",
        TextKey::ThreadCount => "Thread Count",
        TextKey::ProcessorAffinity => "Processor Affinity",
        TextKey::Processors => "Processors",
        TextKey::ProcessorAffinityDescription => {
            "The Processor Affinity setting controls which CPUs the process will be allowed to execute on."
        }
        TextKey::MessageTitleLabel => "&Message title:",
        TextKey::MessageLabel => "Me&ssage:",
        TextKey::ShowFullAccountName => "&Show Full Account Name",
        TextKey::User => "User",
        TextKey::Status => "Status",
        TextKey::ClientName => "Client Name",
        TextKey::Session => "Session",
        TextKey::Adapter => "Adapter",
        TextKey::NetworkUtilization => "Network Utilization",
        TextKey::LinkSpeed => "Link Speed",
        TextKey::State => "State",
        TextKey::BytesSent => "Bytes Sent",
        TextKey::BytesReceived => "Bytes Received",
        TextKey::BytesTotal => "Bytes Total",
        TextKey::Connected => "Connected",
        TextKey::Disconnected => "Disconnected",
        TextKey::Connecting => "Connecting",
        TextKey::Disconnecting => "Disconnecting",
        TextKey::HardwareMissing => "Hardware Missing",
        TextKey::HardwareDisabled => "Hardware Disabled",
        TextKey::HardwareMalfunction => "Hardware Malfunction",
        TextKey::Unknown => "Unknown",
        TextKey::Active => "Active",
        TextKey::ConnectQuery => "Connect Query",
        TextKey::Shadow => "Shadow",
        TextKey::Idle => "Idle",
        TextKey::Listening => "Listening",
        TextKey::Reset => "Reset",
        TextKey::Down => "Down",
        TextKey::Init => "Init",
        TextKey::Bitness32Suffix => "(32-bit)",
        TextKey::NotResponding => "Not Responding",
        TextKey::Running => "Running",
        TextKey::MessageCouldNotBeSent => "The message could not be sent.",
        TextKey::UnableToOpenFileLocation => "Unable to Open File Location",
        TextKey::KillProcessTreePrompt => {
            "This operation will attempt to terminate this process and any\nprocesses which were directly or indirectly started by it.\n\nForcing processes to terminate in this manner can cause\ndata loss and system instability.\n\nAre you sure you wish to continue?"
        }
        TextKey::KillProcessTreeFailed => "Unable to Completely End the Process Tree",
        TextKey::KillProcessTreeFailedBody => {
            "One or more of the processes in this process tree could not\nbe ended. The operation was not fully successful."
        }
        TextKey::ConfirmLogoffSelectedUsers => {
            "Are you sure you want to logoff the selected user(s)?"
        }
        TextKey::ConfirmDisconnectSelectedUsers => {
            "Are you sure you want to disconnect the selected user(s)?"
        }
        TextKey::SelectedUserCouldNotBeLoggedOff => {
            "The selected user could not be logged off."
        }
        TextKey::SelectedUserCouldNotBeDisconnected => {
            "The selected user could not be disconnected."
        }
        TextKey::Win32ErrorPrefix => "Win32 error:",
    }
}

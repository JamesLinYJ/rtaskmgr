//! 葡萄牙语语言表。

use super::text_key::TextKey;
use crate::resource::*;

pub fn resource(id: u32) -> &'static str {
    // 旧式资源 ID 到葡萄牙语字符串的映射表。
    match id {
        IDS_PERFPAGETITLE => "Desempenho",
        IDS_NETPAGETITLE => "Rede",
        IDS_RUNTITLE => "Executar",
        IDS_RUNTEXT => "Digite o nome de um programa, pasta, documento ou recurso da Internet, e o Windows o abrirá para você.",
        IDS_APPTITLE | IDS_TASKMGR => "Gerenciador de Tarefas do Windows NT",
        IDS_PROCPAGETITLE => "Processos",
        IDS_TASKPAGETITLE => "Aplicativos",
        IDS_USERPAGETITLE => "Usuários",
        IDS_TASKMGRDISABLED => "O Gerenciador de Tarefas foi desativado pelo administrador.",
        IDS_WARNING => "Aviso do Gerenciador de Tarefas",
        IDS_LOW => "Baixa",
        IDS_HIGH => "Alta",
        IDS_REALTIME => "Tempo real",
        IDS_NORMAL => "Normal",
        IDS_UNKNOWN => "Desconhecido",
        IDS_CANTSETAFFINITY => "A operação não pôde ser concluída.\n\n",
        IDS_CANTKILL => "Não foi possível encerrar o processo",
        IDS_CANTDEBUG => "Não foi possível anexar o depurador",
        IDS_CANTCHANGEPRI => "Não foi possível alterar a prioridade",
        IDS_FMTTASKS => "Aplicativos: %d",
        IDS_FMTPROCS => "Processos: %d",
        IDS_FMTCPU => "Uso da CPU: %d%%",
        IDS_FMTMEM => "Uso de memória: %dK / %dK",
        IDS_INVALIDOPTION => "Opção inválida",
        IDS_NOAFFINITYMASK => "O processo deve ter afinidade com pelo menos um processador.",
        IDS_FMTCPUNUM => "CPU %d",
        IDS_TOTALTIME => "CPU total",
        IDS_KERNELTIME => "CPU do kernel",
        IDS_MEMUSAGE => "Uso de memória",
        IDS_COL_IMAGENAME => "Nome da imagem",
        IDS_COL_PID => "PID",
        IDS_COL_CPU => "CPU",
        IDS_COL_CPUTIME => "Tempo de CPU",
        IDS_COL_MEMUSAGE => "Uso de memória",
        IDS_COL_MEMUSAGEDIFF => "Delta de memória",
        IDS_COL_PAGEFAULTS => "Falhas de página",
        IDS_COL_PAGEFAULTSDIFF => "Delta de falhas de página",
        IDS_COL_COMMITCHARGE => "Tamanho da memória virtual",
        IDS_COL_PAGEDPOOL => "Pool paginado",
        IDS_COL_NONPAGEDPOOL => "Pool não paginado",
        IDS_COL_BASEPRIORITY => "Prioridade básica",
        IDS_COL_HANDLECOUNT => "Handles",
        IDS_COL_THREADCOUNT => "Threads",
        IDS_COL_SESSIONID => "ID da sessão",
        IDS_COL_USERNAME => "Nome de usuário",
        IDS_COL_TASKNAME => "Tarefa",
        IDS_COL_TASKSTATUS => "Status",
        IDS_COL_TASKWINSTATION => "Estação Win",
        IDS_COL_TASKDESKTOP => "Área de trabalho",
        IDS_PRICHANGE => {
            "AVISO: Alterar a classe de prioridade deste processo pode causar resultados indesejados, incluindo instabilidade do sistema. Tem certeza de que deseja alterar a classe de prioridade?"
        }
        IDS_KILL => {
            "AVISO: Encerrar um processo pode causar resultados indesejados, incluindo perda de dados e instabilidade do sistema. O processo não terá chance de salvar seu estado ou seus dados antes de ser encerrado. Tem certeza de que deseja encerrar este processo?"
        }
        IDS_DEBUG => {
            "AVISO: Depurar este processo pode resultar em perda de dados. Tem certeza de que deseja anexar o depurador?"
        }
        _ => "",
    }
}

pub fn text(key: TextKey) -> &'static str {
    // 新的声明式文本键到葡萄牙语字符串的映射表。
    match key {
        TextKey::File => "&Arquivo",
        TextKey::Options => "&Opções",
        TextKey::View => "E&xibir",
        TextKey::Windows => "&Janelas",
        TextKey::Help => "A&juda",
        TextKey::UpdateSpeed => "&Velocidade de atualização",
        TextKey::CpuHistory => "&Histórico da CPU",
        TextKey::SelectColumnsMenu => "Selecionar colunas...",
        TextKey::SelectColumnsTitle => "Selecionar colunas",
        TextKey::SelectProcessColumnsDescription => {
            "Selecione as colunas que serao exibidas na guia Processos do Gerenciador de Tarefas."
        }
        TextKey::NewTaskMenu | TextKey::NewTaskButton => "&Executar...",
        TextKey::ExitTaskManager => "Sai&r do Gerenciador de Tarefas",
        TextKey::AlwaysOnTop => "&Sempre visível",
        TextKey::MinimizeOnUse => "&Minimizar ao usar",
        TextKey::Confirmations => "&Confirmações",
        TextKey::HideWhenMinimized => "&Ocultar ao minimizar",
        TextKey::RefreshNow => "&Atualizar agora",
        TextKey::High => "&Alta",
        TextKey::Normal => "&Normal",
        TextKey::Low => "&Baixa",
        TextKey::Paused => "&Pausado",
        TextKey::LargeIcons => "Ícones &grandes",
        TextKey::SmallIcons => "Ícones &pequenos",
        TextKey::Details => "&Detalhes",
        TextKey::TileHorizontally => "Lado a lado &horizontalmente",
        TextKey::TileVertically => "Lado a lado &verticalmente",
        TextKey::Minimize => "&Minimizar",
        TextKey::Maximize => "Ma&ximizar",
        TextKey::Cascade => "&Em cascata",
        TextKey::BringToFront => "Trazer para &frente",
        TextKey::HelpTopics => "Tópicos de &ajuda do Gerenciador de Tarefas",
        TextKey::AboutTaskManager => "&Sobre o Gerenciador de Tarefas",
        TextKey::OneGraphAllCpus => "Um gráfico, &todas as CPUs",
        TextKey::OneGraphPerCpu => "Um gráfico &por CPU",
        TextKey::ShowKernelTimes => "Mostrar tempos do &kernel",
        TextKey::RestoreTaskManager => "&Restaurar o Gerenciador de Tarefas",
        TextKey::SwitchTo => "&Alternar para",
        TextKey::EndTask => "&Finalizar tarefa",
        TextKey::EndProcess => "&Finalizar processo",
        TextKey::EndProcessTree
        | TextKey::OpenFileLocation
        | TextKey::AboveNormal
        | TextKey::BelowNormal => super::en_us::text(key),
        TextKey::Debug => "&Depurar",
        TextKey::SetPriority => "Definir &prioridade",
        TextKey::Realtime => "&Tempo real",
        TextKey::SetAffinity => "Definir a&finidade...",
        TextKey::GoToProcess => "&Ir para o processo",
        TextKey::ShowFullAccountName => "Mostrar nome completo da conta",
        TextKey::SendMessageTitle => "Enviar mensagem",
        TextKey::MessageTitleLabel => "Título da mensagem:",
        TextKey::MessageLabel => "Mensagem:",
        TextKey::Disconnect => "&Desconectar",
        TextKey::Logoff => "&Logoff",
        TextKey::SendMessage => "&Enviar mensagem...",
        TextKey::TaskManager => "Gerenciador de Tarefas",
        TextKey::Handles => "Handles",
        TextKey::Threads => "Threads",
        TextKey::ProcessesLabel => "Processos",
        TextKey::User => "Usuário",
        TextKey::Ok => "OK",
        TextKey::Cancel => "Cancelar",
        TextKey::ImageName => "Nome da imagem",
        TextKey::PidProcessIdentifier => "PID (identificador do processo)",
        TextKey::CpuUsage => "Uso da CPU",
        TextKey::CpuUsageHistory => "Histórico de uso da CPU",
        TextKey::PhysicalMemoryK => "Memória física (K)",
        TextKey::CommitChargeK => "Memória confirmada (K)",
        TextKey::KernelMemoryK => "Memória do kernel (K)",
        TextKey::Totals => "Totais",
        TextKey::Total => "Total",
        TextKey::Available => "Disponível",
        TextKey::FileCache => "Cache de arquivos",
        TextKey::Paged => "Paginado",
        TextKey::Nonpaged => "Não paginado",
        TextKey::Limit => "Limite",
        TextKey::Peak => "Pico",
        TextKey::UserName => "Nome de usuário",
        TextKey::SessionId => "ID da sessão",
        TextKey::CpuTime => "Tempo de CPU",
        TextKey::MemoryUsage => "Uso de memória",
        TextKey::MemoryUsageDelta => "Delta de uso de memória",
        TextKey::PageFaults => "Falhas de página",
        TextKey::PageFaultsDelta => "Delta de falhas de página",
        TextKey::VirtualMemorySize => "Tamanho da memória virtual",
        TextKey::PagedPool => "Pool paginado",
        TextKey::NonPagedPool => "Pool não paginado",
        TextKey::BasePriority => "Prioridade básica",
        TextKey::HandleCount => "Contagem de handles",
        TextKey::ThreadCount => "Contagem de threads",
        TextKey::ProcessorAffinity => "Afinidade do processador",
        TextKey::Processors => "Processadores",
        TextKey::ClientName => "Nome do cliente",
        TextKey::Session => "Sessão",
        TextKey::Status => "Status",
        TextKey::Bitness32Suffix => "(32 bits)",
        TextKey::NotResponding => "Nao esta respondendo",
        TextKey::Running => "Em execucao",
        TextKey::MessageCouldNotBeSent => "Nao foi possivel enviar a mensagem.",
        TextKey::UnableToOpenFileLocation
        | TextKey::KillProcessTreePrompt
        | TextKey::KillProcessTreeFailed
        | TextKey::KillProcessTreeFailedBody => super::en_us::text(key),
        TextKey::ConfirmLogoffSelectedUsers => {
            "Tem certeza de que deseja fazer logoff dos usuarios selecionados?"
        }
        TextKey::ConfirmDisconnectSelectedUsers => {
            "Tem certeza de que deseja desconectar os usuarios selecionados?"
        }
        TextKey::SelectedUserCouldNotBeLoggedOff => {
            "Nao foi possivel fazer logoff do usuario selecionado."
        }
        TextKey::SelectedUserCouldNotBeDisconnected => {
            "Nao foi possivel desconectar o usuario selecionado."
        }
        TextKey::Win32ErrorPrefix => "Erro do Win32:",
        TextKey::ProcessorAffinityDescription => {
            "A configuracao de afinidade do processador controla em quais CPUs o processo pode ser executado."
        }
        TextKey::MemUsage => "Uso de memória",
        TextKey::MemoryUsageHistory => "Histórico de uso de memória",
        TextKey::NoActiveNetworkAdaptersFound => "Nenhum adaptador de rede ativo encontrado.",
        TextKey::Adapter => "Adaptador",
        TextKey::NetworkUtilization => "Utilização da rede",
        TextKey::LinkSpeed => "Velocidade do link",
        TextKey::State => "Status",
        TextKey::BytesSent => "Bytes enviados",
        TextKey::BytesReceived => "Bytes recebidos",
        TextKey::BytesTotal => "Total de bytes",
        TextKey::Connected => "Conectado",
        TextKey::Disconnected => "Desconectado",
        TextKey::Connecting => "Conectando",
        TextKey::Disconnecting => "Desconectando",
        TextKey::HardwareMissing => "Hardware ausente",
        TextKey::HardwareDisabled => "Hardware desabilitado",
        TextKey::HardwareMalfunction => "Falha de hardware",
        TextKey::Unknown => "Desconhecido",
        TextKey::Active => "Ativo",
        TextKey::ConnectQuery => "Consulta de conexão",
        TextKey::Shadow => "Sombra",
        TextKey::Idle => "Inativo",
        TextKey::Listening => "Escutando",
        TextKey::Reset => "Redefinir",
        TextKey::Down => "Inativo",
        TextKey::Init => "Inicializando",
    }
}

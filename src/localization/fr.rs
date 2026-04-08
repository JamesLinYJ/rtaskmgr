//! 法语语言表。

use super::text_key::TextKey;
use crate::resource::*;

pub fn resource(id: u32) -> &'static str {
    // 旧式资源 ID 到法语字符串的映射表。
    match id {
        IDS_PERFPAGETITLE => "Performances",
        IDS_NETPAGETITLE => "Réseau",
        IDS_RUNTITLE => "Exécuter",
        IDS_RUNTEXT => "Tapez le nom d’un programme, dossier, document ou d’une ressource Internet, et Windows l’ouvrira pour vous.",
        IDS_APPTITLE | IDS_TASKMGR => "Gestionnaire des tâches Windows NT",
        IDS_PROCPAGETITLE => "Processus",
        IDS_TASKPAGETITLE => "Applications",
        IDS_USERPAGETITLE => "Utilisateurs",
        IDS_TASKMGRDISABLED => "Le Gestionnaire des tâches a été désactivé par votre administrateur.",
        IDS_WARNING => "Avertissement du Gestionnaire des tâches",
        IDS_LOW => "Faible",
        IDS_HIGH => "Élevée",
        IDS_REALTIME => "Temps réel",
        IDS_NORMAL => "Normale",
        IDS_UNKNOWN => "Inconnu",
        IDS_CANTSETAFFINITY => "L’opération n’a pas pu être terminée.\n\n",
        IDS_CANTKILL => "Impossible de terminer le processus",
        IDS_CANTDEBUG => "Impossible d’attacher le débogueur",
        IDS_CANTCHANGEPRI => "Impossible de modifier la priorité",
        IDS_FMTTASKS => "Applications : %d",
        IDS_FMTPROCS => "Processus : %d",
        IDS_FMTCPU => "Utilisation CPU : %d%%",
        IDS_FMTMEM => "Utilisation mémoire : %dK / %dK",
        IDS_INVALIDOPTION => "Option non valide",
        IDS_NOAFFINITYMASK => "Le processus doit avoir une affinité avec au moins un processeur.",
        IDS_FMTCPUNUM => "CPU %d",
        IDS_TOTALTIME => "CPU total",
        IDS_KERNELTIME => "CPU noyau",
        IDS_MEMUSAGE => "Utilisation mémoire",
        IDS_COL_IMAGENAME => "Nom de l'image",
        IDS_COL_PID => "PID",
        IDS_COL_CPU => "CPU",
        IDS_COL_CPUTIME => "Temps CPU",
        IDS_COL_MEMUSAGE => "Utilisation mémoire",
        IDS_COL_MEMUSAGEDIFF => "Delta mémoire",
        IDS_COL_PAGEFAULTS => "Défauts de page",
        IDS_COL_PAGEFAULTSDIFF => "Delta défauts de page",
        IDS_COL_COMMITCHARGE => "Taille mémoire virtuelle",
        IDS_COL_PAGEDPOOL => "Pool paginé",
        IDS_COL_NONPAGEDPOOL => "Pool non paginé",
        IDS_COL_BASEPRIORITY => "Priorité de base",
        IDS_COL_HANDLECOUNT => "Handles",
        IDS_COL_THREADCOUNT => "Threads",
        IDS_COL_SESSIONID => "ID de session",
        IDS_COL_USERNAME => "Nom d'utilisateur",
        IDS_COL_TASKNAME => "Tâche",
        IDS_COL_TASKSTATUS => "État",
        IDS_COL_TASKWINSTATION => "Station Win",
        IDS_COL_TASKDESKTOP => "Bureau",
        IDS_PRICHANGE => {
            "AVERTISSEMENT : la modification de la classe de priorité de ce processus peut entraîner des résultats indésirables, y compris une instabilité du système. Voulez-vous vraiment modifier la classe de priorité ?"
        }
        IDS_KILL => {
            "AVERTISSEMENT : la fin d'un processus peut entraîner des résultats indésirables, notamment une perte de données et une instabilité du système. Le processus n'aura pas la possibilité d'enregistrer son état ou ses données avant d'être terminé. Voulez-vous vraiment terminer ce processus ?"
        }
        IDS_DEBUG => {
            "AVERTISSEMENT : le débogage de ce processus peut entraîner une perte de données. Voulez-vous vraiment attacher le débogueur ?"
        }
        _ => "",
    }
}

pub fn text(key: TextKey) -> &'static str {
    // 新的声明式文本键到法语字符串的映射表。
    match key {
        TextKey::File => "&Fichier",
        TextKey::Options => "&Options",
        TextKey::View => "&Affichage",
        TextKey::Windows => "&Fenêtres",
        TextKey::Help => "&Aide",
        TextKey::UpdateSpeed => "&Vitesse de mise à jour",
        TextKey::CpuHistory => "&Historique CPU",
        TextKey::SelectColumnsMenu => "Choisir les colonnes...",
        TextKey::SelectColumnsTitle => "Choisir les colonnes",
        TextKey::SelectProcessColumnsDescription => {
            "Sélectionnez les colonnes à afficher dans l'onglet Processus du Gestionnaire des tâches."
        }
        TextKey::NewTaskMenu | TextKey::NewTaskButton => "&Exécuter...",
        TextKey::ExitTaskManager => "&Quitter le Gestionnaire des tâches",
        TextKey::AlwaysOnTop => "&Toujours visible",
        TextKey::MinimizeOnUse => "&Réduire après utilisation",
        TextKey::Confirmations => "&Confirmations",
        TextKey::HideWhenMinimized => "&Masquer lors de la réduction",
        TextKey::RefreshNow => "&Actualiser",
        TextKey::High => "&Élevée",
        TextKey::Normal => "&Normale",
        TextKey::Low => "&Faible",
        TextKey::Paused => "&Suspendu",
        TextKey::LargeIcons => "&Grandes icônes",
        TextKey::SmallIcons => "&Petites icônes",
        TextKey::Details => "&Détails",
        TextKey::TileHorizontally => "Mosaïque &horizontale",
        TextKey::TileVertically => "Mosaïque &verticale",
        TextKey::Minimize => "&Réduire",
        TextKey::Maximize => "Ma&ximiser",
        TextKey::Cascade => "&Cascade",
        TextKey::BringToFront => "&Mettre au premier plan",
        TextKey::HelpTopics => "&Rubriques d'aide du Gestionnaire des tâches",
        TextKey::AboutTaskManager => "&À propos du Gestionnaire des tâches",
        TextKey::OneGraphAllCpus => "Un graphique, &tous les CPU",
        TextKey::OneGraphPerCpu => "Un graphique &par CPU",
        TextKey::ShowKernelTimes => "Afficher les temps &noyau",
        TextKey::RestoreTaskManager => "&Restaurer le Gestionnaire des tâches",
        TextKey::SwitchTo => "&Basculer vers",
        TextKey::EndTask => "&Fin de tâche",
        TextKey::EndProcess => "&Terminer le processus",
        TextKey::EndProcessTree
        | TextKey::OpenFileLocation
        | TextKey::AboveNormal
        | TextKey::BelowNormal => super::en_us::text(key),
        TextKey::Debug => "&Déboguer",
        TextKey::SetPriority => "Définir la &priorité",
        TextKey::Realtime => "&Temps réel",
        TextKey::SetAffinity => "Définir l'&affinité...",
        TextKey::GoToProcess => "&Aller au processus",
        TextKey::ShowFullAccountName => "Afficher le nom complet du compte",
        TextKey::SendMessageTitle => "Envoyer un message",
        TextKey::MessageTitleLabel => "Titre du message :",
        TextKey::MessageLabel => "Message :",
        TextKey::Disconnect => "&Déconnecter",
        TextKey::Logoff => "&Fermer la session",
        TextKey::SendMessage => "&Envoyer un message...",
        TextKey::TaskManager => "Gestionnaire des tâches",
        TextKey::Handles => "Handles",
        TextKey::Threads => "Threads",
        TextKey::ProcessesLabel => "Processus",
        TextKey::User => "Utilisateur",
        TextKey::Ok => "OK",
        TextKey::Cancel => "Annuler",
        TextKey::ImageName => "Nom de l'image",
        TextKey::PidProcessIdentifier => "PID (identificateur de processus)",
        TextKey::CpuUsage => "Utilisation CPU",
        TextKey::CpuUsageHistory => "Historique d'utilisation CPU",
        TextKey::PhysicalMemoryK => "Mémoire physique (K)",
        TextKey::CommitChargeK => "Mémoire validée (K)",
        TextKey::KernelMemoryK => "Mémoire noyau (K)",
        TextKey::Totals => "Totaux",
        TextKey::Total => "Total",
        TextKey::Available => "Disponible",
        TextKey::FileCache => "Cache fichiers",
        TextKey::Paged => "Paginé",
        TextKey::Nonpaged => "Non paginé",
        TextKey::Limit => "Limite",
        TextKey::Peak => "Pic",
        TextKey::UserName => "Nom d'utilisateur",
        TextKey::SessionId => "ID de session",
        TextKey::CpuTime => "Temps CPU",
        TextKey::MemoryUsage => "Utilisation mémoire",
        TextKey::MemoryUsageDelta => "Delta d'utilisation mémoire",
        TextKey::PageFaults => "Défauts de page",
        TextKey::PageFaultsDelta => "Delta des défauts de page",
        TextKey::VirtualMemorySize => "Taille de la mémoire virtuelle",
        TextKey::PagedPool => "Pool paginé",
        TextKey::NonPagedPool => "Pool non paginé",
        TextKey::BasePriority => "Priorité de base",
        TextKey::HandleCount => "Nombre de handles",
        TextKey::ThreadCount => "Nombre de threads",
        TextKey::ProcessorAffinity => "Affinité du processeur",
        TextKey::Processors => "Processeurs",
        TextKey::ClientName => "Nom du client",
        TextKey::Session => "Session",
        TextKey::Status => "État",
        TextKey::Bitness32Suffix => "(32 bits)",
        TextKey::NotResponding => "Ne repond pas",
        TextKey::Running => "En cours d'execution",
        TextKey::MessageCouldNotBeSent => "Le message n'a pas pu etre envoye.",
        TextKey::UnableToOpenFileLocation
        | TextKey::KillProcessTreePrompt
        | TextKey::KillProcessTreeFailed
        | TextKey::KillProcessTreeFailedBody => super::en_us::text(key),
        TextKey::ConfirmLogoffSelectedUsers => {
            "Voulez-vous vraiment fermer la session des utilisateurs selectionnes ?"
        }
        TextKey::ConfirmDisconnectSelectedUsers => {
            "Voulez-vous vraiment deconnecter les utilisateurs selectionnes ?"
        }
        TextKey::SelectedUserCouldNotBeLoggedOff => {
            "Impossible de fermer la session de l'utilisateur selectionne."
        }
        TextKey::SelectedUserCouldNotBeDisconnected => {
            "Impossible de deconnecter l'utilisateur selectionne."
        }
        TextKey::Win32ErrorPrefix => "Erreur Win32 :",
        TextKey::ProcessorAffinityDescription => {
            "Le parametre d'affinite du processeur determine sur quels CPU le processus peut s'executer."
        }
        TextKey::MemUsage => "Utilisation mémoire",
        TextKey::MemoryUsageHistory => "Historique d'utilisation mémoire",
        TextKey::NoActiveNetworkAdaptersFound => "Aucun adaptateur réseau actif trouvé.",
        TextKey::Adapter => "Adaptateur",
        TextKey::NetworkUtilization => "Utilisation réseau",
        TextKey::LinkSpeed => "Vitesse du lien",
        TextKey::State => "État",
        TextKey::BytesSent => "Octets envoyés",
        TextKey::BytesReceived => "Octets reçus",
        TextKey::BytesTotal => "Octets au total",
        TextKey::Connected => "Connecté",
        TextKey::Disconnected => "Déconnecté",
        TextKey::Connecting => "Connexion",
        TextKey::Disconnecting => "Déconnexion",
        TextKey::HardwareMissing => "Matériel manquant",
        TextKey::HardwareDisabled => "Matériel désactivé",
        TextKey::HardwareMalfunction => "Dysfonctionnement matériel",
        TextKey::Unknown => "Inconnu",
        TextKey::Active => "Actif",
        TextKey::ConnectQuery => "Interrogation de connexion",
        TextKey::Shadow => "Shadow",
        TextKey::Idle => "Inactif",
        TextKey::Listening => "En écoute",
        TextKey::Reset => "Réinitialiser",
        TextKey::Down => "Hors service",
        TextKey::Init => "Initialisation",
    }
}

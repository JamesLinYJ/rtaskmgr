use crate::localization::{text, TextKey};

pub fn user_column_titles() -> [&'static str; 4] {
    [
        text(TextKey::User),
        text(TextKey::SessionId),
        text(TextKey::Status),
        text(TextKey::ClientName),
    ]
}

pub fn user_session_column_title() -> &'static str {
    text(TextKey::Session)
}

pub fn network_column_titles() -> [&'static str; 7] {
    [
        text(TextKey::Adapter),
        text(TextKey::NetworkUtilization),
        text(TextKey::LinkSpeed),
        text(TextKey::State),
        text(TextKey::BytesSent),
        text(TextKey::BytesReceived),
        text(TextKey::BytesTotal),
    ]
}

pub fn network_graph_labels() -> [&'static str; 3] {
    [text(TextKey::Total), text(TextKey::Recv), text(TextKey::Sent)]
}

pub fn adapter_state(key: &'static str) -> &'static str {
    match key {
        "Connected" => text(TextKey::Connected),
        "Disconnected" => text(TextKey::Disconnected),
        "Connecting" => text(TextKey::Connecting),
        "Disconnecting" => text(TextKey::Disconnecting),
        "Hardware Missing" => text(TextKey::HardwareMissing),
        "Hardware Disabled" => text(TextKey::HardwareDisabled),
        "Hardware Malfunction" => text(TextKey::HardwareMalfunction),
        _ => text(TextKey::Unknown),
    }
}

pub fn session_state(key: &'static str) -> &'static str {
    match key {
        "Active" => text(TextKey::Active),
        "Connected" => text(TextKey::Connected),
        "Connect Query" => text(TextKey::ConnectQuery),
        "Shadow" => text(TextKey::Shadow),
        "Disconnected" => text(TextKey::Disconnected),
        "Idle" => text(TextKey::Idle),
        "Listening" => text(TextKey::Listening),
        "Reset" => text(TextKey::Reset),
        "Down" => text(TextKey::Down),
        "Init" => text(TextKey::Init),
        _ => text(TextKey::Unknown),
    }
}

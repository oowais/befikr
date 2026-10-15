const COMMANDS: &[&str] = &[
    "schedule",
    "cancel",
    "status",
    "request_notifications",
    "open_exact_alarm_settings",
    "open_battery_settings",
    "take_launch_route",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}

const COMMANDS: &[&str] = &[
    "schedule",
    "cancel",
    "status",
    "request_notifications",
    "open_exact_alarm_settings",
    "open_battery_settings",
    "take_launch_route",
    "configure",
    "backup_status",
    "pick_backup_folder",
    "clear_backup_folder",
    "backup_now",
    "export_backup",
    "pick_import_file",
    // addPluginListener (launch route from a notification while the app is open)
    "register_listener",
    "remove_listener",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}

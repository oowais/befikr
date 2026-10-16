# JNI entry points are looked up by name from Rust symbols.
-keep class com.befikr.reminders.Native { *; }
# WorkManager creates workers by class name.
-keep class com.befikr.reminders.SyncWorker { <init>(...); }

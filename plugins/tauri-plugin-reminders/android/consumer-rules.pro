# JNI entry points are looked up by name from Rust symbols.
-keep class com.befikr.reminders.Native { *; }
# WorkManager creates workers by class name.
-keep class com.befikr.reminders.SyncWorker { <init>(...); }
# Room (inside WorkManager) instantiates WorkDatabase_Impl by reflection; R8 full
# mode stripped its no-arg constructor and every process start crashed (0.17.1).
-keep class * extends androidx.room.RoomDatabase { <init>(); }
-keep class androidx.work.impl.WorkDatabase_Impl { *; }

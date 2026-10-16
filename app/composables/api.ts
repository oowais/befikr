// Typed wrappers around the Tauri commands in src-tauri/src/lib.rs.
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

export type Task = {
  id: string
  parentId: string | null
  title: string
  description: string
  dueAt: number | null
  tz: string | null
  rrule: string | null
  adaptive: boolean
  completedAt: number | null
  snoozedUntil: number | null
  createdAt: number
  updatedAt: number
  deletedAt: number | null
}

export type Urgency = 'red' | 'yellow' | 'green' | 'gray'
export type Row = Task & { subtasksDone: number; subtasksTotal: number; urgency: Urgency }
export type Completion = { id: string; taskId: string; title: string; completedAt: number; dueAtThen: number | null }
export type WeekStats = { start: number; onTime: number; late: number; undated: number }
export type Stats = {
  completed: number
  onTime: number
  late: number
  lateMedianMs: number | null
  snoozes: number
  snoozedTasks: number
  missed: number
  weeks: WeekStats[]
}
export type ShoppingItem = { id: string; name: string; boughtAt: number | null }
export type Staple = { id: string; name: string }
export type ShoppingList = { items: ShoppingItem[]; staples: Staple[] }
export type SyncMode = 'off' | 'lan' | 'relay'
export type Peer = { endpoint: string; device: string; name: string; theirSeq: number; pairedAt: number; lastSyncAt: number | null }
export type SyncStatus = {
  mode: SyncMode
  name: string
  running: boolean
  syncing: boolean
  endpoint: string
  peers: Peer[]
  nearby: { endpoint: string; name: string }[]
  pairing: { code: string; untilMs: number } | null
  lastError: string | null
  addresses: string[]
}
export type DesktopPrefs = { autostart: boolean; startHidden: boolean }
export type Settings = { reremindMinutes: number[]; backupDir: string | null; timeWords: Record<string, string> }

export type NewTask = {
  title: string
  description?: string
  parentId?: string | null
  dueAt?: number | null
  tz?: string | null
  rrule?: string | null
  adaptive?: boolean
}

/** Outer undefined = unchanged, null = clear. */
export type TaskPatch = {
  title?: string
  description?: string
  dueAt?: number | null
  tz?: string | null
  rrule?: string | null
  adaptive?: boolean
}

/** Bumped when a write happens off the main list (e.g. an Undo toast); the list reloads. */
export const listVersion = ref(0)
export const listChanged = () => listVersion.value++

export const deviceTz = () => Intl.DateTimeFormat().resolvedOptions().timeZone

export const api = {
  listOpen: () => invoke<Row[]>('list_open', { tz: deviceTz() }),
  getTask: (id: string) => invoke<Task>('get_task', { id }),
  listSubtasks: (id: string) => invoke<Task[]>('list_subtasks', { id }),
  listCompleted: (limit = 200) => invoke<Completion[]>('list_completed', { limit }),
  stats: (since: number | null, weekStarts: number[]) => invoke<Stats>('stats', { since, weekStarts }),
  addTask: (task: NewTask) => invoke<Task>('add_task', { task }),
  updateTask: (id: string, patch: TaskPatch) => invoke<Task>('update_task', { id, patch }),
  completeTask: (id: string) => invoke<Task>('complete_task', { id }),
  reopenTask: (id: string) => invoke<Task>('reopen_task', { id }),
  skipTask: (id: string) => invoke<Task>('skip_task', { id }),
  snoozeTask: (id: string, until: number) => invoke<Task>('snooze_task', { id, until }),
  deleteTask: (id: string) => invoke<Task>('delete_task', { id }),
  restoreTask: (id: string) => invoke<Task>('restore_task', { id }),
  shoppingList: () => invoke<ShoppingList>('shopping_list'),
  addShoppingItem: (name: string) => invoke<ShoppingItem>('add_shopping_item', { name }),
  setShoppingBought: (id: string, bought: boolean) => invoke<ShoppingItem>('set_shopping_bought', { id, bought }),
  setShoppingDeleted: (id: string, deleted: boolean) => invoke<void>('set_shopping_deleted', { id, deleted }),
  shoppingSuggestions: () => invoke<string[]>('shopping_suggestions'),
  addStaple: (name: string) => invoke<Staple>('add_staple', { name }),
  removeStaple: (id: string) => invoke<void>('remove_staple', { id }),
  settings: () => invoke<Settings>('get_settings'),
  setTimeWord: (word: string, time: string) => invoke<void>('set_time_word', { word, time }),
  removeTimeWord: (word: string) => invoke<void>('remove_time_word', { word }),
  setReremindMinutes: (minutes: number[]) => invoke<void>('set_reremind_minutes', { minutes }),
  importBackup: (path: string) => invoke<void>('import_backup', { path }),
  syncStatus: () => invoke<SyncStatus>('sync_status'),
  setSyncMode: (mode: SyncMode) => invoke<void>('set_sync_mode', { mode }),
  setDeviceName: (name: string) => invoke<void>('set_device_name', { name }),
  startPairing: () => invoke<string>('start_pairing'),
  stopPairing: () => invoke<void>('stop_pairing'),
  pairWith: (endpoint: string, code: string) => invoke<void>('pair_with', { endpoint, code }),
  pairWithAddress: (address: string, code: string) => invoke<void>('pair_with_address', { address, code }),
  unpair: (endpoint: string) => invoke<void>('unpair', { endpoint }),
  syncNow: () => invoke<void>('sync_now'),
  rescanSync: () => invoke<void>('rescan_sync'),
  /** Desktop only; rejects on Android. */
  desktopPrefs: () => invoke<DesktopPrefs>('desktop_prefs'),
  setAutostart: (on: boolean) => invoke<void>('set_autostart', { on }),
  setStartHidden: (on: boolean) => invoke<void>('set_start_hidden', { on }),
}

export type BackupStatus = { folder: string | null; lastAt: number | null; lastError: string | null }

export type Status = { notifications: boolean; exactAlarms: boolean; batteryUnrestricted: boolean; sdk: number; device: string }

/** Android-only plugin commands; reject on desktop. */
export const reminders = {
  status: () => invoke<Status>('plugin:reminders|status'),
  requestNotifications: () => invoke('plugin:reminders|request_notifications'),
  openExactAlarmSettings: () => invoke('plugin:reminders|open_exact_alarm_settings'),
  openBatterySettings: () => invoke('plugin:reminders|open_battery_settings'),
  takeLaunchRoute: () => invoke<{ route?: string }>('plugin:reminders|take_launch_route'),
  backupStatus: () => invoke<BackupStatus>('plugin:reminders|backup_status'),
  pickBackupFolder: () => invoke<BackupStatus>('plugin:reminders|pick_backup_folder'),
  clearBackupFolder: () => invoke<BackupStatus>('plugin:reminders|clear_backup_folder'),
  backupNow: () => invoke<BackupStatus>('plugin:reminders|backup_now'),
  exportBackup: () => invoke<{ saved: boolean }>('plugin:reminders|export_backup'),
  pickImportFile: () => invoke<{ path?: string }>('plugin:reminders|pick_import_file'),
}

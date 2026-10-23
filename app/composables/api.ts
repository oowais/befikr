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
  /** Extra reminders, minutes before the due time, largest first. */
  remindBefore: number[]
}

/** Calendar: a task, or a later repeat of one (`upcoming`), placed at `at`. */
export type CalendarItem = Task & { at: number; upcoming: boolean }

export type Urgency = 'red' | 'orange' | 'yellow' | 'green' | 'gray'
export type Row = Task & { subtasksDone: number; subtasksTotal: number; urgency: Urgency }
/** Search result: open or done; `lastDoneAt` = latest completion. */
export type SearchHit = Task & { lastDoneAt: number | null }
export type Completion = {
  id: string
  taskId: string
  title: string
  completedAt: number
  dueAtThen: number | null
  /** Marked not done afterwards: shown, counted as missed. */
  notDone: boolean
}
export type CompletedGroup = {
  taskId: string
  title: string
  count: number
  lastAt: number
  dueAtThen: number | null
  repeating: boolean
  retired: boolean
}
export type Carefree = { streak: number; best: number; onTimeTotal: number; onTimeWeek: number }
export type HabitStreak = { current: number; best: number }
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
export type PastTask = { title: string; count: number; lastAdded: number; usualTime: string | null; notes: string; openId: string | null }
export type SyncMode = 'off' | 'relay'
export type Peer = {
  endpoint: string
  device: string
  name: string
  theirSeq: number
  pairedAt: number
  lastSyncAt: number | null
  /** 'phone' | 'computer', '' until that device has told us (older version). */
  kind: string
}
export type SyncStatus = {
  mode: SyncMode
  name: string
  running: boolean
  syncing: boolean
  endpoint: string
  peers: Peer[]
  pairing: { untilMs: number; link: string } | null
  /** Worded for the user. */
  lastError: string | null
  /** The same, raw (network / protocol). */
  lastErrorDetail: string | null
  conflicts: number
}
export type Conflict = {
  id: string
  table: string
  rowId: string
  field: string
  label: string
  lostValue: unknown
  wonValue: unknown
  wonBy: string | null
  at: number
}
export type DesktopPrefs = { autostart: boolean; startHidden: boolean }
export type Settings = {
  /** Re-remind this long after a swipe; null = off. */
  reremindMinutes: number | null
  backupDir: string | null
  timeWords: Record<string, string>
  /** Morning digest 'HH:MM'; null = off. */
  digestTime: string | null
}

export type NewTask = {
  title: string
  description?: string
  parentId?: string | null
  dueAt?: number | null
  tz?: string | null
  rrule?: string | null
  adaptive?: boolean
  remindBefore?: number[]
}

/** Outer undefined = unchanged, null = clear. */
export type TaskPatch = {
  title?: string
  description?: string
  dueAt?: number | null
  tz?: string | null
  rrule?: string | null
  adaptive?: boolean
  remindBefore?: number[]
}

/** Bumped when a write happens off the main list (e.g. an Undo toast); the list reloads. */
export const listVersion = ref(0)
export const listChanged = () => listVersion.value++

export const deviceTz = () => Intl.DateTimeFormat().resolvedOptions().timeZone

export const api = {
  listOpen: () => invoke<Row[]>('list_open', { tz: deviceTz() }),
  search: (query: string) => invoke<SearchHit[]>('search', { query }),
  calendar: (from: number, to: number) => invoke<CalendarItem[]>('calendar', { from, to, tz: deviceTz() }),
  /** Desktop: save open dated tasks as an .ics file (save dialog). */
  exportIcs: () => invoke<{ saved: boolean }>('export_ics', { tz: deviceTz() }),
  getTask: (id: string) => invoke<Task>('get_task', { id }),
  listSubtasks: (id: string) => invoke<Task[]>('list_subtasks', { id }),
  listCompleted: (limit = 200) => invoke<Completion[]>('list_completed', { limit }),
  completedGroups: () => invoke<CompletedGroup[]>('completed_groups'),
  taskHistory: (id: string) => invoke<Completion[]>('task_history', { id }),
  setCompletionDone: (id: string, done: boolean) => invoke<void>('set_completion_done', { id, done }),
  habitStreak: (id: string) => invoke<HabitStreak | null>('habit_streak', { id, tz: deviceTz() }),
  carefree: () => invoke<Carefree>('carefree', { tz: deviceTz() }),
  stats: (since: number | null, weekStarts: number[]) => invoke<Stats>('stats', { since, weekStarts }),
  addTask: (task: NewTask) => invoke<Task>('add_task', { task }),
  updateTask: (id: string, patch: TaskPatch) => invoke<Task>('update_task', { id, patch }),
  completeTask: (id: string) => invoke<Task>('complete_task', { id }),
  reopenTask: (id: string) => invoke<Task>('reopen_task', { id }),
  skipTask: (id: string) => invoke<Task>('skip_task', { id }),
  retireTask: (id: string) => invoke<Task>('retire_task', { id }),
  snoozePicks: () => invoke<{ label: string; atMs: number }[]>('snooze_picks', { tz: deviceTz() }),
  snoozeTask: (id: string, until: number) => invoke<Task>('snooze_task', { id, until }),
  deleteTask: (id: string) => invoke<Task>('delete_task', { id }),
  restoreTask: (id: string) => invoke<Task>('restore_task', { id }),
  shoppingList: () => invoke<ShoppingList>('shopping_list'),
  addShoppingItem: (name: string) => invoke<ShoppingItem>('add_shopping_item', { name }),
  setShoppingBought: (id: string, bought: boolean) => invoke<ShoppingItem>('set_shopping_bought', { id, bought }),
  setShoppingDeleted: (id: string, deleted: boolean) => invoke<void>('set_shopping_deleted', { id, deleted }),
  shoppingSuggestions: () => invoke<string[]>('shopping_suggestions'),
  pastTasks: () => invoke<PastTask[]>('past_tasks'),
  addStaple: (name: string) => invoke<Staple>('add_staple', { name }),
  removeStaple: (id: string) => invoke<void>('remove_staple', { id }),
  settings: () => invoke<Settings>('get_settings'),
  setTimeWord: (word: string, time: string) => invoke<void>('set_time_word', { word, time }),
  removeTimeWord: (word: string) => invoke<void>('remove_time_word', { word }),
  setReremindMinutes: (minutes: number | null) => invoke<void>('set_reremind_minutes', { minutes }),
  setDigestTime: (time: string | null) => invoke<void>('set_digest_time', { time }),
  importBackup: (path: string) => invoke<void>('import_backup', { path }),
  syncStatus: () => invoke<SyncStatus>('sync_status'),
  setSyncMode: (mode: SyncMode) => invoke<void>('set_sync_mode', { mode }),
  setDeviceName: (name: string) => invoke<void>('set_device_name', { name }),
  startPairing: () => invoke<void>('start_pairing'),
  stopPairing: () => invoke<void>('stop_pairing'),
  pairWithLink: (link: string) => invoke<void>('pair_with_link', { link }),
  unpair: (endpoint: string) => invoke<void>('unpair', { endpoint }),
  syncNow: () => invoke<void>('sync_now'),
  conflicts: () => invoke<Conflict[]>('conflicts'),
  resolveConflict: (id: string, useMine: boolean) => invoke<void>('resolve_conflict', { id, useMine }),
  /** Desktop only; rejects on Android. */
  desktopPrefs: () => invoke<DesktopPrefs>('desktop_prefs'),
  /** Windows: a switch that silences every reminder ('windows' = all notifications off, 'app' = Befikr's). */
  notificationsBlocked: () => invoke<'windows' | 'app' | null>('notifications_blocked').catch(() => null),
  openNotificationSettings: () => invoke<void>('open_notification_settings'),
  setAutostart: (on: boolean) => invoke<void>('set_autostart', { on }),
  setStartHidden: (on: boolean) => invoke<void>('set_start_hidden', { on }),
}

export type BackupStatus = { folder: string | null; lastAt: number | null; lastError: string | null }

export type Status = { notifications: boolean; exactAlarms: boolean; batteryUnrestricted: boolean; sdk: number; device: string }

export const isAndroid = typeof navigator !== 'undefined' && /Android/i.test(navigator.userAgent)
/** Backup runs in the Kotlin plugin on Android, in the app process on computers. */
const backupCmd = (name: string) => (isAndroid ? `plugin:reminders|${name}` : name)

/** Android-only plugin commands (backup ones work on both); reject on desktop. */
export const reminders = {
  status: () => invoke<Status>('plugin:reminders|status'),
  requestNotifications: () => invoke('plugin:reminders|request_notifications'),
  openExactAlarmSettings: () => invoke('plugin:reminders|open_exact_alarm_settings'),
  openBatterySettings: () => invoke('plugin:reminders|open_battery_settings'),
  takeLaunchRoute: () => invoke<{ route?: string }>('plugin:reminders|take_launch_route'),
  backupStatus: () => invoke<BackupStatus>(backupCmd('backup_status')),
  pickBackupFolder: () => invoke<BackupStatus>(backupCmd('pick_backup_folder')),
  clearBackupFolder: () => invoke<BackupStatus>(backupCmd('clear_backup_folder')),
  backupNow: () => invoke<BackupStatus>(backupCmd('backup_now')),
  exportBackup: () => invoke<{ saved: boolean }>(backupCmd('export_backup')),
  pickImportFile: () => invoke<{ path?: string }>(backupCmd('pick_import_file')),
  /** Android: is there an app that takes speech input? (false elsewhere) */
  speechAvailable: () =>
    isAndroid
      ? invoke<{ available: boolean }>('plugin:reminders|speech_available').then((r) => r.available, () => false)
      : Promise.resolve(false),
  /** Android: the phone's speech recogniser; null when cancelled. */
  recognizeSpeech: () => invoke<{ text: string | null }>('plugin:reminders|recognize_speech').then((r) => r.text),
}

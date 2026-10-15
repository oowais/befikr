// Typed wrappers around the Tauri commands in src-tauri/src/lib.rs.
import { invoke } from '@tauri-apps/api/core'

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

export const deviceTz = () => Intl.DateTimeFormat().resolvedOptions().timeZone

export const api = {
  listOpen: () => invoke<Row[]>('list_open', { tz: deviceTz() }),
  getTask: (id: string) => invoke<Task>('get_task', { id }),
  listSubtasks: (id: string) => invoke<Task[]>('list_subtasks', { id }),
  listCompleted: (limit = 200) => invoke<Completion[]>('list_completed', { limit }),
  addTask: (task: NewTask) => invoke<Task>('add_task', { task }),
  updateTask: (id: string, patch: TaskPatch) => invoke<Task>('update_task', { id, patch }),
  completeTask: (id: string) => invoke<Task>('complete_task', { id }),
  reopenTask: (id: string) => invoke<Task>('reopen_task', { id }),
  snoozeTask: (id: string, until: number) => invoke<Task>('snooze_task', { id, until }),
  deleteTask: (id: string) => invoke<Task>('delete_task', { id }),
  settings: () => invoke<Settings>('get_settings'),
  setTimeWord: (word: string, time: string) => invoke<void>('set_time_word', { word, time }),
  removeTimeWord: (word: string) => invoke<void>('remove_time_word', { word }),
  setReremindMinutes: (minutes: number[]) => invoke<void>('set_reremind_minutes', { minutes }),
}

export type Status = { notifications: boolean; exactAlarms: boolean; batteryUnrestricted: boolean; sdk: number; device: string }

/** Android-only plugin commands; reject on desktop. */
export const reminders = {
  status: () => invoke<Status>('plugin:reminders|status'),
  requestNotifications: () => invoke('plugin:reminders|request_notifications'),
  openExactAlarmSettings: () => invoke('plugin:reminders|open_exact_alarm_settings'),
  openBatterySettings: () => invoke('plugin:reminders|open_battery_settings'),
  takeLaunchRoute: () => invoke<{ route?: string }>('plugin:reminders|take_launch_route'),
}

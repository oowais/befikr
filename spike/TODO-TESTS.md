# Spike — tests still to do

Install the latest APK over the old one (data is kept):
https://github.com/oowais/befikr/releases/download/spike-latest/befikr-spike.apk

Already passed on Pixel 9: on-time firing, re-remind +1/+2 min, Snooze → snooze sheet, Complete from notification with app closed, vibration, swipe-away re-fire, 8 at once.

Check the **Event log** in the app after each test.

## 1. Doze (phone asleep) — most important
Without a computer:
- [ ] Schedule **15 min** (`setExactAndAllowWhileIdle`). Screen off, phone still, not charging. Fires on time?
- [ ] Same with **setAlarmClock** mode. Fires on time? (Shows an alarm icon in the status bar — note if that bothers you.)

With a computer (`adb`), faster:
- [ ] Schedule 5 min, screen off, run `adb shell dumpsys deviceidle force-idle`. Fires on time? Repeat with `setAlarmClock`.

## 2. Two at the same time while asleep
- [ ] Schedule two 15 min reminders back to back, screen off. Do both fire on time, or is one late? Try both modes.

## 3. Reboot
- [ ] Schedule 5 min, restart the phone, don't open the app. Still fires?

## 4. Snooze from cold start
- [ ] Swipe the app away from recents. When a reminder fires, tap **Snooze**. Does the app open straight on the snooze sheet?

## Report back
For each: ✅ / ❌ + anything odd. A screenshot of the event log helps.

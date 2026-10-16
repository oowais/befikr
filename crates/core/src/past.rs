//! Past tasks for suggestions in quick add. Worked out from `tasks`, nothing
//! stored. Only titles worth suggesting come back: added twice or more, added
//! in the last 30 days, or still open (so a duplicate can be avoided).

use std::collections::HashMap;

use chrono::Timelike;

use crate::model::PastTask;
use crate::recurrence::{from_ms, parse_tz};
use crate::store::Store;
use crate::Result;

const DAY_MS: i64 = 86_400_000;
const RECENT_MS: i64 = 30 * DAY_MS;
/// Each use counts half as much after this long.
const HALF_LIFE_MS: f64 = 60.0 * DAY_MS as f64;

struct Group {
    task: PastTask,
    score: f64,
    times: HashMap<String, (u32, i64)>,
}

impl Store {
    /// Past top-level task titles worth suggesting, best first (often and
    /// lately added). Deleted tasks and subtasks don't count.
    pub fn past_tasks(&self, limit: usize) -> Result<Vec<PastTask>> {
        let now = self.now();
        let mut st = self.conn.prepare(
            "SELECT id, title, description, due_at, tz, completed_at, created_at FROM tasks
             WHERE parent_id IS NULL AND deleted_at IS NULL ORDER BY created_at, id",
        )?;
        let mut rows = st.query([])?;
        let mut groups: HashMap<String, Group> = HashMap::new();
        while let Some(r) = rows.next()? {
            let id: String = r.get(0)?;
            let title: String = r.get(1)?;
            let notes: String = r.get(2)?;
            let due_at: Option<i64> = r.get(3)?;
            let tz: Option<String> = r.get(4)?;
            let completed_at: Option<i64> = r.get(5)?;
            let created_at: i64 = r.get(6)?;

            let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
            if title.is_empty() {
                continue;
            }
            let g = groups.entry(title.to_lowercase()).or_insert_with(|| Group {
                task: PastTask {
                    title: String::new(),
                    count: 0,
                    last_added: 0,
                    usual_time: None,
                    notes: String::new(),
                    open_id: None,
                },
                score: 0.0,
                times: HashMap::new(),
            });
            // Oldest first, so later rows overwrite.
            g.task.title = title;
            g.task.count += 1;
            g.task.last_added = created_at;
            if !notes.trim().is_empty() {
                g.task.notes = notes;
            }
            if completed_at.is_none() {
                g.task.open_id = Some(id);
            }
            let age = (now - created_at).max(0) as f64;
            g.score += 0.5f64.powf(age / HALF_LIFE_MS);
            if let (Some(due), Some(tz)) = (due_at, tz.as_deref().and_then(|t| parse_tz(t).ok())) {
                let local = from_ms(due, tz);
                let hm = format!("{:02}:{:02}", local.hour(), local.minute());
                let e = g.times.entry(hm).or_insert((0, 0));
                e.0 += 1;
                e.1 = created_at;
            }
        }

        let mut out: Vec<(f64, PastTask)> = groups
            .into_values()
            .filter(|g| g.task.count >= 2 || now - g.task.last_added <= RECENT_MS || g.task.open_id.is_some())
            .map(|mut g| {
                g.task.usual_time = g
                    .times
                    .into_iter()
                    .max_by_key(|(_, (n, last))| (*n, *last))
                    .map(|(hm, _)| hm);
                (g.score, g.task)
            })
            .collect();
        out.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.last_added.cmp(&a.1.last_added)));
        Ok(out.into_iter().take(limit).map(|(_, t)| t).collect())
    }
}

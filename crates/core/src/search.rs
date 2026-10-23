//! Search: tasks whose title, notes or a subtask contain the text, open and done.

use serde::Serialize;

use crate::model::Task;
use crate::store::{row_task, Store, COLS};
use crate::Result;

/// At most this many results.
const LIMIT: u32 = 50;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    #[serde(flatten)]
    pub task: Task,
    /// Latest completion (a one-off's, or a repeat's last occurrence).
    pub last_done_at: Option<i64>,
}

impl Store {
    /// Top-level tasks matching `query` (case-insensitive, every word must
    /// appear somewhere). Open ones first in list order, then done ones,
    /// most recently done first. Deleted ones never.
    pub fn search(&self, query: &str) -> Result<Vec<SearchHit>> {
        let words: Vec<String> = query.split_whitespace().map(like_pattern).collect();
        if words.is_empty() {
            return Ok(vec![]);
        }
        let conds = (1..=words.len())
            .map(|i| {
                format!(
                    "(t.title LIKE ?{i} ESCAPE '\\' OR t.description LIKE ?{i} ESCAPE '\\'
                      OR EXISTS (SELECT 1 FROM tasks s WHERE s.parent_id = t.id AND s.deleted_at IS NULL
                                 AND s.title LIKE ?{i} ESCAPE '\\'))"
                )
            })
            .collect::<Vec<_>>()
            .join(" AND ");
        let cols = COLS.split(", ").map(|c| format!("t.{}", c.trim())).collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT {cols},
               (SELECT MAX(c.completed_at) FROM completions c
                WHERE c.task_id = t.id AND c.deleted_at IS NULL AND c.not_done_at IS NULL) AS last_done
             FROM tasks t
             WHERE t.parent_id IS NULL AND t.deleted_at IS NULL AND {conds}
             ORDER BY t.completed_at IS NOT NULL,
               CASE WHEN t.completed_at IS NULL THEN COALESCE(t.snoozed_until, t.due_at) IS NULL END,
               CASE WHEN t.completed_at IS NULL THEN COALESCE(t.snoozed_until, t.due_at) END,
               COALESCE(last_done, t.completed_at) DESC, t.id
             LIMIT {LIMIT}"
        );
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map(rusqlite::params_from_iter(words.iter()), |r| {
            Ok(SearchHit { task: row_task(r)?, last_done_at: r.get(14)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

/// `%word%`, with LIKE's own wildcards taken literally.
fn like_pattern(word: &str) -> String {
    let mut p = String::from("%");
    for ch in word.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            p.push('\\');
        }
        p.push(ch);
    }
    p.push('%');
    p
}

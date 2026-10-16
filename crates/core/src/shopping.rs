//! Shopping list and staples (chips of things usually bought). Separate from
//! tasks: no dates, no reminders. Same sync-ready rules: UUIDv7 ids,
//! `updated_at` on every write, soft deletes.

use rusqlite::{params, Connection, OptionalExtension};

use crate::model::{ShoppingItem, ShoppingList, Staple};
use crate::store::{new_id, Store};
use crate::{Error, Result};

impl Store {
    /// Items still to buy (oldest first) and staples (by name).
    pub fn shopping_list(&self) -> Result<ShoppingList> {
        let mut st = self.conn.prepare(
            "SELECT id, name, bought_at FROM shopping_items
             WHERE bought_at IS NULL AND deleted_at IS NULL ORDER BY id",
        )?;
        let items = st
            .query_map([], |r| {
                Ok(ShoppingItem {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    bought_at: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut st = self.conn.prepare(
            "SELECT id, name FROM staples WHERE deleted_at IS NULL ORDER BY name COLLATE NOCASE",
        )?;
        let staples = st
            .query_map([], |r| {
                Ok(Staple {
                    id: r.get(0)?,
                    name: r.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(ShoppingList { items, staples })
    }

    /// Add to the list. Already on it (any case) → the existing item.
    pub fn add_shopping_item(&mut self, name: &str) -> Result<ShoppingItem> {
        let name = clean(name)?;
        self.write(|c, now| {
            if let Some(item) = open_item_named(c, &name)? {
                return Ok(item);
            }
            let id = new_id();
            c.execute(
                "INSERT INTO shopping_items (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
                params![id, name, now],
            )?;
            Ok(ShoppingItem { id, name, bought_at: None })
        })
    }

    /// Tick off (`bought`) or put back (undo).
    pub fn set_shopping_bought(&mut self, id: &str, bought: bool) -> Result<ShoppingItem> {
        self.write(|c, now| {
            let at = bought.then_some(now);
            let n = c.execute(
                "UPDATE shopping_items SET bought_at = ?2, updated_at = ?3 WHERE id = ?1 AND deleted_at IS NULL",
                params![id, at, now],
            )?;
            if n == 0 {
                return Err(Error::NotFound(id.to_string()));
            }
            Ok(c.query_row("SELECT id, name, bought_at FROM shopping_items WHERE id = ?1", [id], |r| {
                Ok(ShoppingItem { id: r.get(0)?, name: r.get(1)?, bought_at: r.get(2)? })
            })?)
        })
    }

    /// New chip. Same name (any case) as an existing one → that one.
    pub fn add_staple(&mut self, name: &str) -> Result<Staple> {
        let name = clean(name)?;
        self.write(|c, now| {
            let existing: Option<String> = c
                .query_row(
                    "SELECT id FROM staples WHERE deleted_at IS NULL AND name = ?1 COLLATE NOCASE",
                    [&name],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(id) = existing {
                return Ok(Staple { id, name });
            }
            let id = new_id();
            c.execute(
                "INSERT INTO staples (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
                params![id, name, now],
            )?;
            Ok(Staple { id, name })
        })
    }

    /// Take an item off the list without buying it (`deleted` = false undoes).
    pub fn set_shopping_deleted(&mut self, id: &str, deleted: bool) -> Result<()> {
        self.write(|c, now| {
            let at = deleted.then_some(now);
            let n = c.execute(
                "UPDATE shopping_items SET deleted_at = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, at, now],
            )?;
            if n == 0 {
                return Err(Error::NotFound(id.to_string()));
            }
            Ok(())
        })
    }

    /// Names added to the list at least twice that aren't chips yet, most
    /// often first (latest spelling).
    pub fn shopping_suggestions(&self, limit: u32) -> Result<Vec<String>> {
        let mut st = self.conn.prepare(
            "SELECT (SELECT s.name FROM shopping_items s WHERE lower(s.name) = lower(i.name) ORDER BY s.id DESC LIMIT 1)
             FROM shopping_items i
             WHERE NOT EXISTS (SELECT 1 FROM staples p WHERE p.deleted_at IS NULL AND lower(p.name) = lower(i.name))
             GROUP BY lower(i.name) HAVING count(*) >= 2
             ORDER BY count(*) DESC, max(i.id) DESC LIMIT ?1",
        )?;
        let rows = st.query_map([limit], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn remove_staple(&mut self, id: &str) -> Result<()> {
        self.write(|c, now| {
            c.execute(
                "UPDATE staples SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NULL",
                params![id, now],
            )?;
            Ok(())
        })
    }
}

/// Trimmed, inner whitespace collapsed; empty is an error.
fn clean(name: &str) -> Result<String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err(Error::Invalid("name is empty".into()));
    }
    Ok(name)
}

fn open_item_named(c: &Connection, name: &str) -> Result<Option<ShoppingItem>> {
    Ok(c.query_row(
        "SELECT id, name, bought_at FROM shopping_items
         WHERE bought_at IS NULL AND deleted_at IS NULL AND name = ?1 COLLATE NOCASE",
        [name],
        |r| {
            Ok(ShoppingItem {
                id: r.get(0)?,
                name: r.get(1)?,
                bought_at: r.get(2)?,
            })
        },
    )
    .optional()?)
}

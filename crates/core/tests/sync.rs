use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use befikr_core::*;

const T0: i64 = 1_792_000_000_000;

/// Store with its own clock (devices' clocks differ).
fn device() -> (Store, Arc<AtomicI64>) {
    let clock = Arc::new(AtomicI64::new(T0));
    let c = clock.clone();
    (Store::open_in_memory().unwrap().with_clock(move || c.load(Ordering::SeqCst)), clock)
}

/// Send `from`'s changes after `*seen` to `to`.
fn sync(from: &Store, to: &mut Store, seen: &mut i64) -> MergeOutcome {
    let changes = from.changes_since(*seen, 10_000, None).unwrap();
    if let Some(last) = changes.last() {
        *seen = last.seq;
    }
    to.merge(&changes).unwrap()
}

fn titles(s: &Store) -> Vec<String> {
    s.list_open().unwrap().into_iter().map(|i| i.task.title).collect()
}

#[test]
fn add_and_complete_travel_both_ways() {
    let (mut a, _) = device();
    let (mut b, _) = device();
    let (mut a_seen, mut b_seen) = (0, 0);
    let id = a
        .add_task(NewTask { title: "Pay rent".into(), due_at: Some(T0 + 3_600_000), tz: Some("Europe/Berlin".into()), ..Default::default() })
        .unwrap()
        .task
        .id;

    let out = sync(&a, &mut b, &mut a_seen);
    assert_eq!(titles(&b), ["Pay rent"]);
    assert_eq!(out.alarms, [AlarmChange::Schedule(Alarm { task_id: id.clone(), title: "Pay rent".into(), at_ms: T0 + 3_600_000 })]);

    b.complete_task(&id, Source::App).unwrap();
    let out = sync(&b, &mut a, &mut b_seen);
    assert!(titles(&a).is_empty(), "completed on the other device");
    assert_eq!(a.list_completed(10).unwrap().len(), 1);
    assert_eq!(out.alarms, [AlarmChange::Cancel { task_id: id }]);

    // Echo back: nothing new.
    assert_eq!(sync(&a, &mut b, &mut a_seen).applied, 0);
}

#[test]
fn newest_edit_wins_per_field() {
    let (mut a, ca) = device();
    let (mut b, cb) = device();
    let (mut a_seen, mut b_seen) = (0, 0);
    let id = a.add_task(NewTask { title: "milk".into(), ..Default::default() }).unwrap().task.id;
    sync(&a, &mut b, &mut a_seen);

    // Offline: A renames, then B renames later and edits notes earlier.
    ca.store(T0 + 10_000, Ordering::SeqCst);
    a.update_task(&id, TaskPatch { title: Some("oat milk".into()), ..Default::default() }).unwrap();
    cb.store(T0 + 5_000, Ordering::SeqCst);
    b.update_task(&id, TaskPatch { description: Some("2 cartons".into()), ..Default::default() }).unwrap();
    cb.store(T0 + 20_000, Ordering::SeqCst);
    b.update_task(&id, TaskPatch { title: Some("soy milk".into()), ..Default::default() }).unwrap();

    sync(&a, &mut b, &mut a_seen);
    sync(&b, &mut a, &mut b_seen);
    for s in [&a, &b] {
        let t = s.get_task(&id).unwrap();
        assert_eq!((t.title.as_str(), t.description.as_str()), ("soy milk", "2 cartons"));
    }
}

#[test]
fn device_local_settings_stay_put() {
    let (mut a, _) = device();
    let (mut b, _) = device();
    a.set_reremind_minutes(vec![10, 20]).unwrap();
    a.set_backup_dir(Some("/sdcard/Befikr".into())).unwrap();
    sync(&a, &mut b, &mut 0);
    let s = b.settings().unwrap();
    assert_eq!((s.reremind_minutes, s.backup_dir), (vec![10, 20], None));
}

#[test]
fn rows_in_any_order_and_forwarded() {
    let (mut a, _) = device();
    let (mut b, _) = device();
    let (mut c, _) = device();
    let parent = a.add_task(NewTask { title: "trip".into(), ..Default::default() }).unwrap().task.id;
    a.add_task(NewTask { title: "pack".into(), parent_id: Some(parent.clone()), ..Default::default() }).unwrap();
    a.add_shopping_item("sunscreen").unwrap();

    // Subtask before its parent.
    let mut changes = a.changes_since(0, 10_000, None).unwrap();
    changes.reverse();
    b.merge(&changes).unwrap();
    assert_eq!(b.list_subtasks(&parent).unwrap().len(), 1);

    // B passes A's changes on to C.
    sync(&b, &mut c, &mut 0);
    assert_eq!(titles(&c), ["trip"]);
    assert_eq!(c.shopping_list().unwrap().items[0].name, "sunscreen");
    assert_ne!(a.device_id().unwrap(), b.device_id().unwrap());
}

#[test]
fn peers_and_settings_are_local() {
    let (mut a, _) = device();
    assert_eq!(a.sync_settings().unwrap().mode, SyncMode::Off);
    a.set_sync_mode(SyncMode::Relay).unwrap();
    a.set_device_name("  Pixel   9 ").unwrap();
    a.add_peer("ep1", "dev1", "Desktop").unwrap();
    a.peer_synced("ep1", 42, "Desktop").unwrap();
    a.remove_peer("ep1").unwrap();
    a.add_peer("ep1", "dev1", "Desktop").unwrap();
    assert_eq!(a.peer("ep1").unwrap().unwrap().their_seq, 42, "re-pairing keeps progress");
    a.set_peer_addrs("ep1", &["192.168.1.5:47474".into()]).unwrap();
    assert_eq!(a.peer("ep1").unwrap().unwrap().addrs, ["192.168.1.5:47474"]);

    let (mut b, _) = device();
    sync(&a, &mut b, &mut 0);
    assert!(b.peers().unwrap().is_empty());
    let s = a.sync_settings().unwrap();
    assert_eq!((s.mode, s.name.as_str()), (SyncMode::Relay, "Pixel 9"));
    assert_eq!(b.sync_settings().unwrap().mode, SyncMode::Off);
}

#[test]
fn restoring_a_backup_resyncs_with_peers() {
    let (mut a, ca) = device();
    let (mut b, cb) = device();
    let (mut a_seen, mut b_seen) = (0, 0);
    a.add_peer("ep-b", &b.device_id().unwrap(), "B").unwrap();
    let id = a.add_task(NewTask { title: "milk".into(), ..Default::default() }).unwrap().task.id;
    a.add_task(NewTask { title: "eggs".into(), ..Default::default() }).unwrap();
    sync(&a, &mut b, &mut a_seen);

    let dir = std::env::temp_dir().join(format!("befikr-test-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let backup = dir.join("a.db");
    a.export_to(backup.to_str().unwrap()).unwrap();

    // After the backup: B renames, A gets it and makes more changes of its own.
    cb.store(T0 + 60_000, Ordering::SeqCst);
    b.update_task(&id, TaskPatch { title: Some("oat milk".into()), ..Default::default() }).unwrap();
    sync(&b, &mut a, &mut b_seen);
    a.peer_synced("ep-b", b_seen, "B").unwrap();
    ca.store(T0 + 120_000, Ordering::SeqCst);
    for t in ["bread", "jam"] {
        a.add_task(NewTask { title: t.into(), ..Default::default() }).unwrap();
    }
    sync(&a, &mut b, &mut a_seen);
    let seq_before = a.last_seq().unwrap();

    a.import_from(backup.to_str().unwrap()).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    assert!(titles(&a).contains(&"milk".to_string()), "back to the old data");

    // A asks B for everything again (from 0, A's own changes included), so
    // B's rename and A's lost edits come back...
    assert_eq!(a.peer("ep-b").unwrap().unwrap().their_seq, 0);
    sync(&b, &mut a, &mut 0);
    let now = titles(&a);
    for t in ["oat milk", "bread", "jam"] {
        assert!(now.contains(&t.to_string()), "{t} missing: {now:?}");
    }
    // ...and A's own edits after the restore get seqs B hasn't seen yet.
    ca.store(T0 + 180_000, Ordering::SeqCst);
    a.add_task(NewTask { title: "after restore".into(), ..Default::default() }).unwrap();
    assert!(a.last_seq().unwrap() > seq_before);
    sync(&a, &mut b, &mut a_seen);
    assert!(titles(&b).contains(&"after restore".to_string()));
}

/// A and B both have "milk"; returns its id.
fn shared_task(a: &mut Store, b: &mut Store) -> String {
    let id = a.add_task(NewTask { title: "milk".into(), ..Default::default() }).unwrap().task.id;
    sync(a, b, &mut 0);
    id
}

fn rename(s: &mut Store, id: &str, title: &str) {
    s.update_task(id, TaskPatch { title: Some(title.into()), ..Default::default() }).unwrap();
}

#[test]
fn concurrent_edit_is_a_conflict_on_the_losing_side() {
    let (mut a, ca) = device();
    let (mut b, cb) = device();
    a.add_peer("ep-b", &b.device_id().unwrap(), "Pixel 7").unwrap();
    let id = shared_task(&mut a, &mut b);

    // Both rename before syncing; B's is later, so B wins.
    ca.store(T0 + 1_000, Ordering::SeqCst);
    rename(&mut a, &id, "oat milk");
    cb.store(T0 + 2_000, Ordering::SeqCst);
    rename(&mut b, &id, "soy milk");
    sync(&b, &mut a, &mut 0);
    sync(&a, &mut b, &mut 0);
    assert_eq!(a.get_task(&id).unwrap().title, "soy milk");
    assert_eq!(b.get_task(&id).unwrap().title, "soy milk");

    let c = a.conflicts().unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].field.as_str(), c[0].label.as_str()), ("title", "soy milk"));
    assert_eq!((c[0].lost_value.as_str(), c[0].won_value.as_str()), (Some("oat milk"), Some("soy milk")));
    assert_eq!(c[0].won_by.as_deref(), Some("Pixel 7"));
    assert!(b.conflicts().unwrap().is_empty(), "B's edit won: nothing lost there");

    // Use mine: A's title comes back and wins on B too, without a new conflict.
    ca.store(T0 + 3_000, Ordering::SeqCst);
    a.resolve_conflict(&c[0].id, true).unwrap();
    assert!(a.conflicts().unwrap().is_empty());
    sync(&a, &mut b, &mut 0);
    assert_eq!(b.get_task(&id).unwrap().title, "oat milk");
    assert!(b.conflicts().unwrap().is_empty(), "B had seen its own version being replaced");
}

#[test]
fn overwriting_an_edit_you_have_seen_is_no_conflict() {
    let (mut a, ca) = device();
    let (mut b, cb) = device();
    let id = shared_task(&mut a, &mut b);
    ca.store(T0 + 1_000, Ordering::SeqCst);
    rename(&mut a, &id, "oat milk");
    sync(&a, &mut b, &mut 0);
    cb.store(T0 + 2_000, Ordering::SeqCst);
    rename(&mut b, &id, "soy milk");
    sync(&b, &mut a, &mut 0);
    assert_eq!(a.get_task(&id).unwrap().title, "soy milk");
    assert!(a.conflicts().unwrap().is_empty());
}

#[test]
fn harmless_concurrent_edits_are_not_conflicts() {
    let (mut a, ca) = device();
    let (mut b, cb) = device();
    let id = shared_task(&mut a, &mut b);
    // Different fields.
    ca.store(T0 + 1_000, Ordering::SeqCst);
    rename(&mut a, &id, "oat milk");
    cb.store(T0 + 2_000, Ordering::SeqCst);
    b.update_task(&id, TaskPatch { description: Some("2 l".into()), ..Default::default() }).unwrap();
    // Both complete it, at different times.
    a.complete_task(&id, Source::App).unwrap();
    b.complete_task(&id, Source::App).unwrap();
    sync(&b, &mut a, &mut 0);
    sync(&a, &mut b, &mut 0);
    assert!(a.conflicts().unwrap().is_empty(), "{:?}", a.conflicts().unwrap());
    assert!(b.conflicts().unwrap().is_empty());
    assert_eq!(a.get_task(&id).unwrap().title, "oat milk");
    assert_eq!(a.get_task(&id).unwrap().description, "2 l");
}

#[test]
fn keep_theirs_just_closes_the_conflict() {
    let (mut a, ca) = device();
    let (mut b, cb) = device();
    let id = shared_task(&mut a, &mut b);
    ca.store(T0 + 1_000, Ordering::SeqCst);
    rename(&mut a, &id, "oat milk");
    cb.store(T0 + 2_000, Ordering::SeqCst);
    rename(&mut b, &id, "soy milk");
    sync(&b, &mut a, &mut 0);
    let c = a.conflicts().unwrap();
    assert_eq!(c[0].won_by, None, "B isn't in A's peers here");
    assert_eq!(a.resolve_conflict(&c[0].id, false).unwrap(), None);
    assert!(a.conflicts().unwrap().is_empty());
    assert_eq!(a.get_task(&id).unwrap().title, "soy milk");
    assert!(a.resolve_conflict(&c[0].id, true).is_err(), "already settled");
}

#[test]
fn no_conflicts_from_values_a_restore_rolled_back() {
    let (mut a, ca) = device();
    let (mut b, cb) = device();
    let id = shared_task(&mut a, &mut b);
    let dir = std::env::temp_dir().join(format!("befikr-test-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let backup = dir.join("a.db");
    a.export_to(backup.to_str().unwrap()).unwrap();
    // After the backup A renames (B sees it), then B renames on top.
    ca.store(T0 + 1_000, Ordering::SeqCst);
    rename(&mut a, &id, "oat milk");
    sync(&a, &mut b, &mut 0);
    cb.store(T0 + 2_000, Ordering::SeqCst);
    rename(&mut b, &id, "soy milk");

    a.import_from(backup.to_str().unwrap()).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    sync(&b, &mut a, &mut 0);
    assert_eq!(a.get_task(&id).unwrap().title, "soy milk");
    assert!(a.conflicts().unwrap().is_empty());
}

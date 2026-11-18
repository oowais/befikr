use befikr_core::*;

#[test]
fn pin_and_unpin_without_touching_the_reminder() {
    let mut s = Store::open_in_memory().unwrap();
    let id = s.add_task(NewTask { title: "passport".into(), ..Default::default() }).unwrap().task.id;
    assert!(!s.get_task(&id).unwrap().pinned, "not pinned by default");

    let c = s.update_task(&id, TaskPatch { pinned: Some(true), ..Default::default() }).unwrap();
    assert!(c.task.pinned);
    assert!(s.list_open().unwrap()[0].task.pinned);

    let c = s.update_task(&id, TaskPatch { title: Some("renew passport".into()), ..Default::default() }).unwrap();
    assert!(c.task.pinned, "other edits keep the pin");

    s.update_task(&id, TaskPatch { pinned: Some(false), ..Default::default() }).unwrap();
    assert!(!s.get_task(&id).unwrap().pinned);
}

#[test]
fn pin_syncs() {
    let mut a = Store::open_in_memory().unwrap();
    let id = a.add_task(NewTask { title: "passport".into(), ..Default::default() }).unwrap().task.id;
    a.update_task(&id, TaskPatch { pinned: Some(true), ..Default::default() }).unwrap();
    let mut b = Store::open_in_memory().unwrap();
    b.merge(&a.changes_since(0, 1000, None).unwrap()).unwrap();
    assert!(b.get_task(&id).unwrap().pinned);
}

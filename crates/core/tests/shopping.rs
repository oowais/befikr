use befikr_core::*;

fn names(s: &Store) -> (Vec<String>, Vec<String>) {
    let l = s.shopping_list().unwrap();
    (
        l.items.into_iter().map(|i| i.name).collect(),
        l.staples.into_iter().map(|s| s.name).collect(),
    )
}

#[test]
fn add_tick_undo_without_duplicates() {
    let mut s = Store::open_in_memory().unwrap();
    let milk = s.add_shopping_item("  Milk ").unwrap();
    s.add_shopping_item("eggs").unwrap();
    assert_eq!(
        s.add_shopping_item("milk").unwrap().id,
        milk.id,
        "already on the list"
    );
    assert!(s.add_shopping_item("   ").is_err());
    assert_eq!(names(&s).0, ["Milk", "eggs"]);

    assert!(s
        .set_shopping_bought(&milk.id, true)
        .unwrap()
        .bought_at
        .is_some());
    assert_eq!(names(&s).0, ["eggs"]);
    // Bought earlier → adding again makes a new item.
    assert_ne!(s.add_shopping_item("milk").unwrap().id, milk.id);

    s.set_shopping_bought(&milk.id, false).unwrap();
    assert_eq!(names(&s).0, ["Milk", "eggs", "milk"]);
}

#[test]
fn staples_add_remove() {
    let mut s = Store::open_in_memory().unwrap();
    let bread = s.add_staple("Bread").unwrap();
    s.add_staple("apples").unwrap();
    assert_eq!(s.add_staple("bread").unwrap().id, bread.id);
    assert_eq!(names(&s).1, ["apples", "Bread"]);
    s.remove_staple(&bread.id).unwrap();
    assert_eq!(names(&s).1, ["apples"]);
    assert_ne!(
        s.add_staple("bread").unwrap().id,
        bread.id,
        "removed one stays removed"
    );
}

#[test]
fn delete_undo_and_suggestions() {
    let mut s = Store::open_in_memory().unwrap();
    let soap = s.add_shopping_item("soap").unwrap();
    s.set_shopping_deleted(&soap.id, true).unwrap();
    assert!(names(&s).0.is_empty());
    s.set_shopping_deleted(&soap.id, false).unwrap();
    assert_eq!(names(&s).0, ["soap"]);

    // Bought twice → suggested; once → not; already a chip → not.
    s.set_shopping_bought(&soap.id, true).unwrap();
    s.add_shopping_item("Soap").unwrap();
    s.add_shopping_item("rice").unwrap();
    let milk = s.add_shopping_item("milk").unwrap();
    s.set_shopping_bought(&milk.id, true).unwrap();
    s.add_shopping_item("milk").unwrap();
    s.add_staple("Milk").unwrap();
    assert_eq!(s.shopping_suggestions(5).unwrap(), ["Soap"]);
}

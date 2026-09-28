mod common;

use common::{insert_user, new_file, start_store};
use trovr_metadata::MetadataError;
use uuid::Uuid;

#[tokio::test]
async fn rename_changes_the_name() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let folder = db.store.create_folder(alice, None, "old").await.unwrap();

    let renamed = db.store.rename(folder.id, "new").await.unwrap();

    assert_eq!(renamed.name, "new");
    assert!(renamed.updated_at >= folder.updated_at);
    assert_eq!(db.store.get_node(folder.id).await.unwrap().name, "new");
}

#[tokio::test]
async fn rename_rejects_invalid_and_conflicting_names() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let parent = db.store.create_folder(alice, None, "parent").await.unwrap();
    let a = db
        .store
        .create_file(new_file(alice, Some(parent.id), "a.txt", "k1"))
        .await
        .unwrap();
    db.store
        .create_file(new_file(alice, Some(parent.id), "b.txt", "k2"))
        .await
        .unwrap();

    assert!(matches!(
        db.store.rename(a.id, "b.txt").await,
        Err(MetadataError::NameConflict)
    ));
    assert!(matches!(
        db.store.rename(a.id, "..").await,
        Err(MetadataError::InvalidName(_))
    ));
    assert!(matches!(
        db.store.rename(Uuid::new_v4(), "x").await,
        Err(MetadataError::NotFound)
    ));
    assert_eq!(db.store.get_node(a.id).await.unwrap().name, "a.txt");
}

#[tokio::test]
async fn move_into_another_folder_and_back_to_root() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let source = db.store.create_folder(alice, None, "source").await.unwrap();
    let target = db.store.create_folder(alice, None, "target").await.unwrap();
    let file = db
        .store
        .create_file(new_file(alice, Some(source.id), "a.txt", "k1"))
        .await
        .unwrap();

    let moved = db.store.move_node(file.id, Some(target.id)).await.unwrap();
    assert_eq!(moved.parent_id, Some(target.id));
    assert!(db.store.list_children(source.id).await.unwrap().is_empty());
    assert_eq!(
        db.store.list_children(target.id).await.unwrap(),
        vec![moved]
    );

    let at_root = db.store.move_node(file.id, None).await.unwrap();
    assert_eq!(at_root.parent_id, None);
    let root_names: Vec<String> = db
        .store
        .list_root(alice)
        .await
        .unwrap()
        .into_iter()
        .map(|node| node.name)
        .collect();
    assert_eq!(root_names, vec!["source", "target", "a.txt"]);
}

#[tokio::test]
async fn move_into_self_or_descendant_is_rejected() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let a = db.store.create_folder(alice, None, "a").await.unwrap();
    let b = db
        .store
        .create_folder(alice, Some(a.id), "b")
        .await
        .unwrap();
    let c = db
        .store
        .create_folder(alice, Some(b.id), "c")
        .await
        .unwrap();

    assert!(matches!(
        db.store.move_node(a.id, Some(c.id)).await,
        Err(MetadataError::CycleDetected)
    ));
    assert!(matches!(
        db.store.move_node(a.id, Some(a.id)).await,
        Err(MetadataError::CycleDetected)
    ));

    let path: Vec<Uuid> = db
        .store
        .get_path(c.id)
        .await
        .unwrap()
        .into_iter()
        .map(|node| node.id)
        .collect();
    assert_eq!(
        path,
        vec![a.id, b.id, c.id],
        "a rejected move must leave the tree unchanged"
    );
}

#[tokio::test]
async fn move_rejects_file_targets_name_clashes_and_unknown_ids() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let target = db.store.create_folder(alice, None, "target").await.unwrap();
    db.store
        .create_file(new_file(alice, Some(target.id), "a.txt", "k1"))
        .await
        .unwrap();
    let other_a = db
        .store
        .create_file(new_file(alice, None, "a.txt", "k2"))
        .await
        .unwrap();
    let file_target = db
        .store
        .create_file(new_file(alice, None, "f.txt", "k3"))
        .await
        .unwrap();

    assert!(matches!(
        db.store.move_node(other_a.id, Some(target.id)).await,
        Err(MetadataError::NameConflict)
    ));
    assert!(matches!(
        db.store.move_node(other_a.id, Some(file_target.id)).await,
        Err(MetadataError::NotAFolder)
    ));
    assert!(matches!(
        db.store.move_node(Uuid::new_v4(), Some(target.id)).await,
        Err(MetadataError::NotFound)
    ));
    assert!(matches!(
        db.store.move_node(other_a.id, Some(Uuid::new_v4())).await,
        Err(MetadataError::NotFound)
    ));
    assert_eq!(db.store.get_node(other_a.id).await.unwrap().parent_id, None);
}

mod common;

use common::{content, insert_user, new_file, start_store};
use trovr_metadata::MetadataError;
use uuid::Uuid;

#[tokio::test]
async fn trash_hides_node_and_descendants() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let bob = insert_user(&db.pool, "bob").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let sub = db
        .store
        .create_folder(alice, Some(folder.id), "sub")
        .await
        .unwrap();
    let file = db
        .store
        .create_file(new_file(alice, Some(sub.id), "a.txt", "k1"))
        .await
        .unwrap();
    let elsewhere = db
        .store
        .create_folder(alice, None, "elsewhere")
        .await
        .unwrap();

    let trashed = db.store.trash(folder.id).await.unwrap();
    assert!(trashed.trashed_at.is_some());

    for id in [folder.id, sub.id, file.id] {
        assert!(matches!(
            db.store.get_node(id).await,
            Err(MetadataError::NotFound)
        ));
    }
    assert!(matches!(
        db.store.list_children(sub.id).await,
        Err(MetadataError::NotFound)
    ));
    assert!(matches!(
        db.store.create_folder(alice, Some(sub.id), "new").await,
        Err(MetadataError::NotFound)
    ));
    assert!(matches!(
        db.store.rename(file.id, "b.txt").await,
        Err(MetadataError::NotFound)
    ));
    assert!(matches!(
        db.store.move_node(file.id, Some(elsewhere.id)).await,
        Err(MetadataError::NotFound)
    ));
    assert!(matches!(
        db.store.move_node(elsewhere.id, Some(sub.id)).await,
        Err(MetadataError::NotFound)
    ));
    assert!(matches!(
        db.store.add_version(file.id, alice, content("k2", 1)).await,
        Err(MetadataError::NotFound)
    ));

    let root_names: Vec<String> = db
        .store
        .list_root(alice)
        .await
        .unwrap()
        .into_iter()
        .map(|node| node.name)
        .collect();
    assert_eq!(root_names, vec!["elsewhere"]);

    let trash_ids: Vec<Uuid> = db
        .store
        .list_trash(alice)
        .await
        .unwrap()
        .into_iter()
        .map(|node| node.id)
        .collect();
    assert_eq!(trash_ids, vec![folder.id]);
    assert!(db.store.list_trash(bob).await.unwrap().is_empty());
}

#[tokio::test]
async fn restore_brings_back_the_whole_subtree() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let file = db
        .store
        .create_file(new_file(alice, Some(folder.id), "a.txt", "k1"))
        .await
        .unwrap();

    db.store.trash(folder.id).await.unwrap();
    let restored = db.store.restore(folder.id).await.unwrap();

    assert_eq!(restored.trashed_at, None);
    assert_eq!(db.store.get_node(file.id).await.unwrap(), file);
    assert!(db.store.list_trash(alice).await.unwrap().is_empty());
}

#[tokio::test]
async fn recreating_a_trashed_name_is_allowed() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let old = db
        .store
        .create_file(new_file(alice, Some(folder.id), "report.pdf", "old"))
        .await
        .unwrap();
    let old_root = db
        .store
        .create_folder(alice, None, "Documents")
        .await
        .unwrap();

    db.store.trash(old.id).await.unwrap();
    db.store.trash(old_root.id).await.unwrap();

    db.store
        .create_file(new_file(alice, Some(folder.id), "report.pdf", "new"))
        .await
        .expect("a trashed node must not block its name");
    db.store
        .create_folder(alice, None, "Documents")
        .await
        .expect("a trashed root node must not block its name");

    assert!(matches!(
        db.store.restore(old.id).await,
        Err(MetadataError::NameConflict)
    ));
    assert!(matches!(
        db.store.restore(old_root.id).await,
        Err(MetadataError::NameConflict)
    ));
}

#[tokio::test]
async fn restoring_a_child_of_a_trashed_folder_requires_restoring_the_parent() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let file = db
        .store
        .create_file(new_file(alice, Some(folder.id), "a.txt", "k1"))
        .await
        .unwrap();

    db.store.trash(file.id).await.unwrap();
    db.store.trash(folder.id).await.unwrap();

    assert!(matches!(
        db.store.restore(file.id).await,
        Err(MetadataError::ParentTrashed)
    ));

    db.store.restore(folder.id).await.unwrap();
    db.store.restore(file.id).await.unwrap();
    assert_eq!(db.store.list_children(folder.id).await.unwrap().len(), 1);
}

#[tokio::test]
async fn trash_and_restore_reject_the_wrong_state() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();

    assert!(matches!(
        db.store.restore(folder.id).await,
        Err(MetadataError::NotTrashed)
    ));
    assert!(matches!(
        db.store.restore(Uuid::new_v4()).await,
        Err(MetadataError::NotFound)
    ));

    db.store.trash(folder.id).await.unwrap();
    assert!(matches!(
        db.store.trash(folder.id).await,
        Err(MetadataError::NotFound)
    ));
}

#[tokio::test]
async fn purge_deletes_subtree_and_returns_all_storage_keys() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let a = db
        .store
        .create_file(new_file(alice, Some(folder.id), "a.txt", "a-v1"))
        .await
        .unwrap();
    db.store
        .add_version(a.id, alice, content("a-v2", 7))
        .await
        .unwrap();
    let sub = db
        .store
        .create_folder(alice, Some(folder.id), "sub")
        .await
        .unwrap();
    db.store
        .create_file(new_file(alice, Some(sub.id), "b.txt", "b-v1"))
        .await
        .unwrap();
    let kept = db
        .store
        .create_file(new_file(alice, None, "kept.txt", "kept-v1"))
        .await
        .unwrap();

    db.store.trash(folder.id).await.unwrap();
    let keys = db.store.purge(folder.id).await.unwrap();

    assert_eq!(keys, vec!["a-v1", "a-v2", "b-v1"]);

    let remaining_nodes: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM nodes")
        .fetch_all(&db.pool)
        .await
        .unwrap();
    assert_eq!(remaining_nodes, vec![kept.id]);

    let remaining_keys: Vec<String> = sqlx::query_scalar("SELECT storage_key FROM file_versions")
        .fetch_all(&db.pool)
        .await
        .unwrap();
    assert_eq!(remaining_keys, vec!["kept-v1"]);
}

#[tokio::test]
async fn purge_only_accepts_trashed_nodes() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let child = db
        .store
        .create_folder(alice, Some(folder.id), "child")
        .await
        .unwrap();

    assert!(matches!(
        db.store.purge(folder.id).await,
        Err(MetadataError::NotTrashed)
    ));
    assert!(matches!(
        db.store.purge(Uuid::new_v4()).await,
        Err(MetadataError::NotFound)
    ));

    db.store.trash(folder.id).await.unwrap();
    assert!(
        matches!(
            db.store.purge(child.id).await,
            Err(MetadataError::NotTrashed)
        ),
        "only the trashed subtree root can be purged"
    );
}

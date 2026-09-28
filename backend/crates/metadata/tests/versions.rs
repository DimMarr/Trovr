mod common;

use common::{content, insert_user, new_file, start_store};
use trovr_metadata::MetadataError;

#[tokio::test]
async fn add_version_increments_and_becomes_current() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let bob = insert_user(&db.pool, "bob").await;
    let file = db
        .store
        .create_file(new_file(alice, None, "a.txt", "v1"))
        .await
        .unwrap();

    let v2 = db
        .store
        .add_version(file.id, bob, content("v2", 999))
        .await
        .unwrap();

    assert_eq!(v2.version_number, 2);
    assert_eq!(v2.created_by, bob);
    assert_eq!(db.store.current_version(file.id).await.unwrap(), v2);

    let node = db.store.get_node(file.id).await.unwrap();
    assert_eq!(node.current_version_id, Some(v2.id));
    assert_eq!(node.size_bytes, 999);
    assert!(node.updated_at >= file.updated_at);
}

#[tokio::test]
async fn list_versions_returns_newest_first() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let file = db
        .store
        .create_file(new_file(alice, None, "a.txt", "v1"))
        .await
        .unwrap();
    db.store
        .add_version(file.id, alice, content("v2", 1))
        .await
        .unwrap();
    db.store
        .add_version(file.id, alice, content("v3", 2))
        .await
        .unwrap();

    let versions: Vec<(i32, String)> = db
        .store
        .list_versions(file.id)
        .await
        .unwrap()
        .into_iter()
        .map(|version| (version.version_number, version.storage_key))
        .collect();

    assert_eq!(
        versions,
        vec![
            (3, "v3".to_string()),
            (2, "v2".to_string()),
            (1, "v1".to_string())
        ]
    );
}

#[tokio::test]
async fn versions_are_file_only_and_validated() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let file = db
        .store
        .create_file(new_file(alice, None, "a.txt", "v1"))
        .await
        .unwrap();

    assert!(matches!(
        db.store
            .add_version(folder.id, alice, content("x", 1))
            .await,
        Err(MetadataError::NotAFile)
    ));
    assert!(matches!(
        db.store.list_versions(folder.id).await,
        Err(MetadataError::NotAFile)
    ));
    assert!(matches!(
        db.store.current_version(folder.id).await,
        Err(MetadataError::NotAFile)
    ));
    assert!(matches!(
        db.store.add_version(file.id, alice, content("x", -1)).await,
        Err(MetadataError::InvalidContent(_))
    ));
    assert_eq!(db.store.list_versions(file.id).await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_storage_key_can_back_only_one_version() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let file = db
        .store
        .create_file(new_file(alice, None, "a.txt", "shared-key"))
        .await
        .unwrap();

    assert!(matches!(
        db.store
            .add_version(file.id, alice, content("shared-key", 1))
            .await,
        Err(MetadataError::StorageKeyInUse)
    ));
    assert!(matches!(
        db.store
            .create_file(new_file(alice, None, "b.txt", "shared-key"))
            .await,
        Err(MetadataError::StorageKeyInUse)
    ));
    assert_eq!(db.store.list_versions(file.id).await.unwrap().len(), 1);
    assert_eq!(db.store.list_root(alice).await.unwrap().len(), 1);
}

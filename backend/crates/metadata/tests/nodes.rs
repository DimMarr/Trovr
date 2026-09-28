mod common;

use common::{insert_user, new_file, start_store};
use trovr_metadata::{MetadataError, NewContent, NodeType};
use uuid::Uuid;

#[tokio::test]
async fn list_root_returns_folders_first_then_files_for_that_owner_only() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let bob = insert_user(&db.pool, "bob").await;

    db.store
        .create_file(new_file(alice, None, "a.txt", "k1"))
        .await
        .unwrap();
    db.store.create_folder(alice, None, "photos").await.unwrap();
    db.store
        .create_folder(alice, None, "documents")
        .await
        .unwrap();
    db.store.create_folder(bob, None, "bobs").await.unwrap();

    let names: Vec<String> = db
        .store
        .list_root(alice)
        .await
        .unwrap()
        .into_iter()
        .map(|node| node.name)
        .collect();

    assert_eq!(names, vec!["documents", "photos", "a.txt"]);
}

#[tokio::test]
async fn nested_folders_are_listed_and_have_a_root_first_path() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;

    let root = db.store.create_folder(alice, None, "root").await.unwrap();
    let child = db
        .store
        .create_folder(alice, Some(root.id), "child")
        .await
        .unwrap();
    let file = db
        .store
        .create_file(new_file(alice, Some(child.id), "notes.txt", "k1"))
        .await
        .unwrap();

    let children = db.store.list_children(root.id).await.unwrap();
    assert_eq!(children, vec![child.clone()]);

    let path: Vec<Uuid> = db
        .store
        .get_path(file.id)
        .await
        .unwrap()
        .into_iter()
        .map(|node| node.id)
        .collect();
    assert_eq!(path, vec![root.id, child.id, file.id]);

    assert_eq!(db.store.get_node(child.id).await.unwrap(), child);
}

#[tokio::test]
async fn create_file_records_its_first_version_atomically() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;

    let mut input = new_file(alice, None, "photo.png", "objects/photo-v1");
    input.mime_type = "image/png".to_string();
    input.content = NewContent {
        storage_key: "objects/photo-v1".to_string(),
        size_bytes: 1234,
        checksum_sha256: Some("abc123".to_string()),
    };
    let node = db.store.create_file(input).await.unwrap();

    assert_eq!(node.node_type, NodeType::File);
    assert_eq!(node.size_bytes, 1234);
    assert_eq!(node.mime_type.as_deref(), Some("image/png"));

    let (version_id, version_number, storage_key, size_bytes, created_by): (Uuid, i32, String, i64, Uuid) =
        sqlx::query_as(
            "SELECT id, version_number, storage_key, size_bytes, created_by FROM file_versions WHERE node_id = $1",
        )
        .bind(node.id)
        .fetch_one(&db.pool)
        .await
        .unwrap();

    assert_eq!(node.current_version_id, Some(version_id));
    assert_eq!(version_number, 1);
    assert_eq!(storage_key, "objects/photo-v1");
    assert_eq!(size_bytes, 1234);
    assert_eq!(created_by, alice);
}

#[tokio::test]
async fn duplicate_name_in_the_same_folder_is_a_name_conflict() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let parent = db.store.create_folder(alice, None, "parent").await.unwrap();

    db.store
        .create_folder(alice, Some(parent.id), "dup")
        .await
        .unwrap();

    let folder_again = db.store.create_folder(alice, Some(parent.id), "dup").await;
    assert!(matches!(folder_again, Err(MetadataError::NameConflict)));

    let file_with_same_name = db
        .store
        .create_file(new_file(alice, Some(parent.id), "dup", "k1"))
        .await;
    assert!(matches!(
        file_with_same_name,
        Err(MetadataError::NameConflict)
    ));

    let file_count: i64 = sqlx::query_scalar("SELECT count(*) FROM file_versions")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        file_count, 0,
        "a failed create_file must not leave a version behind"
    );
}

#[tokio::test]
async fn duplicate_root_name_conflicts_per_owner_only() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let bob = insert_user(&db.pool, "bob").await;

    db.store
        .create_folder(alice, None, "Documents")
        .await
        .unwrap();

    let again = db.store.create_folder(alice, None, "Documents").await;
    assert!(matches!(again, Err(MetadataError::NameConflict)));

    db.store
        .create_folder(bob, None, "Documents")
        .await
        .expect("another owner may use the same root name");
}

#[tokio::test]
async fn a_file_cannot_be_used_as_a_parent() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let file = db
        .store
        .create_file(new_file(alice, None, "a.txt", "k1"))
        .await
        .unwrap();

    let child = db.store.create_folder(alice, Some(file.id), "child").await;
    assert!(matches!(child, Err(MetadataError::NotAFolder)));

    let listing = db.store.list_children(file.id).await;
    assert!(matches!(listing, Err(MetadataError::NotAFolder)));
}

#[tokio::test]
async fn unknown_ids_are_not_found() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let unknown = Uuid::new_v4();

    assert!(matches!(
        db.store.get_node(unknown).await,
        Err(MetadataError::NotFound)
    ));
    assert!(matches!(
        db.store.get_path(unknown).await,
        Err(MetadataError::NotFound)
    ));
    assert!(matches!(
        db.store.list_children(unknown).await,
        Err(MetadataError::NotFound)
    ));
    assert!(matches!(
        db.store.create_folder(alice, Some(unknown), "x").await,
        Err(MetadataError::NotFound)
    ));
}

#[tokio::test]
async fn invalid_input_is_rejected_before_touching_the_database() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;

    let bad_name = db.store.create_folder(alice, None, "a/b").await;
    assert!(matches!(bad_name, Err(MetadataError::InvalidName(_))));

    let mut negative_size = new_file(alice, None, "a.txt", "k1");
    negative_size.content.size_bytes = -5;
    assert!(matches!(
        db.store.create_file(negative_size).await,
        Err(MetadataError::InvalidContent(_))
    ));

    let mut no_mime = new_file(alice, None, "a.txt", "k1");
    no_mime.mime_type = String::new();
    assert!(matches!(
        db.store.create_file(no_mime).await,
        Err(MetadataError::InvalidContent(_))
    ));

    assert!(db.store.list_root(alice).await.unwrap().is_empty());
}

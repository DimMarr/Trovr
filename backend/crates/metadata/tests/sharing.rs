mod common;

use common::{insert_user, new_file, start_store};
use trovr_metadata::{MetadataError, Role};

#[tokio::test]
async fn owners_of_a_node_or_any_ancestor_are_owners() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let bob = insert_user(&db.pool, "bob").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let bobs_file = db
        .store
        .create_file(new_file(bob, Some(folder.id), "bob.txt", "k1"))
        .await
        .unwrap();

    assert_eq!(
        db.store.effective_role(folder.id, alice).await.unwrap(),
        Some(Role::Owner)
    );
    assert_eq!(
        db.store.effective_role(bobs_file.id, alice).await.unwrap(),
        Some(Role::Owner)
    );
    assert_eq!(
        db.store.effective_role(bobs_file.id, bob).await.unwrap(),
        Some(Role::Owner)
    );
    assert_eq!(db.store.effective_role(folder.id, bob).await.unwrap(), None);
}

#[tokio::test]
async fn grants_are_inherited_by_the_subtree_and_the_highest_wins() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let carol = insert_user(&db.pool, "carol").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let sub = db
        .store
        .create_folder(alice, Some(folder.id), "sub")
        .await
        .unwrap();
    let deeper = db
        .store
        .create_folder(alice, Some(sub.id), "deeper")
        .await
        .unwrap();
    let elsewhere = db
        .store
        .create_folder(alice, None, "elsewhere")
        .await
        .unwrap();

    db.store
        .share_with_user(folder.id, carol, Role::Viewer, alice)
        .await
        .unwrap();
    db.store
        .share_with_user(sub.id, carol, Role::Editor, alice)
        .await
        .unwrap();

    assert_eq!(
        db.store.effective_role(folder.id, carol).await.unwrap(),
        Some(Role::Viewer)
    );
    assert_eq!(
        db.store.effective_role(sub.id, carol).await.unwrap(),
        Some(Role::Editor)
    );
    assert_eq!(
        db.store.effective_role(deeper.id, carol).await.unwrap(),
        Some(Role::Editor)
    );
    assert_eq!(
        db.store.effective_role(elsewhere.id, carol).await.unwrap(),
        None
    );
}

#[tokio::test]
async fn expired_grants_confer_nothing() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let carol = insert_user(&db.pool, "carol").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let sub = db
        .store
        .create_folder(alice, Some(folder.id), "sub")
        .await
        .unwrap();
    db.store
        .share_with_user(folder.id, carol, Role::Editor, alice)
        .await
        .unwrap();

    sqlx::query("UPDATE node_permissions SET expires_at = now() - interval '1 minute'")
        .execute(&db.pool)
        .await
        .unwrap();

    assert_eq!(
        db.store.effective_role(folder.id, carol).await.unwrap(),
        None
    );
    assert_eq!(db.store.effective_role(sub.id, carol).await.unwrap(), None);
    assert!(db.store.list_shared_with(carol).await.unwrap().is_empty());
}

#[tokio::test]
async fn sharing_again_updates_the_role_and_unsharing_removes_it() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let carol = insert_user(&db.pool, "carol").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();

    db.store
        .share_with_user(folder.id, carol, Role::Viewer, alice)
        .await
        .unwrap();
    let updated = db
        .store
        .share_with_user(folder.id, carol, Role::Editor, alice)
        .await
        .unwrap();
    assert_eq!(updated.role, Role::Editor);
    assert_eq!(updated.user_id, carol);

    let shares = db.store.list_user_shares(folder.id).await.unwrap();
    assert_eq!(shares.len(), 1);
    assert_eq!(shares[0].role, Role::Editor);

    db.store.unshare_with_user(folder.id, carol).await.unwrap();
    assert!(
        db.store
            .list_user_shares(folder.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.store.effective_role(folder.id, carol).await.unwrap(),
        None
    );
    assert!(matches!(
        db.store.unshare_with_user(folder.id, carol).await,
        Err(MetadataError::NotFound)
    ));
}

#[tokio::test]
async fn list_shared_with_returns_directly_shared_live_nodes() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let carol = insert_user(&db.pool, "carol").await;
    let docs = db.store.create_folder(alice, None, "docs").await.unwrap();
    let nested = db
        .store
        .create_folder(alice, Some(docs.id), "nested")
        .await
        .unwrap();
    let trashed = db
        .store
        .create_folder(alice, None, "trashed")
        .await
        .unwrap();
    let carols = db.store.create_folder(carol, None, "carols").await.unwrap();

    for node in [docs.id, nested.id, trashed.id] {
        db.store
            .share_with_user(node, carol, Role::Viewer, alice)
            .await
            .unwrap();
    }
    db.store
        .share_with_user(carols.id, carol, Role::Editor, carol)
        .await
        .unwrap();
    db.store.trash(trashed.id).await.unwrap();

    let names: Vec<String> = db
        .store
        .list_shared_with(carol)
        .await
        .unwrap()
        .into_iter()
        .map(|node| node.name)
        .collect();

    assert_eq!(names, vec!["docs", "nested"]);
}

#[tokio::test]
async fn links_resolve_until_deleted_or_expired() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let other = db.store.create_folder(alice, None, "other").await.unwrap();

    let link = db.store.create_link(folder.id, None, alice).await.unwrap();
    let second = db.store.create_link(folder.id, None, alice).await.unwrap();
    assert_ne!(link.token, second.token);
    assert!(link.token.len() >= 60);
    assert_eq!(link.role, Role::Viewer);
    assert_eq!(
        db.store.resolve_link(&link.token).await.unwrap().node_id,
        folder.id
    );
    assert_eq!(db.store.list_links(folder.id).await.unwrap().len(), 2);

    let expired = db
        .store
        .create_link(
            folder.id,
            Some(chrono::Utc::now() - chrono::TimeDelta::minutes(1)),
            alice,
        )
        .await
        .unwrap();
    assert!(matches!(
        db.store.resolve_link(&expired.token).await,
        Err(MetadataError::NotFound)
    ));

    assert!(matches!(
        db.store.delete_link(other.id, link.id).await,
        Err(MetadataError::NotFound)
    ));
    db.store.delete_link(folder.id, link.id).await.unwrap();
    assert!(matches!(
        db.store.resolve_link(&link.token).await,
        Err(MetadataError::NotFound)
    ));
    assert!(matches!(
        db.store.resolve_link("unknown").await,
        Err(MetadataError::NotFound)
    ));
}

#[tokio::test]
async fn links_to_trashed_nodes_do_not_resolve() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let folder = db.store.create_folder(alice, None, "folder").await.unwrap();
    let child = db
        .store
        .create_folder(alice, Some(folder.id), "child")
        .await
        .unwrap();
    let link = db.store.create_link(child.id, None, alice).await.unwrap();

    db.store.trash(folder.id).await.unwrap();
    assert!(matches!(
        db.store.resolve_link(&link.token).await,
        Err(MetadataError::NotFound)
    ));

    db.store.restore(folder.id).await.unwrap();
    assert_eq!(
        db.store.resolve_link(&link.token).await.unwrap().node_id,
        child.id
    );
}

#[tokio::test]
async fn is_within_only_accepts_the_subtree() {
    let db = start_store().await;
    let alice = insert_user(&db.pool, "alice").await;
    let parent = db.store.create_folder(alice, None, "parent").await.unwrap();
    let root = db
        .store
        .create_folder(alice, Some(parent.id), "root")
        .await
        .unwrap();
    let sibling = db
        .store
        .create_folder(alice, Some(parent.id), "sibling")
        .await
        .unwrap();
    let child = db
        .store
        .create_folder(alice, Some(root.id), "child")
        .await
        .unwrap();
    let grandchild = db
        .store
        .create_folder(alice, Some(child.id), "grandchild")
        .await
        .unwrap();
    let hidden = db
        .store
        .create_folder(alice, Some(root.id), "hidden")
        .await
        .unwrap();
    let under_hidden = db
        .store
        .create_folder(alice, Some(hidden.id), "under")
        .await
        .unwrap();
    db.store.trash(hidden.id).await.unwrap();

    for node in [root.id, child.id, grandchild.id] {
        assert!(db.store.is_within(node, root.id).await.unwrap());
    }
    for node in [parent.id, sibling.id, hidden.id, under_hidden.id] {
        assert!(!db.store.is_within(node, root.id).await.unwrap());
    }
}

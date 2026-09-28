mod common;

use std::time::Duration;

use common::{send, start_storage, upload};
use trovr_storage::{ObjectInfo, ObjectStorage, StorageConfig, StorageError};
use uuid::Uuid;

const OWNER: Uuid = Uuid::nil();

#[tokio::test]
async fn presigned_upload_then_head_then_download_round_trips() {
    let test = start_storage().await;
    let key = ObjectStorage::new_object_key(OWNER);

    let upload = test
        .storage
        .presign_upload(&key, "text/plain", Duration::from_secs(60))
        .await
        .unwrap();
    let response = send(&test.http, &upload, b"hello trovr".to_vec()).await;
    assert!(
        response.status().is_success(),
        "upload failed: {}",
        response.status()
    );

    let info = test.storage.head(&key).await.unwrap();
    assert_eq!(
        info,
        Some(ObjectInfo {
            size_bytes: 11,
            content_type: Some("text/plain".to_string()),
        })
    );

    let download = test
        .storage
        .presign_download(&key, "héllo \"quoted\".txt", Duration::from_secs(60))
        .await
        .unwrap();
    let response = send(&test.http, &download, Vec::new()).await;
    assert!(
        response.status().is_success(),
        "download failed: {}",
        response.status()
    );

    let disposition = response
        .headers()
        .get("content-disposition")
        .expect("download should carry a content disposition")
        .to_str()
        .unwrap()
        .to_string();
    assert!(disposition.starts_with("attachment;"), "{disposition}");
    assert!(
        disposition.contains("filename*=UTF-8''h%C3%A9llo%20%22quoted%22.txt"),
        "{disposition}"
    );
    assert_eq!(response.bytes().await.unwrap().as_ref(), b"hello trovr");
}

#[tokio::test]
async fn upload_without_the_signed_content_type_is_rejected() {
    let test = start_storage().await;
    let key = ObjectStorage::new_object_key(OWNER);

    let mut upload = test
        .storage
        .presign_upload(&key, "image/png", Duration::from_secs(60))
        .await
        .unwrap();
    for (name, value) in &mut upload.headers {
        if name.eq_ignore_ascii_case("content-type") {
            *value = "text/html".to_string();
        }
    }

    let response = send(&test.http, &upload, b"<script>".to_vec()).await;

    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(test.storage.head(&key).await.unwrap(), None);
}

#[tokio::test]
async fn expired_upload_urls_are_rejected() {
    let test = start_storage().await;
    let key = ObjectStorage::new_object_key(OWNER);

    let upload = test
        .storage
        .presign_upload(&key, "text/plain", Duration::from_secs(1))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;

    let response = send(&test.http, &upload, b"too late".to_vec()).await;

    // The status is backend-specific: AWS answers 403, Garage 400.
    assert!(
        response.status().is_client_error(),
        "an expired URL must be refused, got {}",
        response.status()
    );
    assert_eq!(test.storage.head(&key).await.unwrap(), None);
}

#[tokio::test]
async fn head_of_a_missing_object_is_none() {
    let test = start_storage().await;

    let info = test
        .storage
        .head(&ObjectStorage::new_object_key(OWNER))
        .await
        .unwrap();

    assert_eq!(info, None);
}

#[tokio::test]
async fn head_with_wrong_credentials_is_an_error_not_a_missing_object() {
    let test = start_storage().await;
    let key = ObjectStorage::new_object_key(OWNER);
    upload(&test, &key, b"data").await;

    let misconfigured = ObjectStorage::new(&StorageConfig {
        secret_access_key: "wrong-secret".to_string(),
        ..test.config.clone()
    });

    let result = misconfigured.head(&key).await;

    assert!(
        matches!(result, Err(StorageError::Request(_))),
        "expected a request error, got {result:?}"
    );
}

#[tokio::test]
async fn delete_removes_objects_across_batches_and_ignores_missing_keys() {
    let test = start_storage().await;
    let first = ObjectStorage::new_object_key(OWNER);
    let second = ObjectStorage::new_object_key(OWNER);
    upload(&test, &first, b"one").await;
    upload(&test, &second, b"two").await;

    // 1001 keys forces two DeleteObjects requests (S3 caps a request at
    // 1000 keys); all but two of them never existed.
    let mut keys: Vec<String> = (0..999)
        .map(|_| ObjectStorage::new_object_key(OWNER))
        .collect();
    keys.push(first.clone());
    keys.push(second.clone());

    test.storage.delete(&keys).await.unwrap();

    assert_eq!(test.storage.head(&first).await.unwrap(), None);
    assert_eq!(test.storage.head(&second).await.unwrap(), None);
    test.storage
        .delete(&[])
        .await
        .expect("deleting nothing is a no-op");
}

//! Integration tests against an in-memory `SQLite`.
//!
//! Covers the happy path for project / collection / request creation,
//! rename, list, delete, and cascade-on-delete.

use apiovnia_core::model::{HttpMethod, Request};
use apiovnia_storage::{CollectionRepo, Db, ProjectRepo, RequestRepo, StorageError};

async fn fresh_db() -> Db {
    Db::open_in_memory().await.expect("open in-memory db")
}

#[tokio::test]
async fn project_crud_happy_path() {
    let db = fresh_db().await;
    let pool = db.pool();

    let created = ProjectRepo::create(pool, "UDL").await.unwrap();
    assert_eq!(created.name, "UDL");

    let listed = ProjectRepo::list(pool).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);

    let renamed = ProjectRepo::rename(pool, &created.id, "UDL v2").await.unwrap();
    assert_eq!(renamed.name, "UDL v2");
    assert!(renamed.updated_at >= renamed.created_at);

    ProjectRepo::delete(pool, &created.id).await.unwrap();
    assert!(ProjectRepo::list(pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn rejects_empty_names() {
    let db = fresh_db().await;
    let pool = db.pool();

    let err = ProjectRepo::create(pool, "   ").await.unwrap_err();
    assert!(matches!(err, StorageError::InvalidData(_)));
}

#[tokio::test]
async fn delete_project_cascades_to_collections_and_requests() {
    let db = fresh_db().await;
    let pool = db.pool();

    let p = ProjectRepo::create(pool, "P").await.unwrap();
    let c = CollectionRepo::create(pool, &p.id, "C").await.unwrap();
    let _r = RequestRepo::create_blank(pool, &c.id, "R").await.unwrap();

    assert_eq!(
        CollectionRepo::list_in_project(pool, &p.id).await.unwrap().len(),
        1
    );
    assert_eq!(
        RequestRepo::list_in_collection(pool, &c.id).await.unwrap().len(),
        1
    );

    ProjectRepo::delete(pool, &p.id).await.unwrap();

    assert!(CollectionRepo::list_in_project(pool, &p.id).await.unwrap().is_empty());
    assert!(RequestRepo::list_in_collection(pool, &c.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn request_full_round_trip() {
    let db = fresh_db().await;
    let pool = db.pool();

    let p = ProjectRepo::create(pool, "P").await.unwrap();
    let c = CollectionRepo::create(pool, &p.id, "C").await.unwrap();
    let blank = RequestRepo::create_blank(pool, &c.id, "Login").await.unwrap();

    // Mutate everything, save, re-read, assert exact equality.
    let mut patch = blank.clone();
    patch.method = HttpMethod::Post;
    patch.url = "https://api.example/auth/login".into();
    patch.headers = vec![apiovnia_core::model::KeyValue {
        key: "Content-Type".into(),
        value: "application/json".into(),
        enabled: true,
    }];
    patch.body_type = apiovnia_core::model::BodyType::Json;
    patch.body_content = r#"{"email":"x@y.z"}"#.into();
    patch.auth = apiovnia_core::model::AuthConfig::Bearer { token: "tok".into() };

    let saved = RequestRepo::update_full(pool, &blank.id, &patch).await.unwrap();
    assert_eq!(saved.method, HttpMethod::Post);
    assert_eq!(saved.url, "https://api.example/auth/login");
    assert_eq!(saved.headers.len(), 1);
    assert_eq!(saved.body_content, r#"{"email":"x@y.z"}"#);

    // Re-fetch confirms persistence (not just the returned object).
    let fetched = RequestRepo::get(pool, &blank.id).await.unwrap();
    assert_eq!(fetched, saved);
}

#[tokio::test]
async fn updating_unknown_request_yields_not_found() {
    let db = fresh_db().await;
    let pool = db.pool();
    let bogus_id = apiovnia_core::ids::RequestId::new();
    let dummy = Request::new_blank(
        bogus_id.clone(),
        apiovnia_core::ids::CollectionId::new(),
        "x".into(),
        0,
        0,
    );
    let err = RequestRepo::update_full(pool, &bogus_id, &dummy).await.unwrap_err();
    assert!(matches!(err, StorageError::NotFound));
}

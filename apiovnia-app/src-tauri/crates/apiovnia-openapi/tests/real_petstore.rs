//! Integration test against the real OAI petstore-expanded fixture
//! (4 operations across two paths, a couple of $ref bodies, allOf
//! schema for `Pet`). Locks in the dummy-from-schema synthesis we
//! added so it doesn't regress.

use apiovnia_core::ids::{CollectionId, ProjectId};
use apiovnia_core::model::{BodyType, HttpMethod};
use apiovnia_openapi::import_document;

const FIXTURE: &str = include_str!("petstore_sample.yaml");

#[test]
fn imports_real_petstore_with_synthesised_post_body() {
    let out = import_document(FIXTURE, &ProjectId::new(), CollectionId::new()).unwrap();

    assert_eq!(out.collection.name, "Swagger Petstore");
    assert_eq!(out.requests.len(), 4, "expected 4 operations");

    // Sanity: the four ops are findPets, addPet, find pet by id, deletePet.
    let names: Vec<&str> = out.requests.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"findPets"));
    assert!(names.contains(&"addPet"));
    assert!(names.contains(&"deletePet"));

    // The whole point of this fixture: POST /pets has a $ref body with
    // no example. Pre-fix that imported as an empty `{}`; with the schema
    // synthesiser we should see populated keys.
    let add_pet = out
        .requests
        .iter()
        .find(|r| r.method == HttpMethod::Post && r.url.ends_with("/pets"))
        .expect("missing addPet");
    assert_eq!(add_pet.body_type, BodyType::Json);

    let body: serde_json::Value = serde_json::from_str(&add_pet.body_content)
        .expect("addPet body should be valid JSON");
    // NewPet has { name: string, tag: string } — both should be present
    // with string placeholders.
    assert_eq!(body["name"], "string");
    assert_eq!(body["tag"], "string");
}

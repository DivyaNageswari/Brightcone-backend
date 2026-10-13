//! Fails when `openapi.json` is stale. Fix with: `UPDATE_OPENAPI=1 cargo test --test openapi`
//! then commit the file. The frontend regenerates its types from this file.
use brightcone_hr_backend::openapi::ApiDoc;
use utoipa::OpenApi;

#[test]
fn committed_openapi_json_is_up_to_date() {
    let generated = format!(
        "{}\n",
        ApiDoc::openapi().to_pretty_json().expect("serialise")
    );
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");

    if std::env::var("UPDATE_OPENAPI").is_ok() {
        std::fs::write(path, &generated).expect("write openapi.json");
        return;
    }
    let committed = std::fs::read_to_string(path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        committed, generated,
        "openapi.json is out of date. Run: UPDATE_OPENAPI=1 cargo test --test openapi"
    );
}

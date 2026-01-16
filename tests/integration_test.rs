use blup::core::downloader;
use reqwest::Client;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_download_command_with_mock() {
    let mock_server = MockServer::start().await;

    let fixture_path = "tests/fixtures/fake_blender.zip";
    let body = std::fs::read(fixture_path)
        .expect("Fixture not found. Did you run 'zip tests/fixtures/fake_blender.zip ...'?");

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let dest_path = temp_dir.path().join("downloaded_blender.zip");

    let client = Client::new();
    let url = format!("{}/fake_blender.zip", &mock_server.uri());

    let result = downloader::download_file(&client, &url, &dest_path).await;

    assert!(result.is_ok(), "Download failed: {:?}", result.err());
    assert!(dest_path.exists(), "File was not created");

    let metadata = std::fs::metadata(dest_path).unwrap();
    assert!(metadata.len() > 0);
}

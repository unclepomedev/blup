use blup::core::{downloader, extractor};
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

    let client = Client::new();
    let url = format!("{}/fake_blender.zip", &mock_server.uri());
    let archive_path = temp_dir.path().join("downloaded.zip");

    downloader::download_file(&client, &url, &archive_path).await.unwrap();

    let extract_dir = temp_dir.path().join("extracted");
    let result = extractor::extract(&archive_path, &extract_dir);

    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());

    let exe_path = extract_dir.join("Blender5.0").join("blender.exe");
    assert!(exe_path.exists(), "Extracted file not found at {:?}", exe_path);
}

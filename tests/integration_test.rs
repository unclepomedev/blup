use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_download_command() {
    let mock_server = MockServer::start().await;

    let fixture_path = "tests/fixtures/fake_blender.zip";
    let body = std::fs::read(fixture_path).expect("Fixture not found");

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .mount(&mock_server)
        .await;

    let base_url = mock_server.uri();

    // TODO: call Downloader::download
    // let result = blup::core::downloader::download(&base_url, "4.2.0", ...).await;

    // assert!(result.is_ok());
}
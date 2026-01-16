use blup::core::{downloader, extractor};
use reqwest::Client;
use std::io::{Cursor, Write};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};
use zip::write::FileOptions;

#[tokio::test]
async fn test_download_and_extract_flow() {
    let mock_server = MockServer::start().await;

    let mut buffer = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buffer));

        let options =
            FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("Blender5.0/blender.exe", options).unwrap();
        zip.write_all(b"fake blender binary content").unwrap();

        zip.finish().unwrap();
    }

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(buffer))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let client = Client::new();
    let url = format!("{}/fake_blender.zip", &mock_server.uri());
    let archive_path = temp_dir.path().join("downloaded.zip");

    downloader::download_file(&client, &url, &archive_path)
        .await
        .unwrap();

    let extract_dir = temp_dir.path().join("extracted");
    let result = extractor::extract(&archive_path, &extract_dir);

    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());

    let exe_path = extract_dir.join("Blender5.0").join("blender.exe");
    assert!(
        exe_path.exists(),
        "Extracted file not found at {:?}",
        exe_path
    );
}

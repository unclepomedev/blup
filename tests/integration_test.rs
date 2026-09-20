use blup::core::os::Platform;
use blup::core::{archive, downloader, extractor};
use reqwest::Client;
use std::fs;
use std::io::{Cursor, Write};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

#[tokio::test]
async fn test_download_and_extract_flow() {
    let mock_server = MockServer::start().await;

    let buffer = {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));

        let options = FileOptions::<()>::default().compression_method(CompressionMethod::Stored);

        zip.start_file("Blender5.0/blender.exe", options).unwrap();
        zip.write_all(b"fake blender binary content").unwrap();

        let cursor = zip.finish().unwrap();
        cursor.into_inner()
    };

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(buffer))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let client = Client::new();
    let url = format!("{}/fake_blender.zip", mock_server.uri());
    let archive_path = temp_dir.path().join("downloaded.zip");

    downloader::download_file(&client, &url, &archive_path)
        .await
        .unwrap();

    let extract_dir = temp_dir.path().join("extracted");

    let meta = fs::metadata(&archive_path).unwrap();
    println!("Downloaded file size: {} bytes", meta.len());

    let result = extractor::extract(&archive_path, &extract_dir);

    if let Err(e) = &result {
        println!("Extraction error details: {:?}", e);
    }
    assert!(result.is_ok(), "Extraction failed: {:?}", result.err());

    let exe_path = extract_dir.join("blender.exe");
    assert!(
        exe_path.exists(),
        "Extracted file not found at {:?}",
        exe_path
    );
}

#[tokio::test]
async fn test_fetch_all_versions_from_archive() {
    let mock_server = MockServer::start().await;

    let root_index = r#"
        <a href="../">../</a>
        <a href="Blender2.79/">Blender2.79/</a>
        <a href="Blender5.2/">Blender5.2/</a>
        <a href="Blender5.10/">Blender5.10/</a>
    "#;

    let series_5_2 = r#"
        <a href="blender-5.2.0-linux-x64.tar.xz">blender-5.2.0-linux-x64.tar.xz</a>
        <a href="blender-5.2.10-linux-x64.tar.xz">blender-5.2.10-linux-x64.tar.xz</a>
        <a href="blender-5.2.0-macos-arm64.dmg">blender-5.2.0-macos-arm64.dmg</a>
        <a href="blender-5.2.0.sha256">blender-5.2.0.sha256</a>
    "#;

    let series_5_10 = r#"
        <a href="blender-5.10.0-linux-x64.tar.xz">blender-5.10.0-linux-x64.tar.xz</a>
    "#;

    Mock::given(method("GET"))
        .and(path("/release/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(root_index))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/release/Blender5.2/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(series_5_2))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/release/Blender5.10/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(series_5_10))
        .mount(&mock_server)
        .await;

    let platform = Platform {
        os: "linux".to_string(),
        arch: "x64".to_string(),
        ext: "tar.xz".to_string(),
    };

    let client = Client::new();
    let base = format!("{}/release", mock_server.uri());

    let listing = archive::fetch_all_versions(&client, &base, &platform)
        .await
        .unwrap();

    assert_eq!(listing.versions, vec!["5.10.0", "5.2.10", "5.2.0"]);
    assert!(listing.skipped.is_empty());
}

#[tokio::test]
async fn test_fetch_all_versions_reports_skipped_series() {
    let mock_server = MockServer::start().await;

    let root_index = r#"
        <a href="../">../</a>
        <a href="Blender5.2/">Blender5.2/</a>
        <a href="Blender5.10/">Blender5.10/</a>
    "#;

    let series_5_2 = r#"
        <a href="blender-5.2.0-linux-x64.tar.xz">blender-5.2.0-linux-x64.tar.xz</a>
    "#;

    Mock::given(method("GET"))
        .and(path("/release/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(root_index))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/release/Blender5.2/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(series_5_2))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/release/Blender5.10/"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock_server)
        .await;

    let platform = Platform {
        os: "linux".to_string(),
        arch: "x64".to_string(),
        ext: "tar.xz".to_string(),
    };

    let client = Client::new();
    let base = format!("{}/release", mock_server.uri());

    let listing = archive::fetch_all_versions(&client, &base, &platform)
        .await
        .unwrap();

    assert_eq!(listing.versions, vec!["5.2.0"]);
    assert_eq!(listing.skipped.len(), 1);
    assert_eq!(listing.skipped[0].series, "5.10");
    assert!(!listing.skipped[0].error.is_empty());
}

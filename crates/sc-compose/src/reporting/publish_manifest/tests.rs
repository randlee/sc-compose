use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::reporting::index::ReportIndexEntry;
use crate::reporting::output::ARCHIVE_ROOT_RELATIVE_PATH;

use super::archive::latest_archive_root;
use super::files::{artifact_publish_path, build_manifest_files};
use super::model::{PublishManifest, PublishManifestFile, PublishManifestReport};

static NEXT_TEMPORARY_DIRECTORY: AtomicU64 = AtomicU64::new(0);

#[test]
fn artifact_publish_path_rejects_parent_escaped_artifact() {
    let latest_report_root = Path::new("reports/latest").join("smoke");
    let publish_root = Path::new("reports").join("smoke");

    let error = artifact_publish_path(
        "smoke",
        Path::new("reports/latest/../escape.html"),
        &latest_report_root,
        &publish_root,
    )
    .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("invalid publish-manifest artifact path for smoke"));
    assert!(message.contains(&format!(
        "artifact must remain under {}",
        latest_report_root.display()
    )));
}

#[test]
fn artifact_publish_path_rejects_empty_remainder_at_report_root() {
    let latest_report_root = Path::new("reports/latest").join("smoke");
    let publish_root = Path::new("reports").join("smoke");

    let error = artifact_publish_path(
        "smoke",
        &latest_report_root,
        &latest_report_root,
        &publish_root,
    )
    .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("invalid publish-manifest artifact path for smoke"));
    assert!(message.contains(&format!(
        "artifact must remain under {}",
        latest_report_root.display()
    )));
    assert!(message.contains("path must not be empty"));
}

#[test]
fn artifact_publish_path_rejects_absolute_artifact_at_manifest_surface() {
    let latest_report_root = Path::new("reports/latest").join("smoke");
    let publish_root = Path::new("reports").join("smoke");
    let artifact = std::env::current_dir()
        .expect("current directory")
        .join("reports/latest/smoke/index.html");

    let error =
        artifact_publish_path("smoke", &artifact, &latest_report_root, &publish_root).unwrap_err();

    let message = error.to_string();
    assert!(message.contains("invalid publish-manifest artifact path for smoke"));
    assert!(message.contains(&format!(
        "artifact must remain under {}",
        latest_report_root.display()
    )));
    assert!(message.contains("prefix not found"));
}

#[test]
fn build_manifest_files_preserves_roles_and_publish_paths() {
    let entry = ReportIndexEntry {
        report_id: "smoke".to_owned(),
        kind: "smoke".to_owned(),
        required: true,
        status: Some("ok".to_owned()),
        produced_at: Some("2026-07-25T00:00:00Z".to_owned()),
        entrypoint: PathBuf::from("reports/latest/smoke/index.html"),
        metadata: PathBuf::from("reports/latest/smoke/metadata.json"),
        artifacts: vec![
            PathBuf::from("reports/latest/smoke/index.html"),
            PathBuf::from("reports/latest/smoke/metadata.json"),
            PathBuf::from("reports/latest/smoke/panels/chart.html"),
        ],
        missing_paths: Vec::new(),
    };

    let files = build_manifest_files(&entry).expect("manifest files");

    assert_eq!(files.len(), 3);
    assert_eq!(files[0].role, "entrypoint");
    assert_eq!(
        files[0].publish_to,
        PathBuf::from("reports/smoke/index.html")
    );
    assert_eq!(files[1].role, "metadata");
    assert_eq!(
        files[1].publish_to,
        PathBuf::from("reports/smoke/metadata.json")
    );
    assert_eq!(files[2].role, "artifact");
    assert_eq!(
        files[2].publish_to,
        PathBuf::from("reports/smoke/panels/chart.html")
    );
}

#[test]
fn manifest_serialization_preserves_forward_slash_paths() {
    let manifest = PublishManifest {
        generated_at: "2026-08-30T00:00:00Z".to_owned(),
        reports: vec![PublishManifestReport {
            report_id: "smoke".to_owned(),
            kind: "smoke".to_owned(),
            entrypoint: PathBuf::from(r"reports\latest\smoke\index.html"),
            archive_root: Some(PathBuf::from(r"reports\archive\2026-08-30\smoke")),
            files: vec![PublishManifestFile {
                role: "entrypoint".to_owned(),
                path: PathBuf::from(r"reports\latest\smoke\index.html"),
                publish_to: PathBuf::from(r"reports\smoke\index.html"),
            }],
        }],
    };

    let value = serde_json::to_value(manifest).expect("serialize manifest");

    assert_eq!(
        value["reports"][0]["entrypoint"],
        "reports/latest/smoke/index.html"
    );
    assert_eq!(
        value["reports"][0]["archive_root"],
        "reports/archive/2026-08-30/smoke"
    );
    assert_eq!(
        value["reports"][0]["files"][0]["path"],
        "reports/latest/smoke/index.html"
    );
    assert_eq!(
        value["reports"][0]["files"][0]["publish_to"],
        "reports/smoke/index.html"
    );
}

#[test]
fn latest_archive_root_selects_lexically_latest_archive_directory() {
    let root = temp_root("publish-manifest-latest-archive-root");
    create_dir(
        &root
            .join(ARCHIVE_ROOT_RELATIVE_PATH)
            .join("2026-07-14T01-00-00Z")
            .join("sc-lint"),
    );
    create_dir(
        &root
            .join(ARCHIVE_ROOT_RELATIVE_PATH)
            .join("2026-07-15T09-00-00Z")
            .join("sc-lint"),
    );

    let archive_root = latest_archive_root(&root, "sc-lint")
        .unwrap()
        .expect("archive root");

    assert_eq!(
        archive_root,
        PathBuf::from(ARCHIVE_ROOT_RELATIVE_PATH)
            .join("2026-07-15T09-00-00Z")
            .join("sc-lint")
    );
}

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let sequence = NEXT_TEMPORARY_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "sc-compose-{label}-{}-{nanos}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    root
}

fn create_dir(path: &Path) {
    fs::create_dir_all(path).expect("create dir");
}

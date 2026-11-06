use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::include::expand_includes;
use crate::{ComposeError, ComposePolicy, ConfiningRoot, DiagnosticCode};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

#[test]
fn binary_include_target_is_config_read_error() {
    let nonce = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "adr0026-d12-binary-include-{}-{timestamp}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("temporary root");
    fs::write(root.join("root.md.j2"), "@<binary.inc>\n").expect("root template");
    fs::write(root.join("binary.inc"), [b'v', 0xff, b'x']).expect("binary include");

    let error = expand_includes(
        root.join("root.md.j2"),
        &ConfiningRoot::new(&root).expect("confining root"),
        &ComposePolicy::default(),
    )
    .expect_err("binary target must not be rendered as text");

    assert!(
        matches!(error, ComposeError::Include(include) if include.code() == DiagnosticCode::ErrConfigRead)
    );
}

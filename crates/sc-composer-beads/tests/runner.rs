//! Direct-process runner contract coverage.

use std::path::PathBuf;

use sc_composer_beads::CommandSpec;

#[test]
fn command_spec_preserves_executable_argv_and_working_directory_separately() {
    let spec = CommandSpec {
        executable: PathBuf::from("/tools/bd"),
        args: vec![
            "cook".to_owned(),
            "formula with spaces.formula.toml".to_owned(),
            "--dry-run".to_owned(),
        ],
        working_directory: PathBuf::from("/workspace"),
    };

    assert_eq!(
        spec.argv(),
        vec![
            "/tools/bd",
            "cook",
            "formula with spaces.formula.toml",
            "--dry-run"
        ]
    );
    assert_eq!(spec.working_directory, PathBuf::from("/workspace"));
}

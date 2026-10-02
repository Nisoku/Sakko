//! Guard: every file in `Examples/` must parse and typecheck cleanly.

use std::path::{Path, PathBuf};

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Examples")
}

#[test]
fn every_example_parses_and_typechecks() {
    let dir = examples_dir();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read Examples/ at {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".sako"))
        .map(|e| e.path())
        .collect();
    entries.sort();

    assert!(
        !entries.is_empty(),
        "no .sako files found in {}",
        dir.display()
    );

    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(&path).unwrap();
        let report = match sakko::typecheck::check_source(&src) {
            Ok(r) => r,
            Err(e) => panic!("{name}: failed to parse: {e}"),
        };

        assert!(
            report.diagnostics.is_empty(),
            "{name}: expected no typecheck diagnostics, got:\n{}",
            report
                .diagnostics
                .iter()
                .map(|d| d.render())
                .collect::<String>()
        );
    }
}

//! Line origins of expanded template text (ADR-0026 D10).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::types::{ComposePolicy, ConfiningRoot};

use super::{ExpandedTemplate, SourceLocation, expand_includes};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "sc-compose-source-location-{label}-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self {
            root: root.canonicalize().unwrap(),
        }
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }

    fn confining_root(&self) -> ConfiningRoot {
        ConfiningRoot::new(&self.root).unwrap()
    }

    fn expand(&self, template: &Path) -> ExpandedTemplate {
        expand_includes(template, &self.confining_root(), &ComposePolicy::default()).unwrap()
    }

    /// Locate every line of the expanded text as `(file name, line)`.
    fn origins(&self, template: &Path, expanded: &ExpandedTemplate) -> Vec<(String, usize)> {
        let lines = expanded.text.split_inclusive('\n').count();
        (1..=lines)
            .map(|line| {
                let location = expanded
                    .source_location(
                        template,
                        &self.confining_root(),
                        &ComposePolicy::default(),
                        line,
                    )
                    .unwrap();
                let name = location
                    .path
                    .strip_prefix(&self.root)
                    .unwrap()
                    .display()
                    .to_string();
                (name, location.line)
            })
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn owned(pairs: &[(&str, usize)]) -> Vec<(String, usize)> {
    pairs
        .iter()
        .map(|(name, line)| {
            let path: PathBuf = Path::new(name).components().collect();
            (path.display().to_string(), *line)
        })
        .collect()
}

#[test]
fn every_expanded_line_maps_to_its_file_and_line() {
    let fixture = Fixture::new("nested");
    let main = fixture.write(
        "main.md",
        "---\nrequired_variables: []\n---\nm1\n@<a.md>\nm3\n",
    );
    fixture.write("a.md", "a1\n@<sub/b.md>\na3\n");
    fixture.write("sub/b.md", "b1\r\nb2\r\n");
    let expanded = fixture.expand(&main);
    assert_eq!(expanded.text, "m1\na1\nb1\r\nb2\r\na3\nm3\n");
    assert_eq!(
        fixture.origins(&main, &expanded),
        owned(&[
            ("main.md", 4),
            ("a.md", 1),
            ("sub/b.md", 1),
            ("sub/b.md", 2),
            ("a.md", 3),
            ("main.md", 6),
        ])
    );
    let location = expanded
        .source_location(
            &main,
            &fixture.confining_root(),
            &ComposePolicy::default(),
            3,
        )
        .unwrap();
    assert_eq!(
        location.include_chain,
        vec![
            fixture.root.join("main.md"),
            fixture.root.join("a.md"),
            fixture.root.join("sub/b.md"),
        ]
    );
}

#[test]
fn child_without_final_newline_keeps_the_line_it_starts() {
    let fixture = Fixture::new("no-final-newline");
    let main = fixture.write("main.md", "@<a.md>\nafter\n");
    fixture.write("a.md", "a1\na2");
    let expanded = fixture.expand(&main);
    // Lines joined by expansion keep the origin of the source that started
    // them; ADR-0026 D8 changes this layout separately.
    let lines = expanded.text.split_inclusive('\n').count();
    let origins = fixture.origins(&main, &expanded);
    assert_eq!(origins.len(), lines);
    assert_eq!(origins[0], ("a.md".to_owned(), 1));
    assert_eq!(origins[1], ("a.md".to_owned(), 2));
}

#[test]
fn conditional_include_lines_map_to_each_candidate() {
    let fixture = Fixture::new("conditional");
    let main = fixture.write(
        "main.md",
        "top\n@<{{ \"x.md\" if mode == \"x\" else \"y.md\" }}>\nend\n",
    );
    fixture.write("x.md", "x1\nx2\n");
    fixture.write("y.md", "y1\ny2\n");
    let expanded = fixture.expand(&main);
    assert_eq!(
        expanded.text,
        "top\n{% if mode == \"x\" %}x1\nx2\n{% else %}y1\ny2\n{% endif %}end\n"
    );
    // The directive's `{% if %}`, `{% else %}` and `{% endif %}` text is in
    // no source file, so a line it starts belongs to the source text that
    // follows it on that line: the candidate's first line, or the parent line
    // after the directive.
    assert_eq!(
        fixture.origins(&main, &expanded),
        owned(&[
            ("main.md", 1),
            ("x.md", 1),
            ("x.md", 2),
            ("y.md", 1),
            ("y.md", 2),
            ("main.md", 3),
        ])
    );
}

/// Locate `(file, line, column)` for one line and column of expanded text.
fn locate_at(
    fixture: &Fixture,
    template: &Path,
    expanded: &ExpandedTemplate,
    line: usize,
    column: usize,
) -> (String, usize, Option<usize>) {
    let (location, column) = expanded
        .source_location_at(
            template,
            &fixture.confining_root(),
            &ComposePolicy::default(),
            line,
            Some(column),
        )
        .unwrap();
    let name = location
        .path
        .strip_prefix(&fixture.root)
        .unwrap()
        .display()
        .to_string();
    (name, location.line, column)
}

#[test]
fn parent_line_after_a_conditional_include_keeps_its_line_and_column() {
    // A candidate without a final newline gets the directive's terminator
    // (ADR-0026 D8), so `{% endif %}` starts the parent's next line.
    for (x, y) in [("x1", "y1"), ("x1\n", "y1\n"), ("x1\r\n", "y1")] {
        let fixture = Fixture::new("conditional-following");
        let main = fixture.write(
            "main.md",
            "T\n@<{{ \"x.md\" if flag else \"y.md\" }}>\n{{ 1 + }}\n",
        );
        fixture.write("x.md", x);
        fixture.write("y.md", y);
        let expanded = fixture.expand(&main);
        let last = expanded.text.split_inclusive('\n').count();
        let last_line = expanded.text.split_inclusive('\n').next_back().unwrap();
        assert_eq!(last_line, "{% endif %}{{ 1 + }}\n", "{x:?} {y:?}");
        let prefix = "{% endif %}".len();
        // `}}` of `{{ 1 + }}` is source column 8.
        assert_eq!(
            locate_at(&fixture, &main, &expanded, last, prefix + 8),
            ("main.md".to_owned(), 3, Some(8)),
            "{x:?} {y:?}"
        );
        // A column inside the synthetic text locates the directive line.
        assert_eq!(
            locate_at(&fixture, &main, &expanded, last, 3),
            ("main.md".to_owned(), 2, None),
            "{x:?} {y:?}"
        );
        // Each candidate's first line follows `{% if %}` or `{% else %}`.
        let if_width = "{% if flag %}".len();
        assert_eq!(
            locate_at(&fixture, &main, &expanded, 2, if_width + 1),
            ("x.md".to_owned(), 1, Some(1)),
            "{x:?} {y:?}"
        );
        assert_eq!(
            locate_at(&fixture, &main, &expanded, 3, "{% else %}".len() + 2),
            ("y.md".to_owned(), 1, Some(2)),
            "{x:?} {y:?}"
        );
    }
}

#[test]
fn nested_conditional_include_lines_keep_their_own_files() {
    let fixture = Fixture::new("conditional-nested");
    let main = fixture.write("main.md", "top\n@<mid.md>\nend\n");
    fixture.write(
        "mid.md",
        "m1\n@<{{ \"leaf.md\" if deep else \"flat.md\" }}>\nm3\n",
    );
    fixture.write("leaf.md", "g1\ng2");
    fixture.write("flat.md", "f1");
    let expanded = fixture.expand(&main);
    assert_eq!(
        expanded.text,
        "top\nm1\n{% if deep %}g1\ng2\n{% else %}f1\n{% endif %}m3\nend\n"
    );
    assert_eq!(
        fixture.origins(&main, &expanded),
        owned(&[
            ("main.md", 1),
            ("mid.md", 1),
            ("leaf.md", 1),
            ("leaf.md", 2),
            ("flat.md", 1),
            ("mid.md", 3),
            ("main.md", 3),
        ])
    );
    let location = expanded
        .source_location(
            &main,
            &fixture.confining_root(),
            &ComposePolicy::default(),
            6,
        )
        .unwrap();
    assert_eq!(
        location.include_chain,
        vec![fixture.root.join("main.md"), fixture.root.join("mid.md")]
    );
}

#[test]
fn template_without_includes_is_located_without_the_filesystem() {
    // Callers may build an ExpandedTemplate for a template that is not on
    // disk; it is located from its own source text.
    let path = PathBuf::from("stored-template.txt");
    let source = "---\nrequired_variables: []\n---\nl1\nl2\n";
    let expanded = ExpandedTemplate {
        text: "l1\nl2\n".to_owned(),
        resolved_files: vec![path.clone()],
        frontmatters: Vec::new(),
        include_chains: BTreeMap::from([(path.clone(), Vec::new())]),
        source_texts: BTreeMap::from([(path.clone(), source.to_owned())]),
        composition_fingerprint: None,
    };
    let fixture = Fixture::new("stored");
    let locate = |line| {
        expanded.source_location(
            &path,
            &fixture.confining_root(),
            &ComposePolicy::default(),
            line,
        )
    };
    assert_eq!(
        locate(2),
        Some(SourceLocation {
            path: path.clone(),
            line: 5,
            include_chain: Vec::new(),
        })
    );
    assert_eq!(locate(0), None);
    assert_eq!(locate(3), None);
}

#[test]
fn changed_sources_are_not_located() {
    let fixture = Fixture::new("stale");
    let main = fixture.write("main.md", "m1\n@<a.md>\n");
    fixture.write("a.md", "a1\n");
    let expanded = fixture.expand(&main);
    // The sources no longer expand to the same text, so their lines cannot
    // be trusted to match the text that was rendered.
    fixture.write("a.md", "a1\nnew\n");
    assert_eq!(
        expanded.source_location(
            &main,
            &fixture.confining_root(),
            &ComposePolicy::default(),
            2
        ),
        None
    );
}

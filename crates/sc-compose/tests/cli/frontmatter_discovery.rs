//! The default render path uses frontmatter-aware discovery, without --strict.
use crate::support::*;
use serde_json::Value;

#[test]
fn render_frontmatter_tag_blocks_without_undeclared_token_warnings() {
    for body in [
        "{% autoescape false %}hi {{ x }}{% endautoescape %}",
        "{% raw %}{{ hidden }}{% endraw %}hi {{ x }}",
        "{% filter upper %}hi {{ x }}{% endfilter %}",
    ] {
        let fixture = TempFixture::new("frontmatter-tag-render");
        write_file(
            &fixture.path.join("template.md.j2"),
            &format!("---\nrequired_variables: [x]\n---\n{body}"),
        );
        let output = sc_compose()
            .args(["render", "--root"])
            .arg(&fixture.path)
            .args(["--file", "template.md.j2", "--var", "x=a<b", "--json"])
            .bounded_output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(
            value["diagnostics"].as_array().unwrap().is_empty(),
            "{body}: {value}"
        );
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("with warnings"),
            "{output:?}"
        );
    }
}

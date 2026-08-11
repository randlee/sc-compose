mod capability;
mod pass_input;
mod schema;

pub(crate) use capability::command_wants_json;
pub(crate) use pass_input::{
    PassInputArgs, filtered_args_for_clap, parse_cli_from, parse_pass_inputs, parse_var,
    raw_args_want_json,
};
pub(crate) use schema::*;

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use clap::Parser;

    use super::*;

    fn os_args(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    fn json_capability(args: &[&str]) -> bool {
        let cli = Cli::try_parse_from(args).expect("valid CLI arguments");
        command_wants_json(&cli.command)
    }

    #[test]
    fn parse_var_preserves_first_separator_and_rejects_missing_separator() {
        assert_eq!(
            parse_var("key=value=with=separators"),
            Ok(("key".to_owned(), "value=with=separators".to_owned()))
        );
        assert_eq!(
            parse_var("missing-separator"),
            Err("expected key=value".to_owned())
        );
    }

    #[test]
    fn parse_pass_inputs_accepts_mixed_syntax_and_preserves_order() {
        let parsed = parse_pass_inputs(
            os_args(&[
                "sc-compose",
                "render",
                "--pass",
                "1",
                "--var",
                "first=one",
                "--var-file=one.json",
                "--pass=2",
                "--var=second=two",
                "--var-file",
                "two.yaml",
            ]),
            "render",
        )
        .expect("pass groups parse");

        assert_eq!(
            parsed,
            vec![
                PassInputArgs {
                    pass_number: 1,
                    vars: vec![("first".to_owned(), "one".to_owned())],
                    var_files: vec!["one.json".to_owned()],
                },
                PassInputArgs {
                    pass_number: 2,
                    vars: vec![("second".to_owned(), "two".to_owned())],
                    var_files: vec!["two.yaml".to_owned()],
                },
            ]
        );
    }

    #[test]
    fn parse_pass_inputs_rejects_malformed_and_misplaced_arguments() {
        assert_eq!(
            parse_pass_inputs(os_args(&["sc-compose", "render", "--pass"]), "render"),
            Err("--pass requires a numeric pass number".to_owned())
        );
        assert_eq!(
            parse_pass_inputs(
                os_args(&["sc-compose", "render", "--pass", "1", "--var"]),
                "render"
            ),
            Err("--var requires key=value".to_owned())
        );
        assert_eq!(
            parse_pass_inputs(
                os_args(&["sc-compose", "render", "--var=orphan=value"]),
                "render"
            ),
            Err("--var must appear after --pass".to_owned())
        );
        assert!(
            parse_pass_inputs(os_args(&["sc-compose", "render", "--pass=bad"]), "render")
                .unwrap_err()
                .starts_with("invalid pass number `bad`:")
        );
    }

    #[test]
    fn parse_pass_inputs_rejects_misplaced_var_file_arguments() {
        assert_eq!(
            parse_pass_inputs(
                os_args(&["sc-compose", "render", "--var-file", "orphan.json"]),
                "render"
            ),
            Err("--var-file must appear after --pass".to_owned())
        );
        assert_eq!(
            parse_pass_inputs(
                os_args(&["sc-compose", "render", "--var-file=orphan.json"]),
                "render"
            ),
            Err("--var-file must appear after --pass".to_owned())
        );
    }

    #[test]
    fn filtered_args_for_clap_removes_only_pass_scoped_arguments() {
        let filtered = filtered_args_for_clap(os_args(&[
            "sc-compose",
            "render",
            "--pass=1",
            "--var=first=one",
            "--file",
            "template.j2",
            "--pass",
            "2",
            "--var",
            "second=two",
            "--json",
        ]));

        assert_eq!(
            filtered,
            os_args(&["sc-compose", "render", "--file", "template.j2", "--json",])
        );
    }

    #[test]
    fn json_capability_covers_commands_and_nested_subcommands() {
        let json_commands: &[&[&str]] = &[
            &["sc-compose", "render", "--json"],
            &["sc-compose", "resolve", "--json"],
            &["sc-compose", "validate", "--json"],
            &["sc-compose", "verify", "deployed.txt", "--json"],
            &[
                "sc-compose",
                "extract",
                "template.xml.j2",
                "rendered.xml",
                "--json",
            ],
            &["sc-compose", "template-init", "template.txt", "--json"],
            &[
                "sc-compose",
                "frontmatter-init",
                "--file",
                "template.txt",
                "--json",
            ],
            &["sc-compose", "init", "--json"],
            &["sc-compose", "observability-health", "--json"],
            &["sc-compose", "examples", "--json"],
            &["sc-compose", "examples", "list", "--json"],
            &["sc-compose", "templates", "--json"],
            &["sc-compose", "templates", "list", "--json"],
            &["sc-compose", "templates", "add", "pack", "--json"],
            &["sc-compose", "reports", "init", "--json"],
            &[
                "sc-compose",
                "reports",
                "smoke",
                "--fixture",
                "fixture",
                "--vars",
                "vars",
                "--json",
            ],
            &[
                "sc-compose",
                "reports",
                "finalize",
                "--report-id",
                "id",
                "--kind",
                "kind",
                "--entrypoint",
                "report.html",
                "--json",
            ],
            &[
                "sc-compose",
                "reports",
                "render-spec",
                "--spec",
                "spec.toml",
                "--json",
            ],
            &["sc-compose", "reports", "index", "--json"],
            &["sc-compose", "reports", "verify", "--json"],
            &["sc-compose", "reports", "publish-manifest", "--json"],
            &[
                "sc-compose",
                "report-render-many",
                "--id",
                "id",
                "--glob",
                "*.txt",
                "--output-dir",
                "out",
                "--json",
            ],
            &["sc-compose", "report-catalog", "--json"],
        ];

        for args in json_commands {
            assert!(
                json_capability(args),
                "expected JSON capability for {args:?}"
            );
        }

        assert!(!json_capability(&["sc-compose", "render"]));
        assert!(!json_capability(&["sc-compose", "examples"]));
        assert!(!json_capability(&["sc-compose", "templates", "list"]));
    }
}

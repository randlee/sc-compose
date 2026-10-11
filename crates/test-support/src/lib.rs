// Shared test-only reference model. No shell command evaluation occurs here.
// Production inputs are UTF-8 strings without NUL subprocess arguments; arbitrary
// hex bytes are covered independently by decoder vectors, not production IDs.
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

fn select_python(
    mut probe: impl FnMut(&str) -> Result<[u64; 2], String>,
) -> Result<&'static str, String> {
    let mut failures = Vec::new();
    for candidate in ["python3", "python"] {
        match probe(candidate) {
            Ok(version) if version >= [3, 9] => return Ok(candidate),
            Ok(version) => failures.push(format!(
                "{candidate}: Python {}.{} is too old",
                version[0], version[1]
            )),
            Err(error) => failures.push(format!("{candidate}: {error}")),
        }
    }
    Err(format!(
        "Recovery quoting tests require Python >= 3.9 via python3 or python: {}",
        failures.join("; ")
    ))
}

fn python() -> &'static str {
    static PYTHON: OnceLock<&'static str> = OnceLock::new();
    PYTHON.get_or_init(|| {
        select_python(|candidate| {
            let output = Command::new(candidate)
                .args([
                    "-c",
                    "import json, sys; print(json.dumps(list(sys.version_info[:2])))",
                ])
                .output()
                .map_err(|error| error.to_string())?;
            if !output.status.success() {
                return Err(format!(
                    "version probe failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            serde_json::from_slice(&output.stdout)
                .map_err(|error| format!("invalid version probe: {error}"))
        })
        .unwrap_or_else(|error| panic!("{error}"))
    })
}

fn decode(arguments: &str) -> Result<Vec<Vec<u8>>, String> {
    let mut child = Command::new(python())
        .args(["-I", "-c", include_str!("shell_literal_decoder.py")])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("resolved Python interpreter must start");
    let mut input = child.stdin.take().expect("piped Python stdin");
    input
        .write_all(&serde_json::to_vec(arguments).expect("literal JSON input"))
        .expect("write Python decoder input");
    drop(input);
    let output = child.wait_with_output().expect("Python decoder completion");
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid decoder bytes: {error}"))
}

/// Assert that a shell-quoted argument string decodes to exactly `expected`.
///
/// # Panics
///
/// Panics when an expected argument contains NUL, the quoted text is rejected
/// by the literal decoder, or the decoded arguments differ from `expected`.
pub fn assert_round_trip(arguments: &str, expected: &[&str]) {
    assert!(
        expected.iter().all(|argument| !argument.contains('\0')),
        "NUL is not a subprocess argument"
    );
    let actual = decode(arguments)
        .unwrap_or_else(|error| panic!("literal decoder rejected {arguments:?}: {error}"));
    let expected: Vec<Vec<u8>> = expected
        .iter()
        .map(|argument| argument.as_bytes().to_vec())
        .collect();
    assert_eq!(actual, expected, "quoted arguments: {arguments:?}");
}

#[test]
fn python_resolver_falls_back_and_requires_supported_version() {
    let mut calls = Vec::new();
    let selected = select_python(|name| {
        calls.push(name.to_owned());
        Ok(if name == "python3" { [3, 8] } else { [3, 11] })
    })
    .unwrap();
    assert_eq!(selected, "python");
    assert_eq!(calls, ["python3", "python"]);
    assert_eq!(
        select_python(|name| if name == "python3" {
            Err("not found".into())
        } else {
            Ok([3, 9])
        })
        .unwrap(),
        "python"
    );
    let mut probes = 0;
    assert_eq!(
        select_python(|_| {
            probes += 1;
            Ok([3, 9])
        })
        .unwrap(),
        "python3"
    );
    assert_eq!(probes, 1);
    let unavailable = select_python(|_| Err("not found".into())).unwrap_err();
    assert!(
        unavailable.contains("python3: not found; python: not found"),
        "{unavailable}"
    );
    let error = select_python(|_| Ok([2, 7])).unwrap_err();
    assert!(
        error.contains("Python >= 3.9") && error.contains("python3") && error.contains("python"),
        "{error}"
    );
}

#[test]
fn literal_decoder_has_independent_byte_vectors() {
    assert_eq!(
        decode(r#"'' ' ' 'trail\' 'a'"'"'b'"#).unwrap(),
        vec![
            b"".to_vec(),
            b" ".to_vec(),
            b"trail\\".to_vec(),
            b"a'b".to_vec()
        ]
    );
    assert_eq!(
        decode(r"$'\x00\x7F\x80\xFF'").unwrap(),
        vec![vec![0, 127, 128, 255]]
    );
    assert_eq!(
        decode(r"$'\u2028\u2029\U0001F642'").unwrap(),
        vec![vec![
            0xe2, 0x80, 0xa8, 0xe2, 0x80, 0xa9, 0xf0, 0x9f, 0x99, 0x82
        ]]
    );
    assert_eq!(
        decode(r"$'a\'b\\' 'café'").unwrap(),
        vec![b"a'b\\".to_vec(), vec![99, 97, 102, 0xc3, 0xa9]]
    );
    assert_eq!(
        decode(r"'left'$'\x20''right'").unwrap(),
        vec![b"left right".to_vec()]
    );
    assert_round_trip(
        "'$(literal);$HOME`literal`*?[]'",
        &["$(literal);$HOME`literal`*?[]"],
    );
}

#[test]
fn literal_decoder_rejects_nonliteral_and_malformed_syntax() {
    for input in [
        "$HOME",
        "$(literal)",
        "`literal`",
        "plain",
        "'a';'b'",
        "'a'|'b'",
        "'a'\n'b'",
        "\"$HOME\"",
        "'unfinished",
        r"$'\q'",
        r"$'\x0'",
        r"$'\uD800'",
        r"$'\U00110000'",
    ] {
        assert!(
            decode(input).is_err(),
            "accepted nonliteral input {input:?}"
        );
    }
}

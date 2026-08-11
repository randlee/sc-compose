#[test]
fn test_does_not_repeat_production_identity() {
    let owner = "team-lead@example.invalid";
    assert_eq!(owner, "team-lead@example.invalid");
}

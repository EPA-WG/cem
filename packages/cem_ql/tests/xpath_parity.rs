#[test]
fn xpath_parity_target_is_registered() {
    assert_eq!(cem_ql::VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn trybuild_suite() {
    let t = trybuild::TestCases::new();
    t.pass("tests/cases/pass_async.rs");
    t.pass("tests/cases/pass_sync.rs");
    t.pass("tests/cases/pass_context.rs");
    t.pass("tests/cases/pass_method_self.rs");
    t.compile_fail("tests/cases/fail_adjust_blocking.rs");
    t.compile_fail("tests/cases/fail_context_ident.rs");
    t.compile_fail("tests/cases/fail_method_self_context.rs");
    t.compile_fail("tests/cases/fail_method_mut_context.rs");
    t.compile_fail("tests/cases/fail_context_value_self.rs");
}

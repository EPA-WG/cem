use cem_ml::benchmark::BenchmarkBudget;

#[test]
fn selector_benchmark_target_uses_shared_budget_contract() {
    let budget = BenchmarkBudget::default_ac_n_1();
    assert!(budget.effective_budget() >= budget.budget);
}

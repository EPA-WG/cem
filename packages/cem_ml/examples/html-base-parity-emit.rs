//! Explicit JSON scalar test-evidence export; retained document owners stay native.
#[path = "../tests/support/html_base_fixture.rs"]
mod fixture;
#[path = "../tests/support/html_execution_inventory_fixture.rs"]
mod inventory;
#[path = "../tests/support/html_base_freeze_trace.rs"]
mod trace;

fn main() {
    let result = match std::env::args().nth(1).as_deref() {
        None => fixture::run_cases(),
        Some("--confirm-observations") => {
            let observations =
                serde_json::from_reader::<_, Vec<fixture::BrowserObservation>>(std::io::stdin())
                    .unwrap();
            fixture::run_observed_cases(Some(&observations))
        }
        Some("--verify-freeze-trace") => {
            let observations =
                serde_json::from_reader::<_, Vec<trace::TraceObservation>>(std::io::stdin())
                    .unwrap();
            trace::verify_trace(&observations)
        }
        Some("--verify-execution-inventory") => {
            let observations = serde_json::from_reader::<_, Vec<inventory::InventoryObservation>>(
                std::io::stdin(),
            )
            .unwrap();
            inventory::verify(&observations)
        }
        other => panic!("unknown fixture control mode {other:?}"),
    };
    println!("{}", serde_json::to_string(&result).unwrap());
}

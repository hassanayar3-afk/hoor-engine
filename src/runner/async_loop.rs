use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::runner::{CycleError, HoorRunner};

pub async fn run_until_shutdown(
    mut runner: HoorRunner,
    shutdown: Arc<AtomicBool>,
) -> Result<(), CycleError> {
    while !shutdown.load(Ordering::Acquire) {
        match runner.run_one_cycle() {
            Ok(outcome) => {
                eprintln!("cycle: {:?}", outcome);
            }
            Err(CycleError::Exchange(e)) => {
                eprintln!("exchange error (continuing): {e}");
            }
            Err(CycleError::Halted(h)) => {
                eprintln!("halted: {:?}", h);
            }
        }
        tokio::time::sleep(runner.config.cycle_interval).await;
    }
    Ok(())
}

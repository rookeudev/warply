//! Idempotent cleanup: stop only Warply, remove only its rules, then its service copy.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    Tunnel(String),
    Protection(String),
    Storage(String),
}
impl Failure {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Tunnel(_) => 20,
            Self::Protection(_) => 21,
            Self::Storage(_) => 22,
        }
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tunnel(error) => write!(f, "Tunnel shutdown failed: {error}"),
            Self::Protection(error) => write!(f, "Protection cleanup failed: {error}"),
            Self::Storage(error) => write!(f, "The tunnel is stopped, but encrypted service storage could not be removed: {error}"),
        }
    }
}
fn execute(
    stop: impl FnOnce() -> Result<(), String>,
    clear: impl FnOnce() -> Result<(), String>,
    remove: impl FnOnce() -> Result<(), String>,
) -> Result<(), Failure> {
    let tunnel = stop();
    let protection = clear();
    let storage = if tunnel.is_ok() { remove() } else { Ok(()) };
    tunnel.map_err(Failure::Tunnel)?;
    protection.map_err(Failure::Protection)?;
    storage.map_err(Failure::Storage)
}
pub fn run() -> Result<(), Failure> {
    execute(
        || {
            crate::tunnel::backend()
                .disconnect()
                .map_err(|e| e.to_string())
        },
        crate::guard::disable,
        crate::storage::remove_machine_service_profile,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_runs_protection_after_stop_failure_but_retains_needed_service_copy() {
        use std::cell::Cell;
        let cleared = Cell::new(false);
        let result = execute(
            || Err("stop".into()),
            || {
                cleared.set(true);
                Ok(())
            },
            || panic!("running service copy must stay"),
        );
        assert_eq!(result, Err(Failure::Tunnel("stop".into())));
        assert!(cleared.get());
    }
    #[test]
    fn partial_failures_are_distinct_and_storage_cleanup_still_runs() {
        use std::cell::Cell;
        let removed = Cell::new(false);
        let result = execute(
            || Ok(()),
            || Err("guard".into()),
            || {
                removed.set(true);
                Ok(())
            },
        );
        assert_eq!(result.expect_err("failure").exit_code(), 21);
        assert!(removed.get());
        assert_eq!(
            execute(|| Ok(()), || Ok(()), || Err("storage".into()))
                .expect_err("failure")
                .exit_code(),
            22
        );
        for _ in 0..2 {
            assert_eq!(execute(|| Ok(()), || Ok(()), || Ok(())), Ok(()));
        }
    }
}

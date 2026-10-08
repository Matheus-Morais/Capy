use std::{sync::{atomic::{AtomicBool, AtomicU64, Ordering}, Mutex}, time::{Duration, Instant}};

#[derive(Default)]
pub struct Service { pending: AtomicBool, revision: AtomicU64, last: Mutex<Option<Instant>> }
impl Service {
    pub fn request(&self) -> Result<(), String> {
        let mut last=self.last.lock().map_err(|_|"Atualização indisponível.")?;
        if last.is_some_and(|at|at.elapsed()<Duration::from_secs(15)) {
            return Err("Aguarde 15 segundos entre pedidos de atualização.".into());
        }
        *last=Some(Instant::now());self.pending.store(true,Ordering::Release);self.revision.fetch_add(1,Ordering::Release);Ok(())
    }
    pub fn take(&self) -> bool { self.pending.swap(false,Ordering::AcqRel) }
    pub fn revision(&self) -> u64 { self.revision.load(Ordering::Acquire) }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn coalesces_and_limits_refresh_requests() {
        let service=Service::default();assert!(!service.take());assert!(service.request().is_ok());
        assert!(service.request().is_err());assert!(service.take());assert!(!service.take());
        assert_eq!(service.revision(),1);
    }
}

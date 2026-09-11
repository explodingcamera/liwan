use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::{
    app::SqlitePool,
    utils::hash::{onboarding_token, onboarding_token_matches},
};

#[derive(Clone)]
pub struct LiwanOnboarding {
    token: Arc<Mutex<Option<String>>>,
}

impl LiwanOnboarding {
    pub fn try_new(pool: &SqlitePool) -> Result<Self> {
        let onboarding = {
            tracing::debug!("Checking if an onboarding token needs to be generated");
            let conn = pool.get()?;
            let onboarded = conn.prepare("select 1 from users limit 1")?.exists([])?;
            (!onboarded).then(onboarding_token)
        };

        Ok(Self { token: Arc::new(Mutex::new(onboarding)) })
    }

    /// Get the onboarding token, if it exists
    pub fn token(&self) -> Option<String> {
        self.token.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    /// Clear the onboarding token to prevent it from being used again
    pub fn clear(&self) {
        *self.token.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }

    /// Complete setup once when the supplied token is valid.
    pub fn complete_setup(&self, supplied_token: &str, setup: impl FnOnce() -> Result<()>) -> Result<bool> {
        let mut token = self.token.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !token.as_deref().is_some_and(|token| onboarding_token_matches(token, supplied_token)) {
            return Ok(false);
        }

        setup()?;
        *token = None;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn complete_allows_only_one_setup() {
        let onboarding = LiwanOnboarding { token: Arc::new(Mutex::new(Some("token".to_string()))) };
        let setup_count = Arc::new(AtomicUsize::new(0));
        let attempts = (0..2)
            .map(|_| {
                let onboarding = onboarding.clone();
                let setup_count = setup_count.clone();
                std::thread::spawn(move || {
                    onboarding
                        .complete_setup("token", || {
                            setup_count.fetch_add(1, Ordering::Relaxed);
                            Ok(())
                        })
                        .unwrap()
                })
            })
            .map(|attempt| attempt.join().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(attempts.iter().filter(|completed| **completed).count(), 1);
        assert_eq!(setup_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn complete_keeps_token_when_setup_fails() {
        let onboarding = LiwanOnboarding { token: Arc::new(Mutex::new(Some("token".to_string()))) };

        assert!(onboarding.complete_setup("token", || anyhow::bail!("failed")).is_err());
        assert_eq!(onboarding.token().as_deref(), Some("token"));
    }
}

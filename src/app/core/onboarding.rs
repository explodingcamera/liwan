use arc_swap::ArcSwapOption;
use eyre::Result;
use std::sync::Arc;

use crate::{app::SqlitePool, utils::hash::onboarding_token};

#[derive(Clone)]
pub struct LiwanOnboarding {
    token: Arc<ArcSwapOption<String>>,
}

impl LiwanOnboarding {
    pub fn try_new(pool: &SqlitePool) -> Result<Self> {
        let onboarding = {
            tracing::debug!("Checking if an onboarding token needs to be generated");
            let conn = pool.get()?;
            let onboarded = conn.prepare("select 1 from users limit 1")?.exists([])?;
            ArcSwapOption::new(onboarded.then(|| onboarding_token().into()))
        };

        Ok(Self { token: onboarding.into() })
    }

    /// Get the onboarding token, if it exists
    pub fn token(&self) -> Result<Option<String>> {
        Ok((self.token.load()).as_ref().map(|v| (**v).clone()))
    }

    /// Clear the onboarding token to prevent it from being used again
    pub fn clear(&self) -> Result<()> {
        self.token.store(None);
        Ok(())
    }
}

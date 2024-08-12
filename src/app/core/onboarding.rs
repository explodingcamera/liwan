use std::sync::Arc;

use crossbeam::sync::ShardedLock;
use eyre::Result;

use crate::{app::SqlitePool, utils::hash::onboarding_token};

#[derive(Clone)]
pub(crate) struct LiwanOnboarding {
    token: Arc<ShardedLock<Option<String>>>,
}

impl LiwanOnboarding {
    pub(crate) fn try_new(pool: SqlitePool) -> Result<Self> {
        let onboarding = {
            tracing::debug!("Checking if an onboarding token needs to be generated");
            let conn = pool.get()?;
            let mut stmt = conn.prepare("select 1 from users limit 1")?;
            ShardedLock::new(if stmt.exists([])? { None } else { Some(onboarding_token()) })
        };

        Ok(Self { token: onboarding.into() })
    }

    pub(crate) fn token(&self) -> Result<Option<String>> {
        Ok(self
            .token
            .read()
            .map_err(|_| eyre::eyre!("Failed to acquire onboarding token read lock"))?
            .as_ref()
            .cloned())
    }

    pub(crate) fn clear(&self) -> Result<()> {
        let mut onboarding =
            self.token.write().map_err(|_| eyre::eyre!("Failed to acquire onboarding token write lock"))?;
        *onboarding = None;
        Ok(())
    }
}

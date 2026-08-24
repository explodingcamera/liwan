use crate::{
    app::{SqlitePool, models::UserRole},
    utils::validate,
};
mod http;
mod providers;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use providers::{FlowSecret, Provider};

/// An external authentication provider supported by Liwan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExternalAuthProvider {
    Oidc,
    Google,
    Microsoft,
    Github,
}

impl ExternalAuthProvider {
    fn as_str(self) -> &'static str {
        match self {
            Self::Oidc => "oidc",
            Self::Google => "google",
            Self::Microsoft => "microsoft",
            Self::Github => "github",
        }
    }
}

impl TryFrom<String> for ExternalAuthProvider {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        match value.as_str() {
            "oidc" => Ok(Self::Oidc),
            "google" => Ok(Self::Google),
            "microsoft" => Ok(Self::Microsoft),
            "github" => Ok(Self::Github),
            _ => bail!("invalid external authentication provider"),
        }
    }
}

/// The persisted configuration for the active external authentication provider.
#[derive(Clone, PartialEq, Eq)]
pub struct ExternalAuthSettings {
    pub enabled: bool,
    pub provider: ExternalAuthProvider,
    pub display_name: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub issuer_url: Option<String>,
    pub allowed_domain: Option<String>,
    pub allowed_organization: Option<String>,
    pub allow_user_creation: bool,
}

impl std::fmt::Debug for ExternalAuthSettings {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExternalAuthSettings")
            .field("enabled", &self.enabled)
            .field("provider", &self.provider)
            .field("display_name", &self.display_name)
            .field("client_id", &self.client_id)
            .field("client_secret_configured", &self.client_secret.is_some())
            .field("issuer_url", &self.issuer_url)
            .field("allowed_domain", &self.allowed_domain)
            .field("allowed_organization", &self.allowed_organization)
            .field("allow_user_creation", &self.allow_user_creation)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExternalIdentity {
    provider_key: String,
    subject: String,
    username_hint: Option<String>,
}

const FLOW_LIFETIME: Duration = Duration::from_secs(10 * 60);
const PROVIDER_CACHE_LIFETIME: Duration = Duration::from_secs(5 * 60);

/// A newly created provider authorization request.
#[derive(Debug)]
pub struct ExternalAuthStart {
    pub authorization_url: url::Url,
    pub state: String,
}

/// The local account and redirect resolved by a successful provider callback.
#[derive(Debug)]
pub struct ExternalAuthLogin {
    pub username: String,
    pub return_to: String,
}

struct PendingFlow {
    provider: Arc<Provider>,
    secret: FlowSecret,
    settings_fingerprint: blake3::Hash,
    return_to: String,
    expires_at: Instant,
}

struct CachedProvider {
    settings_fingerprint: blake3::Hash,
    provider: Arc<Provider>,
    created_at: Instant,
}

struct RuntimeState {
    http: reqwest::Client,
    redirect_url: String,
    provider: Mutex<Option<CachedProvider>>,
    flows: Mutex<HashMap<String, PendingFlow>>,
}

/// Manages external provider settings, login flows, and local identities.
#[derive(Clone)]
pub struct LiwanExternalAuth {
    pool: SqlitePool,
    runtime: Arc<RuntimeState>,
}

impl LiwanExternalAuth {
    /// Creates the external authentication service and its restricted HTTP client.
    pub fn try_new(pool: SqlitePool, base_url: &str) -> Result<Self> {
        let redirect_url = url::Url::parse(base_url)?.join("/api/dashboard/auth/external/callback")?.to_string();
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            pool,
            runtime: Arc::new(RuntimeState {
                http,
                redirect_url,
                provider: Mutex::new(None),
                flows: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// Returns the current external authentication settings.
    pub fn settings(&self) -> Result<ExternalAuthSettings> {
        let conn = self.pool.get()?;
        let (
            enabled,
            provider,
            display_name,
            client_id,
            client_secret,
            issuer_url,
            allowed_domain,
            allowed_organization,
            allow_user_creation,
        ) = conn.query_row(
            r"select enabled, provider, display_name, client_id, client_secret, issuer_url,
                      allowed_domain, allowed_organization, allow_user_creation
               from external_auth_settings where id = 1",
            [],
            |row| {
                let provider: String = row.get("provider")?;
                Ok((
                    row.get("enabled")?,
                    provider,
                    row.get("display_name")?,
                    row.get("client_id")?,
                    row.get("client_secret")?,
                    row.get("issuer_url")?,
                    row.get("allowed_domain")?,
                    row.get("allowed_organization")?,
                    row.get("allow_user_creation")?,
                ))
            },
        )?;
        Ok(ExternalAuthSettings {
            enabled,
            provider: provider.try_into()?,
            display_name,
            client_id,
            client_secret,
            issuer_url,
            allowed_domain,
            allowed_organization,
            allow_user_creation,
        })
    }

    /// Validates and replaces the external authentication settings.
    pub async fn update_settings(&self, settings: &ExternalAuthSettings) -> Result<()> {
        let settings = normalize_settings(settings);
        let provider = if settings.enabled { Some(Arc::new(self.build_provider(&settings).await?)) } else { None };
        self.persist_settings(&settings)?;
        *self.runtime.provider.lock().expect("external auth provider lock poisoned") =
            provider.map(|provider| CachedProvider {
                settings_fingerprint: settings_fingerprint(&settings),
                provider,
                created_at: Instant::now(),
            });
        Ok(())
    }

    fn persist_settings(&self, settings: &ExternalAuthSettings) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            r"update external_auth_settings set
                   enabled = :enabled,
                   provider = :provider,
                   display_name = :display_name,
                   client_id = :client_id,
                   client_secret = :client_secret,
                   issuer_url = :issuer_url,
                   allowed_domain = :allowed_domain,
                   allowed_organization = :allowed_organization,
                   allow_user_creation = :allow_user_creation
               where id = 1",
            rusqlite::named_params! {
                ":enabled": settings.enabled,
                ":provider": settings.provider.as_str(),
                ":display_name": settings.display_name,
                ":client_id": settings.client_id,
                ":client_secret": settings.client_secret,
                ":issuer_url": settings.issuer_url,
                ":allowed_domain": settings.allowed_domain,
                ":allowed_organization": settings.allowed_organization,
                ":allow_user_creation": settings.allow_user_creation,
            },
        )?;
        Ok(())
    }

    /// Returns the callback URL that must be registered with the provider.
    pub fn callback_url(&self) -> &str {
        &self.runtime.redirect_url
    }

    /// Starts a short-lived external login flow.
    pub async fn begin(&self, return_to: String) -> Result<ExternalAuthStart> {
        if !is_local_return_path(&return_to) {
            bail!("invalid return path");
        }
        let settings = self.settings()?;
        if !settings.enabled {
            bail!("external authentication is disabled");
        }

        let fingerprint = settings_fingerprint(&settings);
        let provider = self.provider(&settings, fingerprint).await?;
        let authorization = provider.authorize();

        let mut flows = self.runtime.flows.lock().expect("external auth flow lock poisoned");
        let now = Instant::now();
        flows.retain(|_, flow| flow.expires_at > now);
        if flows.len() >= 256
            && let Some(oldest) = flows.iter().min_by_key(|(_, flow)| flow.expires_at).map(|(state, _)| state.clone())
        {
            flows.remove(&oldest);
        }
        flows.insert(
            authorization.state.clone(),
            PendingFlow {
                provider,
                secret: authorization.secret,
                settings_fingerprint: fingerprint,
                return_to,
                expires_at: now + FLOW_LIFETIME,
            },
        );
        Ok(ExternalAuthStart { authorization_url: authorization.url, state: authorization.state })
    }

    /// Consumes a login flow and resolves it to a local Liwan account.
    pub async fn finish(&self, state: &str, code: String) -> Result<ExternalAuthLogin> {
        // Removing before token exchange makes every callback attempt one-use, including failures.
        let flow = self
            .runtime
            .flows
            .lock()
            .expect("external auth flow lock poisoned")
            .remove(state)
            .context("external authentication flow not found or already used")?;
        if flow.expires_at <= Instant::now() {
            bail!("external authentication flow expired");
        }

        let settings = self.settings()?;
        if !settings.enabled || settings_fingerprint(&settings) != flow.settings_fingerprint {
            bail!("external authentication settings changed during login");
        }

        let identity = flow.provider.complete(code, flow.secret, &self.runtime.http).await?;

        let username = match self.find_user(&identity.provider_key, &identity.subject)? {
            Some(username) => username,
            None if settings.allow_user_creation => self.create_user(&identity)?,
            None => bail!("external user creation is disabled"),
        };
        Ok(ExternalAuthLogin { username, return_to: flow.return_to })
    }

    /// Cancels an in-progress login flow.
    pub fn cancel(&self, state: &str) {
        self.runtime.flows.lock().expect("external auth flow lock poisoned").remove(state);
    }

    async fn provider(&self, settings: &ExternalAuthSettings, fingerprint: blake3::Hash) -> Result<Arc<Provider>> {
        if let Some(provider) = self
            .runtime
            .provider
            .lock()
            .expect("external auth provider lock poisoned")
            .as_ref()
            .filter(|provider| {
                provider.settings_fingerprint == fingerprint && provider.created_at.elapsed() < PROVIDER_CACHE_LIFETIME
            })
            .map(|provider| provider.provider.clone())
        {
            return Ok(provider);
        }

        let provider = Arc::new(self.build_provider(settings).await?);
        *self.runtime.provider.lock().expect("external auth provider lock poisoned") = Some(CachedProvider {
            settings_fingerprint: fingerprint,
            provider: provider.clone(),
            created_at: Instant::now(),
        });
        Ok(provider)
    }

    async fn build_provider(&self, settings: &ExternalAuthSettings) -> Result<Provider> {
        if settings.client_id.trim().is_empty() {
            bail!("client ID is required");
        }
        if settings.client_secret.as_deref().is_none_or(|secret| secret.trim().is_empty()) {
            bail!("client secret is required");
        }
        if settings.display_name.trim().is_empty() {
            bail!("display name is required");
        }

        Provider::from_settings(settings, &self.runtime.redirect_url, &self.runtime.http).await
    }

    fn find_user(&self, provider_key: &str, subject: &str) -> Result<Option<String>> {
        let conn = self.pool.get()?;
        Ok(conn
            .query_row(
                "select username from external_identities where provider_key = ? and subject = ?",
                [provider_key, subject],
                |row| row.get(0),
            )
            .optional()?)
    }

    fn create_user(&self, identity: &ExternalIdentity) -> Result<String> {
        if identity.provider_key.is_empty() || identity.subject.is_empty() {
            bail!("external identity is missing a stable identifier");
        }

        let mut conn = self.pool.get()?;
        let transaction = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(username) = transaction
            .query_row(
                "select username from external_identities where provider_key = ? and subject = ?",
                [&identity.provider_key, &identity.subject],
                |row| row.get(0),
            )
            .optional()?
        {
            return Ok(username);
        }

        let base = username_base(identity.username_hint.as_deref());
        let hash = blake3::hash(format!("{}\0{}", identity.provider_key, identity.subject).as_bytes()).to_hex();
        let suffix = &hash[..8];
        let mut attempt = 0;
        let username = loop {
            let candidate = match attempt {
                0 => base.clone(),
                1 => format!("{base}-{suffix}"),
                _ => format!("{base}-{suffix}-{attempt}"),
            };
            let exists: bool = transaction.query_row(
                "select exists(select 1 from users where username = ?)",
                [&candidate],
                |row| row.get(0),
            )?;
            if !exists && validate::is_valid_username(&candidate) {
                break candidate;
            }
            attempt += 1;
        };

        transaction.execute(
            "insert into users (username, password_hash, role, projects) values (?, null, ?, '')",
            rusqlite::params![username, UserRole::User.to_string()],
        )?;
        transaction.execute(
            "insert into external_identities (provider_key, subject, username) values (?, ?, ?)",
            rusqlite::params![identity.provider_key, identity.subject, username],
        )?;
        transaction.commit()?;
        Ok(username)
    }
}

fn username_base(hint: Option<&str>) -> String {
    let hint = hint.unwrap_or("user");
    let hint = hint.split_once('@').map_or(hint, |(local, _)| local);
    let mut base = String::new();
    for character in hint
        .chars()
        .filter(|character| character.is_alphanumeric() || matches!(character, '-' | '_'))
        .flat_map(char::to_lowercase)
    {
        if base.len() + character.len_utf8() > 48 {
            break;
        }
        base.push(character);
    }

    if validate::is_valid_username(&base) { base } else { "user".to_string() }
}

fn normalize_settings(settings: &ExternalAuthSettings) -> ExternalAuthSettings {
    let optional = |value: &Option<String>| {
        value.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()).map(str::to_string)
    };
    ExternalAuthSettings {
        enabled: settings.enabled,
        provider: settings.provider,
        display_name: settings.display_name.trim().to_string(),
        client_id: settings.client_id.trim().to_string(),
        client_secret: optional(&settings.client_secret),
        issuer_url: optional(&settings.issuer_url),
        allowed_domain: optional(&settings.allowed_domain).map(|value| value.to_lowercase()),
        allowed_organization: optional(&settings.allowed_organization).map(|value| value.to_lowercase()),
        allow_user_creation: settings.allow_user_creation,
    }
}

fn settings_fingerprint(settings: &ExternalAuthSettings) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new();
    for value in [
        if settings.enabled { "true" } else { "false" },
        settings.provider.as_str(),
        &settings.display_name,
        &settings.client_id,
        settings.client_secret.as_deref().unwrap_or_default(),
        settings.issuer_url.as_deref().unwrap_or_default(),
        settings.allowed_domain.as_deref().unwrap_or_default(),
        settings.allowed_organization.as_deref().unwrap_or_default(),
        if settings.allow_user_creation { "true" } else { "false" },
    ] {
        hasher.update(&(value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
    hasher.finalize()
}

fn is_local_return_path(path: &str) -> bool {
    path.parse::<::http::Uri>().is_ok_and(|uri| {
        uri.scheme().is_none()
            && uri.authority().is_none()
            && uri.path().starts_with('/')
            && !uri.path().starts_with("//")
            && !path.contains('\\')
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::Liwan, config::Config};

    #[tokio::test]
    async fn settings_round_trip() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let settings = ExternalAuthSettings {
            enabled: false,
            provider: ExternalAuthProvider::Google,
            display_name: "Google".to_string(),
            client_id: "client-id".to_string(),
            client_secret: Some("secret".to_string()),
            issuer_url: None,
            allowed_domain: Some("example.com".to_string()),
            allowed_organization: None,
            allow_user_creation: true,
        };

        app.external_auth.update_settings(&settings).await.unwrap();
        assert_eq!(app.external_auth.settings().unwrap(), settings);
    }

    #[test]
    fn creates_external_users_without_passwords() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let identity = ExternalIdentity {
            provider_key: "https://issuer.example".to_string(),
            subject: "123".to_string(),
            username_hint: Some("Test Person@example.com".to_string()),
        };

        let username = app.external_auth.create_user(&identity).unwrap();
        assert_eq!(username, "testperson");
        assert_eq!(app.external_auth.find_user(&identity.provider_key, &identity.subject).unwrap(), Some(username));
        assert!(!app.users.check_login("testperson", "anything").unwrap());
    }

    #[test]
    fn reuses_identity_and_resolves_username_collisions() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        app.users.create("person", "password", UserRole::User, &[]).unwrap();
        let identity = ExternalIdentity {
            provider_key: "github".to_string(),
            subject: "42".to_string(),
            username_hint: Some("person".to_string()),
        };

        let username = app.external_auth.create_user(&identity).unwrap();
        assert!(username.starts_with("person-"));
        assert_eq!(app.external_auth.create_user(&identity).unwrap(), username);
    }

    #[test]
    fn deleting_a_user_removes_auth_records() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let identity = ExternalIdentity {
            provider_key: "github.com".to_string(),
            subject: "42".to_string(),
            username_hint: Some("person".to_string()),
        };
        let username = app.external_auth.create_user(&identity).unwrap();
        app.sessions.create("session", &username, chrono::Utc::now() + chrono::Duration::hours(1)).unwrap();

        app.users.delete(&username).unwrap();
        assert_eq!(app.external_auth.find_user(&identity.provider_key, &identity.subject).unwrap(), None);
        assert!(app.sessions.get("session").unwrap().is_none());
    }

    #[tokio::test]
    async fn rejects_external_return_paths() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        assert!(app.external_auth.begin("https://example.com".to_string()).await.is_err());
        assert!(app.external_auth.begin("//example.com".to_string()).await.is_err());
        assert!(app.external_auth.begin("/\\example.com".to_string()).await.is_err());
        assert!(app.external_auth.begin("/path\r\nlocation:https://example.com".to_string()).await.is_err());
    }

    #[test]
    fn generated_usernames_stay_valid() {
        let base = username_base(Some("éééééééééééééééééééééééééééééééé"));
        assert!(validate::is_valid_username(&base));
        assert!(base.len() <= 48);
        assert!(validate::is_valid_username(&format!("{base}-12345678")));
    }
}

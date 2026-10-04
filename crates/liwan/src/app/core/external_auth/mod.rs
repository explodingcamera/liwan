use crate::{
    app::{SqlitePool, models::UserRole},
    utils::validate,
};
mod http;
mod provider;

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use rusqlite::{OptionalExtension, TransactionBehavior};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use provider::{FlowSecret, Provider};

/// An external authentication provider supported by Liwan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExternalAuthProvider {
    Oidc,
    Google,
    Microsoft,
}

impl ExternalAuthProvider {
    fn as_str(self) -> &'static str {
        match self {
            Self::Oidc => "oidc",
            Self::Google => "google",
            Self::Microsoft => "microsoft",
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
    pub tenant_id: Option<String>,
    pub allow_user_creation: bool,
    pub allow_session_reuse: bool,
    pub default_team_id: Option<String>,
    pub group_team_mappings: Vec<GroupTeamMapping>,
    pub additional_scopes: String,
    pub group_claim_name: String,
}

/// An identity provider group mapped to a Liwan team at account creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupTeamMapping {
    pub group_id: String,
    pub team_id: String,
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
            .field("tenant_id", &self.tenant_id)
            .field("allow_user_creation", &self.allow_user_creation)
            .field("allow_session_reuse", &self.allow_session_reuse)
            .field("default_team_id", &self.default_team_id)
            .field("group_team_mappings", &self.group_team_mappings)
            .field("additional_scopes", &self.additional_scopes)
            .field("group_claim_name", &self.group_claim_name)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExternalIdentity {
    provider_key: String,
    subject: String,
    username_hint: Option<String>,
    groups: Option<Vec<String>>,
    groups_overage: bool,
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
    settings_update: tokio::sync::Mutex<()>,
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
    pub fn try_new(pool: SqlitePool, redirect_url: url::Url) -> Result<Self> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            pool,
            runtime: Arc::new(RuntimeState {
                http,
                redirect_url: redirect_url.to_string(),
                settings_update: tokio::sync::Mutex::new(()),
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
            tenant_id,
            allow_user_creation,
            allow_session_reuse,
            default_team_id,
            group_team_mappings,
            additional_scopes,
            group_claim_name,
        ) = conn.query_row(
            r"select enabled, provider, display_name, client_id, client_secret, issuer_url,
                       allowed_domain, tenant_id, allow_user_creation, allow_session_reuse,
                       default_team_id, group_team_mappings, additional_scopes, group_claim_name
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
                    row.get("tenant_id")?,
                    row.get("allow_user_creation")?,
                    row.get("allow_session_reuse")?,
                    row.get("default_team_id")?,
                    row.get::<_, String>("group_team_mappings")?,
                    row.get("additional_scopes")?,
                    row.get("group_claim_name")?,
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
            tenant_id,
            allow_user_creation,
            allow_session_reuse,
            default_team_id,
            group_team_mappings: serde_json::from_str(&group_team_mappings)?,
            additional_scopes,
            group_claim_name,
        })
    }

    /// Validates and replaces the external authentication settings.
    pub async fn update_settings(&self, settings: &ExternalAuthSettings) -> Result<()> {
        let _guard = self.runtime.settings_update.lock().await;
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
        if settings.provider == ExternalAuthProvider::Google && !settings.group_team_mappings.is_empty() {
            bail!("Google does not provide group claims");
        }
        if settings.group_team_mappings.len() > 100 {
            bail!("too many group mappings");
        }
        if settings.provider != ExternalAuthProvider::Oidc && !settings.additional_scopes.is_empty() {
            bail!("additional scopes are only supported for OpenID Connect");
        }
        if settings.provider != ExternalAuthProvider::Oidc && !settings.group_claim_name.is_empty() {
            bail!("custom group claims are only supported for OpenID Connect");
        }
        if settings.provider == ExternalAuthProvider::Oidc
            && (settings.group_claim_name.is_empty() && !settings.group_team_mappings.is_empty()
                || settings.group_claim_name.len() > 255
                || settings.group_claim_name.chars().any(char::is_whitespace)
                || settings.group_claim_name.chars().any(char::is_control)
                || matches!(
                    settings.group_claim_name.as_str(),
                    "sub" | "iss" | "aud" | "email" | "tid" | "hd" | "hasgroups" | "_claim_names"
                ))
        {
            bail!("invalid group claim name");
        }
        if settings.additional_scopes.len() > 1024
            || settings
                .additional_scopes
                .split_whitespace()
                .any(|scope| scope.len() > 255 || !scope.bytes().all(|byte| (0x21..=0x7e).contains(&byte)))
        {
            bail!("invalid additional scopes");
        }
        let mut conn = self.pool.get()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut groups = HashSet::new();
        for team_id in settings.default_team_id.iter().chain(settings.group_team_mappings.iter().map(|m| &m.team_id)) {
            if !tx.prepare_cached("select 1 from teams where id = ?")?.exists([team_id])? {
                bail!("team not found: {team_id}");
            }
        }
        for mapping in &settings.group_team_mappings {
            if mapping.group_id.trim().is_empty() || mapping.group_id.len() > 255 || !groups.insert(&mapping.group_id) {
                bail!("group IDs must be unique and between 1 and 255 characters");
            }
        }
        let mappings = serde_json::to_string(&settings.group_team_mappings)?;
        tx.execute(
            r"update external_auth_settings set
                   enabled = :enabled,
                   provider = :provider,
                   display_name = :display_name,
                   client_id = :client_id,
                   client_secret = :client_secret,
                   issuer_url = :issuer_url,
                   allowed_domain = :allowed_domain,
                   tenant_id = :tenant_id,
                   allow_user_creation = :allow_user_creation,
                   allow_session_reuse = :allow_session_reuse,
                   default_team_id = :default_team_id,
                   group_team_mappings = :group_team_mappings,
                   additional_scopes = :additional_scopes,
                   group_claim_name = :group_claim_name
               where id = 1",
            rusqlite::named_params! {
                ":enabled": settings.enabled,
                ":provider": settings.provider.as_str(),
                ":display_name": settings.display_name,
                ":client_id": settings.client_id,
                ":client_secret": settings.client_secret,
                ":issuer_url": settings.issuer_url,
                ":allowed_domain": settings.allowed_domain,
                ":tenant_id": settings.tenant_id,
                ":allow_user_creation": settings.allow_user_creation,
                ":allow_session_reuse": settings.allow_session_reuse,
                ":default_team_id": settings.default_team_id,
                ":group_team_mappings": mappings,
                ":additional_scopes": settings.additional_scopes,
                ":group_claim_name": settings.group_claim_name,
            },
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Returns the callback URL that must be registered with the provider.
    pub fn callback_url(&self) -> &str {
        &self.runtime.redirect_url
    }

    /// Starts a short-lived external login flow.
    pub async fn begin(&self, return_to: String) -> Result<ExternalAuthStart> {
        let return_to = local_return_path(&return_to).context("invalid return path")?;
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

        self.current_flow_settings(flow.settings_fingerprint)?;
        let identity = flow.provider.complete(code, flow.secret, &self.runtime.http).await?;

        // Provider requests are asynchronous, so policy may have changed while one was in progress.
        let _guard = self.runtime.settings_update.lock().await;
        let settings = self.current_flow_settings(flow.settings_fingerprint)?;
        let username = match self.find_user(&identity.provider_key, &identity.subject)? {
            Some(username) => username,
            None if settings.allow_user_creation => self.create_user(&identity, &settings)?,
            None => bail!("external user creation is disabled"),
        };
        Ok(ExternalAuthLogin { username, return_to: flow.return_to })
    }

    /// Cancels an in-progress login flow.
    pub fn cancel(&self, state: &str) {
        self.runtime.flows.lock().expect("external auth flow lock poisoned").remove(state);
    }

    fn current_flow_settings(&self, fingerprint: blake3::Hash) -> Result<ExternalAuthSettings> {
        let settings = self.settings()?;
        if !settings.enabled || settings_fingerprint(&settings) != fingerprint {
            bail!("external authentication settings changed during login");
        }
        Ok(settings)
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

    fn create_user(&self, identity: &ExternalIdentity, settings: &ExternalAuthSettings) -> Result<String> {
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

        let mut teams: HashSet<&str> = settings.default_team_id.as_deref().into_iter().collect();
        if !settings.group_team_mappings.is_empty() {
            if identity.groups_overage {
                bail!("provider group list is incomplete");
            }
            let groups = identity.groups.as_ref().context("provider did not return groups")?;
            for mapping in &settings.group_team_mappings {
                if groups.contains(&mapping.group_id) {
                    teams.insert(&mapping.team_id);
                }
            }
        }
        for team_id in settings.default_team_id.iter().chain(settings.group_team_mappings.iter().map(|m| &m.team_id)) {
            if !transaction.prepare_cached("select 1 from teams where id = ?")?.exists([team_id])? {
                bail!("team not found: {team_id}");
            }
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
            "insert into users (username, password_hash, role) values (?, null, ?)",
            rusqlite::params![username, UserRole::User.to_string()],
        )?;
        transaction.execute(
            "insert into external_identities (provider_key, subject, username) values (?, ?, ?)",
            rusqlite::params![identity.provider_key, identity.subject, username],
        )?;
        for team_id in teams {
            transaction.execute("insert into team_users (team_id, username) values (?, ?)", (team_id, &username))?;
        }
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
    let client_secret = settings.client_secret.as_ref().filter(|secret| !secret.trim().is_empty()).cloned();
    let (issuer_url, allowed_domain, tenant_id) = match settings.provider {
        ExternalAuthProvider::Oidc => (optional(&settings.issuer_url), None, None),
        ExternalAuthProvider::Google => {
            (None, optional(&settings.allowed_domain).map(|value| value.to_lowercase()), None)
        }
        ExternalAuthProvider::Microsoft => {
            (None, None, optional(&settings.tenant_id).map(|value| value.to_lowercase()))
        }
    };
    ExternalAuthSettings {
        enabled: settings.enabled,
        provider: settings.provider,
        display_name: settings.display_name.trim().to_string(),
        client_id: settings.client_id.trim().to_string(),
        client_secret,
        issuer_url,
        allowed_domain,
        tenant_id,
        allow_user_creation: settings.allow_user_creation,
        allow_session_reuse: settings.allow_session_reuse,
        default_team_id: optional(&settings.default_team_id),
        group_team_mappings: settings
            .group_team_mappings
            .iter()
            .map(|mapping| GroupTeamMapping {
                group_id: mapping.group_id.clone(),
                team_id: mapping.team_id.trim().to_string(),
            })
            .collect(),
        additional_scopes: if settings.provider == ExternalAuthProvider::Oidc {
            settings.additional_scopes.split_whitespace().collect::<Vec<_>>().join(" ")
        } else {
            String::new()
        },
        group_claim_name: if settings.provider == ExternalAuthProvider::Oidc {
            settings.group_claim_name.trim().to_string()
        } else {
            String::new()
        },
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
        settings.tenant_id.as_deref().unwrap_or_default(),
        if settings.allow_user_creation { "true" } else { "false" },
        if settings.allow_session_reuse { "true" } else { "false" },
        settings.default_team_id.as_deref().unwrap_or_default(),
        &settings.additional_scopes,
        &settings.group_claim_name,
    ] {
        hasher.update(&(value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
    for mapping in &settings.group_team_mappings {
        for value in [&mapping.group_id, &mapping.team_id] {
            hasher.update(&(value.len() as u64).to_le_bytes());
            hasher.update(value.as_bytes());
        }
    }
    hasher.finalize()
}

fn local_return_path(path: &str) -> Option<String> {
    if !path.starts_with('/') || path.contains('\\') || path.chars().any(char::is_control) {
        return None;
    }

    let base = url::Url::parse("http://liwan.invalid").expect("valid return path base");
    let target = base.join(path).ok()?;
    if target.origin() != base.origin() {
        return None;
    }

    let mut normalized = target.path().to_string();
    if let Some(query) = target.query() {
        normalized.push('?');
        normalized.push_str(query);
    }
    if let Some(fragment) = target.fragment() {
        normalized.push('#');
        normalized.push_str(fragment);
    }
    Some(normalized)
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
            tenant_id: None,
            allow_user_creation: true,
            allow_session_reuse: false,
            default_team_id: None,
            group_team_mappings: vec![],
            additional_scopes: String::new(),
            group_claim_name: String::new(),
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
            groups: None,
            groups_overage: false,
        };

        let username = app.external_auth.create_user(&identity, &app.external_auth.settings().unwrap()).unwrap();
        assert_eq!(username, "testperson");
        assert_eq!(app.external_auth.find_user(&identity.provider_key, &identity.subject).unwrap(), Some(username));
        assert!(!app.users.check_login("testperson", "anything").unwrap());
    }

    #[test]
    fn reuses_identity_and_resolves_username_collisions() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        app.users.create("person", "password", UserRole::User).unwrap();
        let identity = ExternalIdentity {
            provider_key: "https://issuer.example".to_string(),
            subject: "42".to_string(),
            username_hint: Some("person".to_string()),
            groups: None,
            groups_overage: false,
        };

        let settings = app.external_auth.settings().unwrap();
        let username = app.external_auth.create_user(&identity, &settings).unwrap();
        assert!(username.starts_with("person-"));
        assert_eq!(app.external_auth.create_user(&identity, &settings).unwrap(), username);
    }

    #[test]
    fn assigns_teams_only_when_creating_an_external_user() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let default_team = app.teams.create("Default").unwrap();
        let group_team = app.teams.create("Engineering").unwrap();
        let identity = ExternalIdentity {
            provider_key: "https://issuer.example".to_string(),
            subject: "employee".to_string(),
            username_hint: Some("employee".to_string()),
            groups: Some(vec!["engineering".to_string(), "other".to_string()]),
            groups_overage: false,
        };
        let mut settings = app.external_auth.settings().unwrap();
        settings.default_team_id = Some(default_team.clone());
        settings.group_team_mappings = vec![
            GroupTeamMapping { group_id: "engineering".into(), team_id: group_team.clone() },
            GroupTeamMapping { group_id: "other".into(), team_id: group_team.clone() },
        ];
        let username = app.external_auth.create_user(&identity, &settings).unwrap();
        let teams = app.teams.all().unwrap();
        assert!(teams.iter().filter(|team| team.users.contains(&username)).count() == 2);

        settings.default_team_id = None;
        settings.group_team_mappings.clear();
        assert_eq!(app.external_auth.create_user(&identity, &settings).unwrap(), username);
        assert!(app.teams.all().unwrap().iter().all(|team| team.users == vec![username.clone()]));
        app.teams.delete(&default_team).unwrap();
        assert_eq!(app.external_auth.create_user(&identity, &settings).unwrap(), username);
    }

    #[test]
    fn incomplete_groups_do_not_create_accounts() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let team_id = app.teams.create("Engineering").unwrap();
        let mut settings = app.external_auth.settings().unwrap();
        settings.group_team_mappings = vec![GroupTeamMapping { group_id: "engineering".into(), team_id }];
        let mut identity = ExternalIdentity {
            provider_key: "https://issuer.example".to_string(),
            subject: "employee".to_string(),
            username_hint: Some("employee".to_string()),
            groups: None,
            groups_overage: false,
        };
        assert!(app.external_auth.create_user(&identity, &settings).is_err());
        identity.groups = Some(vec!["engineering".into()]);
        identity.groups_overage = true;
        assert!(app.external_auth.create_user(&identity, &settings).is_err());
        identity.groups_overage = false;
        app.teams.delete(&settings.group_team_mappings[0].team_id).unwrap();
        assert!(app.external_auth.create_user(&identity, &settings).is_err());
        assert!(app.external_auth.find_user(&identity.provider_key, &identity.subject).unwrap().is_none());
    }

    #[test]
    fn google_uses_default_team_without_group_claims() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let team_id = app.teams.create("New users").unwrap();
        let mut settings = app.external_auth.settings().unwrap();
        settings.provider = ExternalAuthProvider::Google;
        settings.default_team_id = Some(team_id);
        let identity = ExternalIdentity {
            provider_key: "https://accounts.google.com".into(),
            subject: "123".into(),
            username_hint: Some("person@example.com".into()),
            groups: None,
            groups_overage: false,
        };
        let username = app.external_auth.create_user(&identity, &settings).unwrap();
        assert_eq!(app.teams.all().unwrap()[0].users, vec![username]);
    }

    #[tokio::test]
    async fn validates_assignment_settings() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let mut settings = app.external_auth.settings().unwrap();
        settings.default_team_id = Some("missing".into());
        assert!(app.external_auth.update_settings(&settings).await.is_err());
        settings.default_team_id = None;
        settings.group_team_mappings =
            vec![GroupTeamMapping { group_id: "engineering".into(), team_id: "missing".into() }];
        assert!(app.external_auth.update_settings(&settings).await.is_err());
        let team_id = app.teams.create("Engineering").unwrap();
        settings.group_team_mappings[0].team_id = team_id;
        settings.provider = ExternalAuthProvider::Google;
        assert!(app.external_auth.update_settings(&settings).await.is_err());
        settings.provider = ExternalAuthProvider::Oidc;
        assert!(app.external_auth.update_settings(&settings).await.is_err());
        settings.group_claim_name = "groups".into();
        app.external_auth.update_settings(&settings).await.unwrap();
        assert_eq!(app.external_auth.settings().unwrap(), settings);
    }

    #[tokio::test]
    async fn preserves_exact_claim_values_in_mappings() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let team_id = app.teams.create("Engineering").unwrap();
        let mut settings = app.external_auth.settings().unwrap();
        settings.group_claim_name = "roles".into();
        settings.group_team_mappings = vec![GroupTeamMapping { group_id: " engineering ".into(), team_id }];
        app.external_auth.update_settings(&settings).await.unwrap();
        assert_eq!(app.external_auth.settings().unwrap().group_team_mappings, settings.group_team_mappings);
    }

    #[test]
    fn deleting_a_user_removes_auth_records() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        let identity = ExternalIdentity {
            provider_key: "https://issuer.example".to_string(),
            subject: "42".to_string(),
            username_hint: Some("person".to_string()),
            groups: None,
            groups_overage: false,
        };
        let username = app.external_auth.create_user(&identity, &app.external_auth.settings().unwrap()).unwrap();
        app.sessions.create("session", &username, chrono::Utc::now() + chrono::Duration::hours(1)).unwrap();

        app.users.delete(&username).unwrap();
        assert_eq!(app.external_auth.find_user(&identity.provider_key, &identity.subject).unwrap(), None);
        assert!(app.sessions.get("session").unwrap().is_none());
    }

    #[test]
    fn validates_and_normalizes_return_paths() {
        assert_eq!(local_return_path("/settings?tab=auth#provider"), Some("/settings?tab=auth#provider".to_string()));
        assert_eq!(local_return_path("/settings/../projects"), Some("/projects".to_string()));
        assert_eq!(local_return_path("https://example.com"), None);
        assert_eq!(local_return_path("//example.com"), None);
        assert_eq!(local_return_path("/\\example.com"), None);
        assert_eq!(local_return_path("/\texample.com"), None);
        assert_eq!(local_return_path("/path\r\nlocation:https://example.com"), None);
    }

    #[test]
    fn generated_usernames_stay_valid() {
        let base = username_base(Some("éééééééééééééééééééééééééééééééé"));
        assert!(validate::is_valid_username(&base));
        assert!(base.len() <= 48);
        assert!(validate::is_valid_username(&format!("{base}-12345678")));
    }
}

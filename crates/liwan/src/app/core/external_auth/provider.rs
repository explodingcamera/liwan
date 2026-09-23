use std::net::IpAddr;

use anyhow::{Context, Result, bail};
use oauth2::{AuthType, EndpointMaybeSet, EndpointNotSet, EndpointSet};
use openidconnect::{
    AdditionalClaims, AuthenticationFlow, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    EmptyExtraTokenFields, IdTokenFields, IssuerUrl, JsonWebKeySet, Nonce, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, StandardErrorResponse, StandardTokenResponse, TokenResponse,
    core::{
        CoreAuthDisplay, CoreAuthPrompt, CoreClientAuthMethod, CoreErrorResponseType, CoreGenderClaim, CoreJsonWebKey,
        CoreJweContentEncryptionAlgorithm, CoreProviderMetadata, CoreResponseType, CoreRevocableToken,
        CoreRevocationErrorResponse, CoreTokenIntrospectionResponse, CoreTokenType,
    },
};
use serde::{Deserialize, Serialize};

use super::{ExternalAuthProvider, ExternalAuthSettings, ExternalIdentity, http};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ProviderClaims {
    #[serde(default)]
    hd: Option<String>,
    #[serde(default)]
    tid: Option<String>,
}

impl AdditionalClaims for ProviderClaims {}

type ProviderIdTokenFields = IdTokenFields<
    ProviderClaims,
    EmptyExtraTokenFields,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    openidconnect::core::CoreJwsSigningAlgorithm,
>;
type ProviderTokenResponse = StandardTokenResponse<ProviderIdTokenFields, CoreTokenType>;
type ProviderClient = Client<
    ProviderClaims,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<CoreErrorResponseType>,
    ProviderTokenResponse,
    CoreTokenIntrospectionResponse,
    CoreRevocableToken,
    CoreRevocationErrorResponse,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

enum ProviderPolicy {
    Generic,
    Google { allowed_domain: Option<String> },
    Microsoft { tenant_id: String },
}

pub(super) struct FlowSecret {
    nonce: Nonce,
    verifier: PkceCodeVerifier,
}

pub(super) struct Authorization {
    pub(super) url: url::Url,
    pub(super) state: String,
    pub(super) secret: FlowSecret,
}

pub(super) struct Provider {
    client: ProviderClient,
    provider_key: String,
    policy: ProviderPolicy,
    allow_session_reuse: bool,
}

impl Provider {
    pub(super) async fn from_settings(
        settings: &ExternalAuthSettings,
        redirect_url: &str,
        client: &reqwest::Client,
    ) -> Result<Self> {
        let (issuer, policy) = match settings.provider {
            ExternalAuthProvider::Oidc => {
                (settings.issuer_url.clone().context("issuer URL is required")?, ProviderPolicy::Generic)
            }
            ExternalAuthProvider::Google => (
                "https://accounts.google.com".to_string(),
                ProviderPolicy::Google { allowed_domain: settings.allowed_domain.clone() },
            ),
            ExternalAuthProvider::Microsoft => {
                let tenant_id = settings.tenant_id.clone().context("Microsoft tenant ID is required")?;
                (microsoft_issuer(&tenant_id)?, ProviderPolicy::Microsoft { tenant_id })
            }
        };
        validate_issuer(&issuer)?;
        let issuer = IssuerUrl::new(issuer)?;
        let metadata = discover(issuer, client).await?;
        let provider_key = metadata.issuer().to_string();
        let auth_type = token_auth_type(metadata.token_endpoint_auth_methods_supported())?;
        let oidc_client = ProviderClient::from_provider_metadata(
            metadata,
            ClientId::new(settings.client_id.clone()),
            settings.client_secret.clone().map(ClientSecret::new),
        )
        .set_auth_type(auth_type)
        .set_redirect_uri(RedirectUrl::new(redirect_url.to_string())?);

        Ok(Self { client: oidc_client, provider_key, policy, allow_session_reuse: settings.allow_session_reuse })
    }

    pub(super) fn authorize(&self) -> Authorization {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let request = self
            .client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .set_pkce_challenge(challenge);
        let request = if self.allow_session_reuse { request } else { request.add_prompt(CoreAuthPrompt::Login) };
        let (url, state, nonce) = request.url();
        Authorization { url, state: state.secret().clone(), secret: FlowSecret { nonce, verifier } }
    }

    pub(super) async fn complete(
        &self,
        code: String,
        secret: FlowSecret,
        http_client: &reqwest::Client,
    ) -> Result<ExternalIdentity> {
        let token = self
            .client
            .exchange_code(AuthorizationCode::new(code))?
            .set_pkce_verifier(secret.verifier)
            .request_async(&|request| http::execute(http_client.clone(), request))
            .await?;
        let id_token = token.id_token().context("provider did not return an ID token")?;
        let claims = id_token.claims(&self.client.id_token_verifier(), &secret.nonce)?;

        match &self.policy {
            ProviderPolicy::Google { allowed_domain } => {
                if let Some(domain) = allowed_domain
                    && !claims.additional_claims().hd.as_deref().is_some_and(|claim| claim.eq_ignore_ascii_case(domain))
                {
                    bail!("Google account is outside the allowed Workspace domain");
                }
            }
            ProviderPolicy::Microsoft { tenant_id } => {
                if !claims.additional_claims().tid.as_deref().is_some_and(|claim| claim.eq_ignore_ascii_case(tenant_id))
                {
                    bail!("Microsoft account is outside the configured tenant");
                }
            }
            ProviderPolicy::Generic => {}
        }

        let username_hint = claims
            .preferred_username()
            .map(|value| value.as_str().to_string())
            .or_else(|| claims.email().map(|value| value.as_str().to_string()))
            .or_else(|| claims.name().and_then(|name| name.get(None)).map(|value| value.as_str().to_string()));
        Ok(ExternalIdentity {
            provider_key: self.provider_key.clone(),
            subject: claims.subject().as_str().to_string(),
            username_hint,
        })
    }
}

async fn discover(issuer: IssuerUrl, client: &reqwest::Client) -> Result<CoreProviderMetadata> {
    // The crate's URL join drops the final segment from path-based issuers such as Keycloak and Microsoft.
    let mut discovery_url = issuer.url().clone();
    let path = format!("{}/.well-known/openid-configuration", discovery_url.path().trim_end_matches('/'));
    discovery_url.set_path(&path);

    let response = http::execute(
        client.clone(),
        ::http::Request::builder()
            .uri(discovery_url.as_str())
            .header(::http::header::ACCEPT, "application/json")
            .body(Vec::new())?,
    )
    .await?;
    if !response.status().is_success() {
        bail!("OIDC discovery failed with status {}", response.status());
    }
    let metadata: CoreProviderMetadata = serde_json::from_slice(response.body())?;
    if metadata.issuer() != &issuer {
        bail!("OIDC discovery issuer does not match the configured issuer");
    }
    validate_server_endpoint(metadata.authorization_endpoint().url(), metadata.issuer())?;
    validate_server_endpoint(metadata.jwks_uri().url(), metadata.issuer())?;
    validate_server_endpoint(
        metadata.token_endpoint().context("provider has no token endpoint")?.url(),
        metadata.issuer(),
    )?;
    let jwks =
        JsonWebKeySet::fetch_async(metadata.jwks_uri(), &|request| http::execute(client.clone(), request)).await?;
    Ok(metadata.set_jwks(jwks))
}

fn validate_server_endpoint(endpoint: &url::Url, issuer: &IssuerUrl) -> Result<()> {
    if endpoint.host_str().and_then(|host| host.parse::<IpAddr>().ok()).is_some_and(is_private_ip)
        && endpoint.host_str() != issuer.url().host_str()
    {
        bail!("OIDC provider endpoint cannot target a private IP address");
    }
    if endpoint.scheme() != "https" {
        let loopback = endpoint.host_str().is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"));
        let issuer_loopback =
            issuer.url().host_str().is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"));
        if !(endpoint.scheme() == "http" && loopback && issuer_loopback) {
            bail!("OIDC provider endpoint must use HTTPS");
        }
    }
    Ok(())
}

fn is_private_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || address.is_broadcast()
                || address.is_unspecified()
        }
        IpAddr::V6(address) => address.is_loopback() || address.is_unspecified() || address.is_unique_local(),
    }
}

fn token_auth_type(methods: Option<&Vec<CoreClientAuthMethod>>) -> Result<AuthType> {
    match methods {
        None => Ok(AuthType::BasicAuth),
        Some(methods) if methods.contains(&CoreClientAuthMethod::ClientSecretBasic) => Ok(AuthType::BasicAuth),
        Some(methods) if methods.contains(&CoreClientAuthMethod::ClientSecretPost) => Ok(AuthType::RequestBody),
        _ => bail!("provider does not support client secret authentication"),
    }
}

fn microsoft_issuer(tenant_id: &str) -> Result<String> {
    let valid_tenant_id = tenant_id.len() == 36
        && tenant_id.char_indices().all(|(index, character)| {
            if matches!(index, 8 | 13 | 18 | 23) { character == '-' } else { character.is_ascii_hexdigit() }
        });
    if !valid_tenant_id {
        bail!("Microsoft tenant ID must be a UUID");
    }
    let mut issuer = url::Url::parse("https://login.microsoftonline.com")?;
    issuer
        .path_segments_mut()
        .map_err(|_| anyhow::anyhow!("invalid Microsoft authority"))?
        .push(&tenant_id.to_lowercase())
        .push("v2.0");
    Ok(issuer.to_string())
}

fn validate_issuer(issuer: &str) -> Result<()> {
    let url = url::Url::parse(issuer)?;
    let loopback = url.host_str().is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"));
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        bail!("OIDC issuer must use HTTPS");
    }
    if url.query().is_some() || url.fragment().is_some() {
        bail!("OIDC issuer cannot contain a query or fragment");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_token_auth_methods() {
        assert!(matches!(token_auth_type(None).unwrap(), AuthType::BasicAuth));
        let post = vec![CoreClientAuthMethod::ClientSecretPost];
        assert!(matches!(token_auth_type(Some(&post)).unwrap(), AuthType::RequestBody));
        let private_key = vec![CoreClientAuthMethod::PrivateKeyJwt];
        assert!(token_auth_type(Some(&private_key)).is_err());
    }

    #[test]
    fn microsoft_requires_a_tenant_id() {
        for tenant_id in ["", "common", "example.onmicrosoft.com", "not-a-uuid"] {
            assert!(microsoft_issuer(tenant_id).is_err());
        }
        assert_eq!(
            microsoft_issuer("72F988BF-86F1-41AF-91AB-2D7CD011DB47").unwrap(),
            "https://login.microsoftonline.com/72f988bf-86f1-41af-91ab-2d7cd011db47/v2.0"
        );
    }
}

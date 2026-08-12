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

use super::super::{ExternalAuthSettings, ExternalIdentity, http};

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

pub(super) enum OidcKind {
    Generic { issuer: String },
    Google { allowed_domain: Option<String> },
    Microsoft { tenant: String },
}

enum OidcPolicy {
    Generic,
    Google { allowed_domain: Option<String> },
    Microsoft { tenant_id: String },
}

pub(crate) struct OidcProvider {
    client: ProviderClient,
    provider_key: String,
    policy: OidcPolicy,
}

impl OidcProvider {
    pub(super) async fn discover(
        kind: OidcKind,
        settings: &ExternalAuthSettings,
        redirect_url: &str,
        client: &reqwest::Client,
    ) -> Result<Self> {
        let issuer = match &kind {
            OidcKind::Generic { issuer } => issuer.clone(),
            OidcKind::Google { .. } => "https://accounts.google.com".to_string(),
            OidcKind::Microsoft { tenant } => microsoft_issuer(tenant)?,
        };
        validate_issuer(&issuer)?;
        let issuer = IssuerUrl::new(issuer)?;
        let metadata = discover(issuer, client).await?;
        let provider_key = metadata.issuer().to_string();
        let auth_type = token_auth_type(metadata.token_endpoint_auth_methods_supported())?;
        let policy = match kind {
            OidcKind::Generic { .. } => OidcPolicy::Generic,
            OidcKind::Google { allowed_domain } => OidcPolicy::Google { allowed_domain },
            OidcKind::Microsoft { .. } => {
                OidcPolicy::Microsoft { tenant_id: microsoft_tenant_id(metadata.issuer().url())? }
            }
        };
        let oidc_client = ProviderClient::from_provider_metadata(
            metadata,
            ClientId::new(settings.client_id.clone()),
            settings.client_secret.clone().map(ClientSecret::new),
        )
        .set_auth_type(auth_type)
        .set_redirect_uri(RedirectUrl::new(redirect_url.to_string())?);

        Ok(Self { client: oidc_client, provider_key, policy })
    }

    pub(super) fn authorize(&self) -> (url::Url, String, Nonce, PkceCodeVerifier) {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state, nonce) = self
            .client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .set_pkce_challenge(challenge)
            .url();
        (url, state.secret().clone(), nonce, verifier)
    }

    pub(super) async fn complete(
        &self,
        code: String,
        nonce: Nonce,
        verifier: PkceCodeVerifier,
        http_client: &reqwest::Client,
    ) -> Result<ExternalIdentity> {
        let token = self
            .client
            .exchange_code(AuthorizationCode::new(code))?
            .set_pkce_verifier(verifier)
            .request_async(&|request| http::execute(http_client, request))
            .await?;
        let id_token = token.id_token().context("provider did not return an ID token")?;
        let claims = id_token.claims(&self.client.id_token_verifier(), &nonce)?;

        match &self.policy {
            OidcPolicy::Google { allowed_domain } => {
                if let Some(domain) = allowed_domain
                    && !claims.additional_claims().hd.as_deref().is_some_and(|claim| claim.eq_ignore_ascii_case(domain))
                {
                    bail!("Google account is outside the allowed Workspace domain");
                }
            }
            OidcPolicy::Microsoft { tenant_id } => {
                if !claims.additional_claims().tid.as_deref().is_some_and(|claim| claim.eq_ignore_ascii_case(tenant_id))
                {
                    bail!("Microsoft account is outside the configured tenant");
                }
            }
            OidcPolicy::Generic => {}
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
        client,
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
    validate_server_endpoint(metadata.authorization_endpoint().url(), &issuer)?;
    validate_server_endpoint(metadata.jwks_uri().url(), &issuer)?;
    validate_server_endpoint(metadata.token_endpoint().context("provider has no token endpoint")?.url(), &issuer)?;
    let jwks = JsonWebKeySet::fetch_async(metadata.jwks_uri(), &|request| http::execute(client, request)).await?;
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

fn microsoft_tenant_id(issuer: &url::Url) -> Result<String> {
    let tenant_id =
        issuer.path_segments().and_then(|mut segments| segments.next()).context("invalid Microsoft issuer")?;
    if !tenant_id.is_empty() {
        Ok(tenant_id.to_lowercase())
    } else {
        bail!("Microsoft discovery returned an invalid tenant")
    }
}

fn microsoft_issuer(tenant: &str) -> Result<String> {
    let mut issuer = url::Url::parse("https://login.microsoftonline.com")?;
    issuer.path_segments_mut().map_err(|_| anyhow::anyhow!("invalid Microsoft authority"))?.push(tenant).push("v2.0");
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
}

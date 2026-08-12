use anyhow::{Context, Result, bail};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl, basic::BasicClient,
};
use serde::Deserialize;

use super::super::{ExternalAuthSettings, ExternalIdentity, http};

type GithubClient = BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

pub(crate) struct GithubProvider {
    client: GithubClient,
    allowed_organization: Option<String>,
}

#[derive(Deserialize)]
struct GithubUser {
    id: u64,
    login: String,
}

#[derive(Deserialize)]
struct GithubMembership {
    state: String,
}

impl GithubProvider {
    pub(super) fn new(settings: &ExternalAuthSettings, redirect_url: &str) -> Result<Self> {
        let client = BasicClient::new(ClientId::new(settings.client_id.clone()))
            .set_client_secret(ClientSecret::new(settings.client_secret.clone().context("client secret is required")?))
            .set_auth_uri(AuthUrl::new("https://github.com/login/oauth/authorize".to_string())?)
            .set_token_uri(TokenUrl::new("https://github.com/login/oauth/access_token".to_string())?)
            .set_redirect_uri(RedirectUrl::new(redirect_url.to_string())?);
        Ok(Self { client, allowed_organization: settings.allowed_organization.clone() })
    }

    pub(super) fn authorize(&self) -> (url::Url, String, PkceCodeVerifier) {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let mut request = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("read:user".to_string()))
            .set_pkce_challenge(challenge);
        if self.allowed_organization.is_some() {
            request = request.add_scope(Scope::new("read:org".to_string()));
        }
        let (url, state) = request.url();
        (url, state.secret().clone(), verifier)
    }

    pub(super) async fn complete(
        &self,
        code: String,
        verifier: PkceCodeVerifier,
        http_client: &reqwest::Client,
    ) -> Result<ExternalIdentity> {
        let token = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(verifier)
            .request_async(&|request| http::execute(http_client, request))
            .await?;
        let access_token = token.access_token().secret();
        let user: GithubUser = github_request(http_client, "https://api.github.com/user", access_token).await?;

        if let Some(organization) = &self.allowed_organization {
            let mut url = url::Url::parse("https://api.github.com/user/memberships/orgs/")?;
            url.path_segments_mut().map_err(|_| anyhow::anyhow!("invalid GitHub API URL"))?.push(organization);
            let membership: GithubMembership = github_request(http_client, url.as_str(), access_token).await?;
            if membership.state != "active" {
                bail!("GitHub organization membership is not active");
            }
        }

        Ok(ExternalIdentity {
            provider_key: "github.com".to_string(),
            subject: user.id.to_string(),
            username_hint: Some(user.login),
        })
    }
}

async fn github_request<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    access_token: &str,
) -> Result<T> {
    let response = http::execute(
        client,
        ::http::Request::builder()
            .uri(url)
            .header(::http::header::AUTHORIZATION, format!("Bearer {access_token}"))
            .header(::http::header::ACCEPT, "application/vnd.github+json")
            .header(::http::header::USER_AGENT, "Liwan")
            .body(Vec::new())?,
    )
    .await?;
    if !response.status().is_success() {
        bail!("GitHub API request failed with status {}", response.status());
    }
    Ok(serde_json::from_slice(response.body())?)
}

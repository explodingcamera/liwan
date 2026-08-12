mod github;
mod oidc;

use anyhow::{Context, Result, bail};

use super::{ExternalAuthProvider, ExternalAuthSettings, ExternalIdentity};

pub(super) enum Provider {
    Oidc(Box<oidc::OidcProvider>),
    Github(Box<github::GithubProvider>),
}

pub(super) enum FlowSecret {
    Oidc { nonce: openidconnect::Nonce, verifier: openidconnect::PkceCodeVerifier },
    Github { verifier: oauth2::PkceCodeVerifier },
}

pub(super) struct Authorization {
    pub(super) url: url::Url,
    pub(super) state: String,
    pub(super) secret: FlowSecret,
}

impl Provider {
    pub(super) async fn from_settings(
        settings: &ExternalAuthSettings,
        redirect_url: &str,
        http: &reqwest::Client,
    ) -> Result<Self> {
        let provider = match settings.provider {
            ExternalAuthProvider::Oidc => Self::Oidc(Box::new(
                oidc::OidcProvider::discover(
                    oidc::OidcKind::Generic { issuer: settings.issuer_url.clone().context("issuer URL is required")? },
                    settings,
                    redirect_url,
                    http,
                )
                .await?,
            )),
            ExternalAuthProvider::Google => Self::Oidc(Box::new(
                oidc::OidcProvider::discover(
                    oidc::OidcKind::Google { allowed_domain: settings.allowed_domain.clone() },
                    settings,
                    redirect_url,
                    http,
                )
                .await?,
            )),
            ExternalAuthProvider::Microsoft => Self::Oidc(Box::new(
                oidc::OidcProvider::discover(
                    oidc::OidcKind::Microsoft {
                        tenant: settings.allowed_organization.clone().context("Microsoft organization is required")?,
                    },
                    settings,
                    redirect_url,
                    http,
                )
                .await?,
            )),
            ExternalAuthProvider::Github => {
                Self::Github(Box::new(github::GithubProvider::new(settings, redirect_url)?))
            }
        };
        Ok(provider)
    }

    pub(super) fn authorize(&self) -> Authorization {
        match self {
            Self::Oidc(provider) => {
                let (url, state, nonce, verifier) = provider.authorize();
                Authorization { url, state, secret: FlowSecret::Oidc { nonce, verifier } }
            }
            Self::Github(provider) => {
                let (url, state, verifier) = provider.authorize();
                Authorization { url, state, secret: FlowSecret::Github { verifier } }
            }
        }
    }

    pub(super) async fn complete(
        &self,
        code: String,
        secret: FlowSecret,
        http: &reqwest::Client,
    ) -> Result<ExternalIdentity> {
        match (self, secret) {
            (Self::Oidc(provider), FlowSecret::Oidc { nonce, verifier }) => {
                provider.complete(code, nonce, verifier, http).await
            }
            (Self::Github(provider), FlowSecret::Github { verifier }) => provider.complete(code, verifier, http).await,
            _ => bail!("external authentication flow provider changed"),
        }
    }
}

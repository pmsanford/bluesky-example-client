use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use atrium_api::app::bsky::actor::get_profile::Parameters as GetProfileParams;
use atrium_api::app::bsky::feed::defs::PostView;
use atrium_api::client::{AtpService, AtpServiceClient, AtpServiceWrapper};
use atrium_api::com::atproto::server::create_session::Input as CreateSessionInput;
use atrium_xrpc::client::reqwest::ReqwestClient;

use super::session::BSkySession;
use atrium_api::app::bsky::feed::get_posts::Parameters as GetPostsParams;
use chrono::Utc;

#[derive(Clone)]
struct BSkyXrpc {
    inner: Arc<ReqwestClient>,
    session: Arc<Mutex<BSkySession>>,
}

#[async_trait::async_trait]
impl AtpService for BSkyXrpc {}

pub struct BSky {
    client: Arc<AtpServiceClient<AtpServiceWrapper<BSkyXrpc>>>,
    xrpc: BSkyXrpc,
}

impl BSky {
    pub async fn login(username: String, password: String) -> Result<Self> {
        let bootstrap = AtpServiceClient::new(ReqwestClient::new("https://bsky.social".into()));
        let input = CreateSessionInput {
            identifier: username,
            password,
        };
        let session = bootstrap
            .service
            .com
            .atproto
            .server
            .create_session(input)
            .await?;
        let xrpc = BSkyXrpc {
            inner: Arc::new(ReqwestClient::new("https://bsky.social".into())),
            session: Arc::new(Mutex::new(session.try_into()?)),
        };
        Ok(Self {
            client: Arc::new(AtpServiceClient::new(xrpc.clone())),
            xrpc,
        })
    }

    pub async fn get_post(&self, handle: String, id: String) -> Result<PostView> {
        self.ensure_token_valid().await?;
        let prof_params = GetProfileParams {
            actor: handle.clone(),
        };

        let profile = self
            .client
            .service
            .app
            .bsky
            .actor
            .get_profile(prof_params)
            .await?;

        let posts_params = GetPostsParams {
            uris: vec![format!("at://{}/app.bsky.feed.post/{id}", profile.did)],
        };
        let posts = self
            .client
            .service
            .app
            .bsky
            .feed
            .get_posts(posts_params)
            .await?;
        let post = posts
            .posts
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Couldn't find post {id} for {handle}"))?;

        Ok(post)
    }

    async fn ensure_token_valid(&self) -> Result<()> {
        let jwt_expired = {
            let session = self
                .xrpc
                .session
                .lock()
                .map_err(|e| anyhow!("session mutex is poisoned: {e}"))?;
            Utc::now() > session.access_jwt_exp
        };
        if jwt_expired {
            let refreshed = self
                .client
                .service
                .com
                .atproto
                .server
                .refresh_session()
                .await?;
            let mut session = self
                .xrpc
                .session
                .lock()
                .map_err(|e| anyhow!("session mutex is poisoned: {e}"))?;
            *session = refreshed.try_into()?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl atrium_xrpc::HttpClient for BSkyXrpc {
    async fn send_http(
        &self,
        req: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.send_http(req).await
    }
}

impl atrium_xrpc::XrpcClient for BSkyXrpc {
    fn host(&self) -> &str {
        "https://bsky.social"
    }
    fn auth(&self, is_refresh: bool) -> Option<String> {
        // If the mutex is poisoned, silently return None
        // We'll fail on the next call to ensure_token_valid
        self.session.lock().ok().map(|session| {
            if is_refresh {
                session.refresh_jwt.clone()
            } else {
                session.access_jwt.clone()
            }
        })
    }
}

use std::sync::Mutex;

use anyhow::{anyhow, Result};
use atrium_api::app::bsky::actor::get_profile::{GetProfile, Parameters as GetProfileParams};
use atrium_api::app::bsky::feed::defs::PostView;

use super::session::{create_session, BSkySession};
use atrium_api::app::bsky::feed::get_posts::{GetPosts, Parameters as GetPostsParams};
use atrium_api::com::atproto::server::refresh_session::RefreshSession;
use atrium_api::xrpc::http;
use chrono::Utc;

pub struct BSky {
    inner: reqwest::Client,
    session: Mutex<BSkySession>,
}

impl BSky {
    pub async fn login(username: String, password: String) -> Result<Self> {
        let session = create_session(username, password).await.map_anyhow()?;
        Ok(Self {
            inner: Default::default(),
            session: Mutex::new(session),
        })
    }

    pub async fn get_post(&self, handle: String, id: String) -> Result<PostView> {
        self.ensure_token_valid().await?;
        let prof_params = GetProfileParams {
            actor: handle.clone(),
        };
        let profile = self.get_profile(prof_params).await.map_anyhow()?;

        let posts_params = GetPostsParams {
            uris: vec![format!("at://{}/app.bsky.feed.post/{id}", profile.did)],
        };
        let posts = self.get_posts(posts_params).await.map_anyhow()?;
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
                .session
                .lock()
                .map_err(|e| anyhow!("session mutex is poisoned: {e}"))?;
            Utc::now() > session.access_jwt_exp
        };
        if jwt_expired {
            let refreshed = self.refresh_session().await.map_anyhow()?;
            let mut session = self
                .session
                .lock()
                .map_err(|e| anyhow!("session mutex is poisoned: {e}"))?;
            *session = refreshed.try_into()?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl atrium_api::xrpc::HttpClient for BSky {
    async fn send(
        &self,
        req: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, Box<dyn std::error::Error>> {
        let res = self.inner.execute(req.try_into()?).await?;
        let mut builder = http::Response::builder().status(res.status());
        for (k, v) in res.headers() {
            builder = builder.header(k, v);
        }
        builder
            .body(res.bytes().await?.to_vec())
            .map_err(Into::into)
    }
}

#[async_trait::async_trait]
impl atrium_api::xrpc::XrpcClient for BSky {
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

trait BoxToAnyhow<T> {
    fn map_anyhow(self) -> Result<T>;
}

impl<T> BoxToAnyhow<T> for Result<T, Box<dyn std::error::Error>> {
    fn map_anyhow(self) -> Result<T> {
        self.map_err(|e| anyhow!("{e}"))
    }
}

atrium_api::impl_traits!(BSky);

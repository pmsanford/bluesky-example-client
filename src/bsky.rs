use anyhow::{anyhow, Result};
use atrium_api::agent::AtpAgent;
use atrium_api::app::bsky::actor::get_profile::Parameters as GetProfileParams;
use atrium_api::app::bsky::feed::defs::PostView;
use atrium_api::xrpc::client::reqwest::ReqwestClient;

use atrium_api::app::bsky::feed::get_posts::Parameters as GetPostsParams;

pub struct BSky {
    client: AtpAgent<ReqwestClient>,
}

impl BSky {
    pub async fn login(username: String, password: String) -> Result<Self> {
        let client = AtpAgent::new(ReqwestClient::new("https://bsky.social".into()));
        client.login(&username, &password).await?;
        Ok(Self { client })
    }

    pub async fn get_post(&self, handle: String, id: String) -> Result<PostView> {
        let prof_params = GetProfileParams {
            actor: handle.clone(),
        };

        let profile = self
            .client
            .api
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
            .api
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
}

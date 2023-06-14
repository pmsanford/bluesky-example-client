use anyhow::{anyhow, Result};
use atrium_api::com::atproto::server::refresh_session::Output as RefreshSessionOutput;
use atrium_api::{
    com::atproto::server::create_session::{CreateSession, Input, Output as CreateSessionOutput},
    xrpc::http,
};
use chrono::{DateTime, TimeZone, Utc};
use jwt::{Header, Token};
use serde::Deserialize;

struct LoginClient;

pub async fn create_session(
    identifier: String,
    password: String,
) -> Result<BSkySession, Box<dyn std::error::Error>> {
    let l = LoginClient;
    let input = Input {
        identifier,
        password,
    };
    let result = l.create_session(input).await?;

    Ok(result.try_into()?)
}

#[async_trait::async_trait]
impl atrium_api::xrpc::HttpClient for LoginClient {
    async fn send(
        &self,
        req: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, Box<dyn std::error::Error>> {
        let res = reqwest::Client::default().execute(req.try_into()?).await?;
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
impl atrium_api::xrpc::XrpcClient for LoginClient {
    fn host(&self) -> &str {
        "https://bsky.social"
    }
    fn auth(&self, _: bool) -> Option<String> {
        None
    }
}

atrium_api::impl_traits!(LoginClient);

pub struct BSkySession {
    pub access_jwt: String,
    pub access_jwt_exp: DateTime<Utc>,
    pub refresh_jwt: String,
}

#[derive(Deserialize)]
struct AtprotoClaims {
    exp: i64,
}

pub fn get_token_expiration(jwt_string: &str) -> Result<DateTime<Utc>> {
    let token: Token<Header, AtprotoClaims, _> = Token::parse_unverified(jwt_string)?;
    let expiration_time = Utc
        .timestamp_millis_opt(token.claims().exp)
        .earliest()
        .ok_or_else(|| anyhow!("couldn't interpret expiration timestamp"))?;

    Ok(expiration_time)
}

impl TryInto<BSkySession> for CreateSessionOutput {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<BSkySession> {
        let access_jwt_exp = get_token_expiration(&self.access_jwt)?;
        Ok(BSkySession {
            access_jwt: self.access_jwt,
            access_jwt_exp,
            refresh_jwt: self.refresh_jwt,
        })
    }
}

impl TryInto<BSkySession> for RefreshSessionOutput {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<BSkySession> {
        let access_jwt_exp = get_token_expiration(&self.access_jwt)?;
        Ok(BSkySession {
            access_jwt: self.access_jwt,
            access_jwt_exp,
            refresh_jwt: self.refresh_jwt,
        })
    }
}

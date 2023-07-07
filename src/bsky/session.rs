use anyhow::{anyhow, Result};
use atrium_api::com::atproto::server::create_session::Output as CreateSessionOutput;
use atrium_api::com::atproto::server::refresh_session::Output as RefreshSessionOutput;
use chrono::{DateTime, TimeZone, Utc};
use jwt::{Header, Token};
use serde::Deserialize;

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

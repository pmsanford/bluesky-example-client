use anyhow::{Context, Result};
mod bsky;
mod formatters;
use lazy_static::lazy_static;
use regex::Regex;
use rustyline::{error::ReadlineError, DefaultEditor};

use crate::{bsky::BSky, formatters::get_formatted_post};

lazy_static! {
    static ref BSKY_REGEX: Regex = Regex::new(
        r"https?://(:?www.|staging.)?bsky.app/profile/(?P<handle>.+)/post/(?P<postid>.+)\??.*"
    )
    .unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    let username = std::env::var("BSKY_USERNAME")
        .context("put your bluesky username in the BSKY_USERNAME environment variable")?;
    let password = std::env::var("BSKY_APP_PASS").context(
        "put an app password (NOT your real password) in the BSKY_APP_PASS environment variable",
    )?;

    let client = BSky::login(username, password).await?;

    let mut rl = DefaultEditor::new()?;
    println!("Paste some bluesky post urls (like https://bsky.app/profile/craigweekend.bsky.social/post/3jxrdefxibv2u)");
    println!("Press Control+D to exit");
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                if let Some(caps) = BSKY_REGEX.captures(&line) {
                    match (caps.name("handle"), caps.name("postid")) {
                        (Some(handle), Some(id)) => {
                            match get_formatted_post(
                                &client,
                                handle.as_str().to_owned(),
                                id.as_str().to_owned(),
                            )
                            .await
                            {
                                Ok(cont) => println!("{cont}"),
                                Err(err) => println!("Woops: {:?}", err),
                            }
                        }
                        _ => println!("Couldn't understand that URL"),
                    }
                } else {
                    println!("Didn't understand that");
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(err) => {
                println!("Unexpected: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

use anyhow::{bail, Context, Result};
mod bsky;
use atrium_api::{
    app::bsky::embed::{images::View as ImageView, record::View as RecordView},
    app::bsky::{
        actor::defs::ProfileViewBasic, feed::defs::PostViewEmbedEnum,
        feed::post::Record as PostRecord,
    },
    records::Record,
};
use bsky::BSky;
use chrono::{DateTime, Local};
use lazy_static::lazy_static;
use regex::Regex;
use rustyline::{error::ReadlineError, DefaultEditor};

lazy_static! {
    static ref BSKY_REGEX: Regex = Regex::new(
        r#"https?://(:?www.|staging.)?bsky.app/profile/(?P<handle>.+)/post/(?P<postid>.+)\??.*"#
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
                    println!("Couldn't find a post URL in there");
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

async fn get_formatted_post(client: &BSky, handle: String, id: String) -> Result<String> {
    let post = client
        .get_post(handle.as_str().to_owned(), id.as_str().to_owned())
        .await?;

    if let Record::AppBskyFeedPost(record) = post.record {
        let content_summary = summarize_post_content(&post.author, &record)?;

        let embed_summary = if let Some(ref embed) = post.embed {
            summarize_post_embeds(embed).await?
        } else {
            String::default()
        };

        Ok(format!("{}{}", content_summary, embed_summary))
    } else {
        bail!("Whoa, got {:?} instead of AppBskyFeedPost", post.record);
    }
}

fn summarize_post_content(author: &ProfileViewBasic, record: &PostRecord) -> Result<String> {
    let display_name = author.display_name.as_ref().unwrap_or(&author.handle);

    let handle = &author.handle;
    let timestamp: DateTime<Local> =
        DateTime::from(DateTime::parse_from_rfc3339(&record.created_at)?);

    Ok(format!(
        "[{}] {} ({}): {}",
        timestamp.format("%Y-%m-%d %I:%M:%S %P"),
        display_name,
        handle,
        record.text
    ))
}

async fn summarize_post_embeds(embed: &PostViewEmbedEnum) -> Result<String> {
    Ok(format!(
        "\n\t{}",
        match embed {
            PostViewEmbedEnum::AppBskyEmbedImagesView(images) => summarize_images(images),
            PostViewEmbedEnum::AppBskyEmbedExternalView(ext) =>
                format!("Links to {} ({})", ext.external.title, ext.external.uri),
            PostViewEmbedEnum::AppBskyEmbedRecordView(record) =>
                summarize_quoted_post(record).await?,
            // I think this is for future use with video/gifs?
            PostViewEmbedEnum::AppBskyEmbedRecordWithMediaView(_media) => "embeds media".to_owned(),
        }
    ))
}

fn summarize_images(view: &ImageView) -> String {
    let descriptions = view
        .images
        .iter()
        .enumerate()
        .map(|(idx, i)| {
            format!(
                "Image {}: {}",
                idx + 1,
                if i.alt.is_empty() {
                    "<no alt text>"
                } else {
                    &i.alt
                }
            )
        })
        .collect::<Vec<_>>();

    descriptions.join("\n\t")
}

async fn summarize_quoted_post(record: &RecordView) -> Result<String> {
    Ok(match record.record {
        atrium_api::app::bsky::embed::record::ViewRecordEnum::ViewRecord(ref rec) => {
            let author = &rec.author;
            if let Record::AppBskyFeedPost(ref record) = rec.value {
                summarize_post_content(author, record)?
            } else {
                bail!("Whoa, got {:?} instead of AppBskyFeedPost", rec.value);
            }
        }
        atrium_api::app::bsky::embed::record::ViewRecordEnum::ViewNotFound(ref _nf) => {
            "Couldn't find embed".into()
        }
        atrium_api::app::bsky::embed::record::ViewRecordEnum::ViewBlocked(ref _blocked) => {
            "Blocked by the embed".into()
        }
        atrium_api::app::bsky::embed::record::ViewRecordEnum::AppBskyFeedDefsGeneratorView(
            ref feed,
        ) => format!(
            "Embeds feed {} by {}",
            feed.display_name, feed.creator.handle
        ),
    })
}

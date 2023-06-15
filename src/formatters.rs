use crate::BSky;
use anyhow::{bail, Result};
use atrium_api::{
    app::bsky::embed::{
        external::View as ExternalView, images::View as ImageView, record::View as RecordView,
        record_with_media::View as MediaView,
    },
    app::bsky::{
        actor::defs::ProfileViewBasic, feed::defs::PostViewEmbedEnum,
        feed::post::Record as PostRecord,
    },
    records::Record,
};
use chrono::{DateTime, Local};
use colored::Colorize;

pub async fn get_formatted_post(client: &BSky, handle: String, id: String) -> Result<String> {
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
        timestamp
            .format("%Y-%m-%d %I:%M:%S %P")
            .to_string()
            .underline(),
        display_name.yellow(),
        handle.blue(),
        record.text
    ))
}

async fn summarize_post_embeds(embed: &PostViewEmbedEnum) -> Result<String> {
    Ok(format!(
        "\n\t{}:\n\t{}",
        "embeds".bright_red(),
        match embed {
            PostViewEmbedEnum::AppBskyEmbedImagesView(images) => summarize_images(images),
            PostViewEmbedEnum::AppBskyEmbedExternalView(ext) => format!(
                "{} Links to {} ({})",
                "-".red(),
                ext.external.title,
                ext.external.uri
            ),
            PostViewEmbedEnum::AppBskyEmbedRecordView(record) =>
                summarize_quoted_post(record).await?,
            // Not sure why a post would use this vs the first two items in this enum - it just
            // contains one of them
            PostViewEmbedEnum::AppBskyEmbedRecordWithMediaView(media) => summarize_media(media),
        }
    ))
}

fn summarize_media(media: &MediaView) -> String {
    match media.media {
        atrium_api::app::bsky::embed::record_with_media::ViewMediaEnum::AppBskyEmbedImagesView(ref images) => summarize_images(images),
        atrium_api::app::bsky::embed::record_with_media::ViewMediaEnum::AppBskyEmbedExternalView(ref external) => summarize_external(external),
    }
}

fn summarize_external(external: &ExternalView) -> String {
    format!(
        "{} Links to {} ({})",
        "-".red(),
        external.external.title,
        external.external.uri
    )
}

fn summarize_images(view: &ImageView) -> String {
    let descriptions = view
        .images
        .iter()
        .enumerate()
        .map(|(idx, i)| {
            format!(
                "{} Image {} alt text: {}",
                "-".red(),
                idx + 1,
                if i.alt.is_empty() {
                    "<no alt text>".into()
                } else {
                    i.alt.replace('\n', "\n\t\t")
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
            "{} Embeds feed {} by {}",
            "-".red(),
            feed.display_name,
            feed.creator.handle
        ),
    })
}

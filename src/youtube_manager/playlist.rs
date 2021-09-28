use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use google_youtube3::{api::PlaylistItemListResponse, client::Result, YouTube};
use hyper::Response;

#[derive(Default)]
pub struct Item {
    pub video_id: String,
    pub title: String,
    pub scheduled_start_time: Option<DateTime<FixedOffset>>,
    pub actual_start_time: Option<DateTime<FixedOffset>>,
}

#[async_trait]
pub trait Playlist {
    async fn items(self: Self) -> Result<Vec<Item>>;
}

struct PlaylistImpl {
    hub: YouTube,
    id: String,
}

pub fn new(hub: YouTube, id: &str) -> impl Playlist {
    PlaylistImpl {
        hub: hub,
        id: id.to_owned(),
    }
}

#[async_trait]
impl Playlist for PlaylistImpl {
    async fn items(self: PlaylistImpl) -> Result<Vec<Item>> {
        let mut list: Vec<Item> = vec![];

        let (_, mut res) = playlist_items(&self.hub, &self.id, &None).await?;
        while let Some(items) = &res.items {
            for item in items {
                let video_id = item
                    .content_details
                    .as_ref()
                    .unwrap()
                    .video_id
                    .as_ref()
                    .unwrap();

                let (_, v) = self
                    .hub
                    .videos()
                    .list(&vec!["liveStreamingDetails".into()])
                    .add_id(video_id)
                    .doit()
                    .await?;

                let mut it = Item {
                    video_id: video_id.to_owned(),
                    title: item
                        .snippet
                        .as_ref()
                        .unwrap()
                        .title
                        .as_ref()
                        .unwrap()
                        .to_owned(),
                    ..Default::default()
                };

                let videos = v.items.unwrap();

                if videos.len() > 0 {
                    let live_streaming_details =
                        videos.get(0).unwrap().live_streaming_details.as_ref();
                    if let Some(details) = live_streaming_details {
                        it.scheduled_start_time = details
                            .scheduled_start_time
                            .as_ref()
                            .map(|d| DateTime::parse_from_rfc3339(&d).unwrap());
                        it.actual_start_time = details
                            .actual_start_time
                            .as_ref()
                            .map(|d| DateTime::parse_from_rfc3339(&d).unwrap());
                    }
                }
                list.push(it)
            }
            if res.next_page_token.is_some() {
                res = playlist_items(&self.hub, &self.id, &res.next_page_token)
                    .await?
                    .1;
            } else {
                res.items = None;
            }
        }

        Ok(list)
    }
}

async fn playlist_items(
    hub: &YouTube,
    playlist_id: &str,
    next_page_token: &Option<String>,
) -> Result<(Response<hyper::body::Body>, PlaylistItemListResponse)> {
    let mut req = hub
        .playlist_items()
        .list(&vec![
            "snippet".into(),
            "id".into(),
            "contentDetails".into(),
        ])
        .playlist_id(playlist_id);
    if let Some(next) = next_page_token {
        req = req.page_token(&next);
    }
    req.doit().await
}

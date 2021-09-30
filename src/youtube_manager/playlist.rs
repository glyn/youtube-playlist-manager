use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use google_youtube3::{
    api::Scope,
    api::{PlaylistItem, PlaylistItemListResponse, PlaylistItemSnippet},
    client::Result,
    YouTube,
};
use hyper::Response;
use std::{cmp::Ordering, fmt};

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Item {
    pub video_id: String,
    playlist_item_id: String,
    pub title: String,
    pub scheduled_start_time: Option<DateTime<FixedOffset>>,
    pub actual_start_time: Option<DateTime<FixedOffset>>,
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}: {})", self.video_id, self.title)
    }
}

#[async_trait]
pub trait Playlist {
    /// items returns a vector of the items in the playlist
    async fn items(self: &Self) -> Result<Vec<Item>>;

    /// sort orders the playlist as follows:
    /// * streamed videos in reverse chronological order (newest first), followed
    /// * not-yet-streamed videos again in reverse chronological order (newest first), followed by
    /// * videos for which there is no time information
    async fn sort(self: Self) -> Result<()>;

    /// prune removes any invalid videos from the playlist. These include:
    /// * deleted videos
    /// * videos for which there is no time information (e.g. with no live streaming information such as scheduled start time)
    async fn prune(self: &Self) -> Result<()>;
}

struct PlaylistImpl {
    hub: YouTube,
    id: String,
}

/// new constructs a Playlist trait implementation for manipulating the playlist with the given playlist id.
pub fn new(hub: YouTube, id: &str) -> impl Playlist {
    PlaylistImpl {
        hub: hub,
        id: id.to_owned(),
    }
}

#[async_trait]
impl Playlist for PlaylistImpl {
    async fn items(self: &PlaylistImpl) -> Result<Vec<Item>> {
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
                    playlist_item_id: item.id.as_ref().unwrap().to_owned(),
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

    async fn sort(self: Self) -> Result<()> {
        let mut items = self.items().await?;
        let original_items = items.clone();
        sort_items(&mut items);
        if items != original_items {
            // Re-order the playlist to match the sorted items.
            for (n, item) in items.iter().enumerate() {
                self.hub
                    .playlist_items()
                    .update(PlaylistItem {
                        id: Some(item.playlist_item_id.clone()),
                        snippet: Some(PlaylistItemSnippet {
                            //playlist_id: Some(self.id.clone()), //needed?
                            position: Some(n as u32),
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .add_scope(Scope::Full)
                    .doit()
                    .await?;
            }
        }
        Ok(())
    }

    async fn prune(self: &Self) -> Result<()> {
        for item in self.items().await? {
            if item.scheduled_start_time.is_none() {
                eprintln!("Deleting playlist item for video {}", item);
                self.hub
                    .playlist_items()
                    .delete(&item.playlist_item_id)
                    .add_scope(Scope::ForceSsl)
                    .add_scope(Scope::Partner)
                    .add_scope(Scope::Full)
                    .add_scope(Scope::Upload)
                    .add_scope(Scope::ChannelMembershipCreator)
                    .add_scope(Scope::PartnerChannelAudit)
                    .doit()
                    .await?;
            }
        }
        Ok(())
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

fn sort_items(items: &mut Vec<Item>) {
    items.sort_by(|v, w| {
        println!("v: {:?}\nw: {:?}", v, w);
        if v.actual_start_time.is_some() {
            if w.actual_start_time.is_some() {
                // Order streamed items in reverse chronological order
                v.actual_start_time
                    .unwrap()
                    .cmp(&w.actual_start_time.unwrap())
                    .reverse()
            } else {
                // Order streamed items before unstreamed items
                Ordering::Less
            }
        } else if w.actual_start_time.is_some() {
            // Order streamed items before unstreamed items
            Ordering::Greater
        } else if v.scheduled_start_time.is_some() {
            if w.scheduled_start_time.is_some() {
                // Order unstreamed, scheduled items in reverse chronological order
                v.scheduled_start_time
                    .unwrap()
                    .cmp(&w.scheduled_start_time.unwrap())
                    .reverse()
            } else {
                // Order unstreamed, scheduled items before unstreamed, unscheduled items
                Ordering::Less
            }
        } else if w.scheduled_start_time.is_some() {
            // Order unstreamed, scheduled items before unstreamed, unscheduled items
            Ordering::Greater
        } else {
            // Leave the order of unstreamed, unscheduled items alone
            Ordering::Equal
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_items_empty() {
        let mut v = vec![];
        sort_items(&mut v);
        assert_eq!(v, vec![]);
    }

    #[test]
    fn sort_items_unstreamed_scheduled() {
        let mut v = vec![new_scheduled_item(1), new_scheduled_item(2)];
        sort_items(&mut v);
        assert_video_ids(v, vec!["v2", "v1"]);

        v = vec![new_scheduled_item(2), new_scheduled_item(1)];
        sort_items(&mut v);
        assert_video_ids(v, vec!["v2", "v1"]);
    }

    #[test]
    fn sort_items_scheduled_before_unstreamed_unscheduled() {
        let mut v = vec![new_item(1), new_scheduled_item(2)];
        sort_items(&mut v);
        assert_video_ids(v, vec!["v2", "v1"]);

        v = vec![new_scheduled_item(1), new_item(2)];
        sort_items(&mut v);
        assert_video_ids(v, vec!["v1", "v2"]);
    }

    #[test]
    fn sort_items_streamed() {
        let mut v = vec![new_streamed_item(1), new_streamed_item(2)];
        sort_items(&mut v);
        assert_video_ids(v, vec!["v2", "v1"]);

        v = vec![new_streamed_item(2), new_streamed_item(1)];
        sort_items(&mut v);
        assert_video_ids(v, vec!["v2", "v1"]);
    }

    #[test]
    fn sort_items_streamed_before_scheduled() {
        let mut v = vec![new_scheduled_item(2), new_streamed_item(1)];
        sort_items(&mut v);
        assert_video_ids(v, vec!["v1", "v2"]);

        v = vec![new_streamed_item(1), new_scheduled_item(2)];
        sort_items(&mut v);
        assert_video_ids(v, vec!["v1", "v2"]);
    }

    fn new_scheduled_item(n: u32) -> Item {
        let mut i = new_item(n);
        i.scheduled_start_time =
            Some(DateTime::parse_from_rfc3339(&format!("2021-09-30T10:55:0{}+01:00", n)).unwrap());
        i
    }

    fn new_streamed_item(n: u32) -> Item {
        let mut i = new_scheduled_item(n);
        i.actual_start_time =
            Some(DateTime::parse_from_rfc3339(&format!("2021-09-30T10:56:0{}+01:00", n)).unwrap());
        i
    }

    fn new_item(n: u32) -> Item {
        assert!(n <= 9);
        Item {
            video_id: format!("v{}", n).to_owned(),
            playlist_item_id: format!("pii{}", n).to_owned(),
            title: format!("video {}", n).to_owned(),
            ..Default::default()
        }
    }

    fn assert_video_ids(v: Vec<Item>, expected: Vec<&str>) {
        assert_eq!(v.len(), expected.len());
        for (n, i) in v.iter().enumerate() {
            assert_eq!(i.video_id, expected.get(n).unwrap().to_owned());
        }
    }
}

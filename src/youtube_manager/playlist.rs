use async_trait::async_trait;
use chrono::DateTime;
use chrono_tz::Tz;
use google_youtube3::{
    api::Scope,
    api::{PlaylistItem, PlaylistItemListResponse, PlaylistItemSnippet, ResourceId},
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
    pub scheduled_start_time: Option<DateTime<Tz>>,
    pub actual_start_time: Option<DateTime<Tz>>,
    pub published_at: Option<DateTime<Tz>>,
    pub blocked: bool,
}

pub trait ItemProperties {
    /// viewable videos are either streamed or uploaded, but not blocked
    fn viewable(self: &Self) -> bool;

    fn viewable_time(self: &Self) -> Option<DateTime<Tz>>;

    /// available videos are either streamed or uploaded and may be blocked
    fn available(self: &Self) -> bool;

    fn available_time(self: &Self) -> Option<DateTime<Tz>>;
}

impl ItemProperties for Item {
    fn viewable(self: &Item) -> bool {
        self.viewable_time().is_some()
    }

    fn viewable_time(self: &Self) -> Option<DateTime<Tz>> {
        if self.blocked {
            None
        } else {
            self.available_time()
        }
    }

    fn available(self: &Item) -> bool {
        self.available_time().is_some()
    }

    fn available_time(self: &Self) -> Option<DateTime<Tz>> {
        if self.actual_start_time.is_some() {
            // streamed videos have an actual start time
            self.actual_start_time
        } else if self.published_at.is_some()
            && self.actual_start_time.is_none()
            && self.scheduled_start_time.is_none()
        {
            // uploaded videos have a published time, but not an actual or scheduled start time
            self.published_at
        } else {
            None
        }
    }
}

pub trait Pruning {
    /// prune returns None if the video should not be pruned. If the video should
    /// be pruned, it returns some string which gives the reason for pruning the video.
    fn prune(self: &Self) -> Option<String>;
}

impl Pruning for Item {
    fn prune(self: &Item) -> Option<String> {
        if self.blocked {
            Some("blocked".to_string())
        } else if self.scheduled_start_time.is_none() && self.published_at.is_none() {
            Some("unscheduled and unpublished".to_string())
        } else {
            None
        }
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} {}", self.video_id, self.title, time(&self))
    }
}

#[async_trait]
pub trait Playlist {
    /// items returns a vector of the items in the playlist.
    async fn items(self: &Self) -> Result<Vec<Item>>;

    /// sort orders the playlist as follows:
    /// * streamed videos in reverse chronological order (newest first), followed
    /// * not-yet-streamed videos again in reverse chronological order (newest first), followed by
    /// * videos for which there is no time information.
    async fn sort(self: &Self) -> Result<()>;

    /// prune removes any invalid videos from the playlist. These include:
    /// * deleted videos
    /// * videos for which there is no time information (e.g. with no live streaming information such as scheduled start time).
    async fn prune(self: &Self, max_catch_up: usize) -> Result<()>;

    // print prints the playlist to standard error.
    async fn print(self: &Self) -> Result<()>;
}

struct PlaylistImpl {
    hub: YouTube,
    id: String,
    dry_run: bool,
    debug: bool,
    timezone: Tz,
}

/// new constructs a Playlist trait implementation for manipulating the playlist with the given playlist id.
/// If dry-run is true, information will be printed out but the playlist will not be updated on YouTube.
/// Debugging information is printed if and only if debug is true.
pub fn new(hub: YouTube, id: &str, time_zone: String, dry_run: bool, debug: bool) -> impl Playlist {
    let tz: Tz = match time_zone.parse() {
        Ok(v) => v,
        Err(e) => {
            panic!("Invalid timezone: {}", e);
        }
    };

    PlaylistImpl {
        hub: hub,
        id: id.to_owned(),
        dry_run: dry_run,
        debug: debug,
        timezone: tz,
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
                    .list(&vec![
                        "liveStreamingDetails".into(),
                        "contentDetails".into(),
                    ])
                    .add_id(video_id)
                    .doit()
                    .await?;

                let mut it =
                    Item {
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
                        published_at: item.snippet.as_ref().unwrap().published_at.as_ref().map(
                            |d| {
                                DateTime::parse_from_rfc3339(&d)
                                    .unwrap()
                                    .with_timezone(&self.timezone)
                            },
                        ),
                        ..Default::default()
                    };

                let videos = v.items.unwrap();

                if videos.len() > 0 {
                    let live_streaming_details =
                        videos.get(0).unwrap().live_streaming_details.as_ref();
                    if let Some(details) = live_streaming_details {
                        it.scheduled_start_time = details.scheduled_start_time.as_ref().map(|d| {
                            DateTime::parse_from_rfc3339(&d)
                                .unwrap()
                                .with_timezone(&self.timezone)
                        });
                        it.actual_start_time = details.actual_start_time.as_ref().map(|d| {
                            DateTime::parse_from_rfc3339(&d)
                                .unwrap()
                                .with_timezone(&self.timezone)
                        });
                    }
                    if let Some(content_details) = videos.get(0).unwrap().content_details.as_ref() {
                        if let Some(restriction) = content_details.region_restriction.as_ref() {
                            if let Some(blocked) = restriction.blocked.as_ref() {
                                it.blocked = !blocked.is_empty();
                            }
                        }
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

        if self.debug {
            eprintln!("playlist items: {:?}", list);
        }
        Ok(list)
    }

    async fn sort(self: &Self) -> Result<()> {
        let mut items = self.items().await?;
        let original_items = items.clone();
        sort_items(&mut items);
        if items == original_items {
            eprintln!("Playlist is already in the correct order");
            Ok(())
        } else {
            if self.dry_run {
                eprintln!("Playlist would be sorted into this order:");
                print(items)?;
                eprintln!("");
            } else {
                // Re-order the playlist to match the sorted items.
                for (n, item) in items.iter().enumerate() {
                    self.hub
                        .playlist_items()
                        .update(PlaylistItem {
                            id: Some(item.playlist_item_id.clone()),
                            snippet: Some(PlaylistItemSnippet {
                                playlist_id: Some(self.id.clone()),
                                resource_id: Some(ResourceId {
                                    kind: Some("youtube#video".to_owned()),
                                    video_id: Some(item.video_id.clone()),
                                    ..Default::default()
                                }),
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
    }

    async fn prune(self: &Self, max_streamed: usize) -> Result<()> {
        // Remove surplus and other unwanted videos from the playlist
        self.sort().await?;
        let mut n = 0;
        for i in self.items().await? {
            if let Some(prune_reason) = i.prune() {
                prune_and_log_item(&self.hub, &i, prune_reason, self.dry_run).await?
            } else if i.viewable() {
                n += 1;
                if n > max_streamed {
                    prune_and_log_item(&self.hub, &i, "surplus".to_string(), self.dry_run).await?
                }
            }
        }
        Ok(())
    }

    async fn print(self: &Self) -> Result<()> {
        print(self.items().await?)
    }
}

fn print(items: Vec<Item>) -> Result<()> {
    for video in items {
        eprintln!("{}", video);
    }
    Ok(())
}

fn time(video: &Item) -> String {
    if video.viewable() {
        format!(
            "{} on {}",
            if video.scheduled_start_time.is_some() {
                "streamed"
            } else {
                "uploaded"
            },
            video.viewable_time().unwrap().to_rfc2822()
        )
    } else if video.available() {
        format!(
            "{} on {} but **blocked**",
            if video.scheduled_start_time.is_some() {
                "streamed"
            } else {
                "uploaded"
            },
            video.available_time().unwrap().to_rfc2822()
        )
    } else if video.scheduled_start_time.is_some() {
        format!(
            "scheduled for {}",
            video.scheduled_start_time.unwrap().to_rfc2822()
        )
    } else {
        "invalid".to_string()
    }
}

async fn prune_and_log_item(hub: &YouTube, i: &Item, reason: String, dry_run: bool) -> Result<()> {
    if !dry_run {
        eprintln!("Removing {} video from playlist: {}", reason, i);
        prune_item(&hub, &i.playlist_item_id).await?;
    } else {
        eprintln!("Video {} would be removed from playlist: {}", reason, i);
    }
    Ok(())
}

async fn prune_item(hub: &YouTube, playlist_item_id: &String) -> Result<()> {
    hub.playlist_items()
        .delete(&playlist_item_id)
        .add_scope(Scope::Full)
        .doit()
        .await?;
    Ok(())
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
        // println!("v: {:?}\nw: {:?}", v, w);
        if v.viewable() {
            if w.viewable() {
                // Order viewable items in reverse chronological order
                v.viewable_time()
                    .unwrap()
                    .cmp(&w.viewable_time().unwrap())
                    .reverse()
            } else {
                // Order viewable items before unviewable items
                Ordering::Less
            }
        } else if w.viewable() {
            // Order viewable items before unviewabled items
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
        } else if v.available() {
            if w.available() {
                // Order available items in reverse chronological order
                v.available_time()
                    .unwrap()
                    .cmp(&w.available_time().unwrap())
                    .reverse()
            } else {
                // Order unavailable items before available items
                Ordering::Greater
            }
        } else if w.available() {
            // Order unavailable items before available items
            Ordering::Less
        } else {
            // Leave the order of invalid items alone
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
    fn sort_items_by_type() {
        // items are sorted in this order:
        // 1. streamed or uploaded (i.e. published but not streamed)
        // 2. scheduled
        // 3. unstreamed, unscheduled, and unpublished
        // 4. blocked
        // Note: that items in 3 and 4 are subject to pruning, when this is requested.
        transitive_less_than(vec![
            new_streamed_item,
            new_scheduled_item,
            new_invalid_item,
            new_blocked_item,
        ]);

        transitive_less_than(vec![
            new_uploaded_item,
            new_scheduled_item,
            new_invalid_item,
            new_blocked_item,
        ]);
    }

    #[test]
    fn sort_items_of_the_same_type_reverse_chronologically() {
        sort_reverse_chronologically(new_streamed_item);
        sort_reverse_chronologically(new_uploaded_item);
        sort_reverse_chronologically(new_scheduled_item);
        // (invalid items cannot be sorted reverse chronologically as they have no times associated with them)
        sort_reverse_chronologically(new_blocked_item);
    }

    fn sort_reverse_chronologically(f: fn(u32) -> (Item, &'static str)) {
        let message = format!("{} not sorted reverse chronologically", f(0).1);
        let mut v = vec![f(1).0, f(2).0];
        sort_items(&mut v);
        assert_video_ids_with_message(v, vec!["v2", "v1"], &message);

        v = vec![f(2).0, f(1).0];
        sort_items(&mut v);
        assert_video_ids_with_message(v, vec!["v2", "v1"], &message);
    }

    fn transitive_less_than(order: Vec<fn(u32) -> (Item, &'static str)>) {
        let n = order.len();
        for i in 0..n {
            let f = order.get(i).unwrap();
            for j in (i + 1)..n {
                let g = order.get(j).unwrap();
                less_than(*f, *g);
            }
        }
    }

    fn less_than(lower: fn(u32) -> (Item, &'static str), higher: fn(u32) -> (Item, &'static str)) {
        let message = format!("{} is not less than {}", lower(0).1, higher(0).1);

        // lower item is less than higher item, regardless of chronological order
        let mut v = vec![higher(2).0, lower(1).0];
        sort_items(&mut v);
        assert_video_ids_with_message(v, vec!["v1", "v2"], &message);

        v = vec![lower(1).0, higher(2).0];
        sort_items(&mut v);
        assert_video_ids_with_message(v, vec!["v1", "v2"], &message);

        v = vec![higher(1).0, lower(2).0];
        sort_items(&mut v);
        assert_video_ids_with_message(v, vec!["v2", "v1"], &message);

        v = vec![lower(2).0, higher(1).0];
        sort_items(&mut v);
        assert_video_ids_with_message(v, vec!["v2", "v1"], &message);
    }

    #[test]
    fn sort_items_invalid() {
        // sort should not change the order of invalid items
        let mut v = vec![new_invalid_item(1).0, new_invalid_item(2).0];
        sort_items(&mut v);
        assert_video_ids(v, vec!["v1", "v2"]);

        v = vec![new_invalid_item(2).0, new_invalid_item(1).0];
        sort_items(&mut v);
        assert_video_ids(v, vec!["v2", "v1"]);
    }

    #[test]
    fn prune_item() {
        assert!(new_scheduled_item(1).0.prune().is_none());
        assert!(new_streamed_item(1).0.prune().is_none());
        assert!(new_uploaded_item(1).0.prune().is_none());

        assert!(new_blocked_item(1).0.prune().is_some());
        assert!(new_invalid_item(1).0.prune().is_some());
    }

    fn new_scheduled_item(n: u32) -> (Item, &'static str) {
        let mut i = new_item(n);
        i.scheduled_start_time = Some(
            DateTime::parse_from_rfc3339(&format!("2021-09-30T10:55:0{}+01:00", n))
                .unwrap()
                .with_timezone(&chrono_tz::UTC),
        );
        (i, "scheduled item")
    }

    fn new_streamed_item(n: u32) -> (Item, &'static str) {
        let mut i = new_published_item(n);
        i.actual_start_time = Some(
            DateTime::parse_from_rfc3339(&format!("2021-09-30T10:56:0{}+01:00", n))
                .unwrap()
                .with_timezone(&chrono_tz::UTC),
        );
        (i, "streamed item")
    }

    fn new_uploaded_item(n: u32) -> (Item, &'static str) {
        (new_published_item(n), "uploaded item")
    }

    fn new_published_item(n: u32) -> Item {
        let mut i = new_item(n);
        i.published_at = Some(
            DateTime::parse_from_rfc3339(&format!("2021-09-30T10:56:0{}+01:00", n))
                .unwrap()
                .with_timezone(&chrono_tz::UTC),
        );
        i
    }

    fn new_blocked_item(n: u32) -> (Item, &'static str) {
        let (mut i, _) = new_streamed_item(n);
        i.blocked = true;
        (i, "blocked item")
    }

    fn new_invalid_item(n: u32) -> (Item, &'static str) {
        (new_item(n), "invalid item")
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
        assert_video_ids_with_message(v, expected, &"".to_string());
    }

    fn assert_video_ids_with_message(v: Vec<Item>, expected: Vec<&str>, message: &String) {
        assert_eq!(v.len(), expected.len());
        for (n, i) in v.iter().enumerate() {
            assert_eq!(
                i.video_id,
                expected.get(n).unwrap().to_owned(),
                "{}",
                message
            );
        }
    }
}

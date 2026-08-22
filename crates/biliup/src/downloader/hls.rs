use crate::downloader::error::{Error, Result};
use crate::downloader::util::{LifecycleFile, Segmentable};
use m3u8_rs::{MediaPlaylist, Playlist};

use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use url::Url;

use crate::client::StatelessClient;

const MIN_PLAYLIST_POLL_INTERVAL: Duration = Duration::from_secs(1);

fn parse_media_playlist(bytes: &[u8]) -> Result<MediaPlaylist> {
    m3u8_rs::parse_media_playlist(bytes)
        .map(|(_, playlist)| playlist)
        .map_err(|error| Error::Custom(format!("Unable to parse media playlist content: {error}")))
}

fn playlist_poll_interval(playlist: &MediaPlaylist) -> Duration {
    Duration::from_secs(playlist.target_duration).max(MIN_PLAYLIST_POLL_INTERVAL)
}

fn playlist_should_refresh(playlist: &MediaPlaylist) -> bool {
    !playlist.end_list
}

pub async fn download(
    url: &str,
    client: &StatelessClient,
    file: LifecycleFile<'_>,
    mut splitting: Segmentable,
) -> Result<()> {
    info!("Downloading {}...", url);
    let resp = client.retryable(url).await?;
    info!("{}", resp.status());
    // let mut resp = resp.bytes_stream();
    let bytes = resp.bytes().await?;
    let mut ts_file = TsFile::new(file)?;

    let mut media_url = Url::parse(url)?;
    let mut pl = match m3u8_rs::parse_playlist(&bytes) {
        Ok((_i, Playlist::MasterPlaylist(pl))) => {
            info!("Master playlist:\n{:#?}", pl);
            // Pick the highest-bandwidth playable variant. The first variant is not
            // necessarily the best quality (e.g. Twitch orders transcodes ahead of the
            // source), so prefer the highest-bandwidth stream that carries a resolution.
            // Skip I-frame (trick-play) streams, which are not full playable renditions.
            // Fall back to the highest-bandwidth non-I-frame variant, then the first one.
            let best = pl
                .variants
                .iter()
                .filter(|v| !v.is_i_frame && v.resolution.is_some())
                .max_by_key(|v| v.bandwidth)
                .or_else(|| {
                    pl.variants
                        .iter()
                        .filter(|v| !v.is_i_frame)
                        .max_by_key(|v| v.bandwidth)
                })
                .unwrap_or(&pl.variants[0]);
            info!(
                "Selected variant: bandwidth={}, resolution={:?}, video={:?}",
                best.bandwidth, best.resolution, best.video
            );
            media_url = media_url.join(&best.uri)?;
            info!("media url: {media_url}");
            let resp = client.retryable(media_url.as_str()).await?;
            let bs = resp.bytes().await?;
            parse_media_playlist(&bs)?
        }
        Ok((_i, Playlist::MediaPlaylist(pl))) => {
            info!("Media playlist:\n{:#?}", pl);
            info!("index {}", pl.media_sequence);
            pl
        }
        Err(e) => return Err(Error::Custom(format!("Parsing playlist error: {e}"))),
    };
    let mut previous_last_segment = 0;
    let mut last_playlist_load = Instant::now();
    loop {
        if pl.segments.is_empty() {
            debug!("Segments array is empty - waiting for playlist update");
        }
        let mut seq = pl.media_sequence;
        for segment in &pl.segments {
            if seq > previous_last_segment {
                if (previous_last_segment > 0) && (seq > (previous_last_segment + 1)) {
                    warn!("SEGMENT INFO SKIPPED");
                }
                debug!("Yield segment");
                if segment.discontinuity {
                    warn!("#EXT-X-DISCONTINUITY");
                    ts_file.create_new()?;
                    // splitting = Segment::from_seg(splitting);
                    splitting.reset();
                }
                let length = download_to_file(
                    media_url.join(&segment.uri)?,
                    client,
                    &mut ts_file.buf_writer,
                )
                .await?;
                splitting.increase_size(length);
                splitting.increase_time(Duration::from_secs(segment.duration as u64));
                if splitting.needed() {
                    ts_file.create_new()?;
                    splitting.reset();
                }
                previous_last_segment = seq;
            }
            seq += 1;
        }

        if !playlist_should_refresh(&pl) {
            info!("#EXT-X-ENDLIST received - stream finished");
            break;
        }

        let poll_interval = playlist_poll_interval(&pl);
        let refresh_delay = poll_interval.saturating_sub(last_playlist_load.elapsed());
        if !refresh_delay.is_zero() {
            debug!("Waiting {refresh_delay:?} before refreshing media playlist");
            tokio::time::sleep(refresh_delay).await;
        }

        let resp = client.retryable(media_url.as_str()).await?;
        let bs = resp.bytes().await?;
        pl = parse_media_playlist(&bs)?;
        last_playlist_load = Instant::now();
    }
    info!("Done...");
    Ok(())
}

async fn download_to_file(url: Url, client: &StatelessClient, out: &mut impl Write) -> Result<u64> {
    debug!("url: {url}");
    let mut response = client.retryable(url.as_str()).await?;
    let mut length: u64 = 0;
    while let Some(chunk) = response.chunk().await? {
        length += chunk.len() as u64;
        out.write_all(&chunk)?;
    }
    // let mut out = File::options()
    //     .append(true)
    //     .open(format!("{file_name}.ts"))?;
    // let length = response.copy_to(out)?;
    Ok(length)
}

pub struct TsFile<'a> {
    pub buf_writer: BufWriter<File>,
    pub file: LifecycleFile<'a>,
}

impl<'a> TsFile<'a> {
    pub fn new(mut file: LifecycleFile<'a>) -> std::io::Result<Self> {
        let path = file.create()?;
        Ok(Self {
            buf_writer: Self::create(path)?,
            file,
        })
    }

    pub fn create_new(&mut self) -> std::io::Result<()> {
        self.file.rename();
        let path = self.file.create()?;
        self.buf_writer = Self::create(path)?;
        Ok(())
    }

    fn create<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<BufWriter<File>> {
        let path = path.as_ref();
        let out = match File::create(path) {
            Ok(o) => o,
            Err(e) => {
                return Err(std::io::Error::new(
                    e.kind(),
                    format!("Unable to create file {}", path.display()),
                ));
            }
        };
        info!("create file {}", path.display());
        Ok(BufWriter::new(out))
    }
}

impl Drop for TsFile<'_> {
    fn drop(&mut self) {
        self.file.rename()
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_media_playlist, playlist_poll_interval, playlist_should_refresh};
    use m3u8_rs::MediaPlaylist;
    use reqwest::Url;
    use std::time::Duration;

    #[test]
    fn test_url() -> Result<(), Box<dyn std::error::Error>> {
        let url = Url::parse("h://host.path/to/remote/resource.m3u8")?;
        let scheme = url.scheme();
        let new_url = url.join("http://path.host/remote/resource.ts")?;
        println!("{url}, {scheme}");
        println!("{new_url}, {scheme}");
        Ok(())
    }

    #[test]
    fn it_works() -> Result<(), Box<dyn std::error::Error>> {
        // download(
        //     "test.ts")?;
        Ok(())
    }

    #[test]
    fn playlist_poll_interval_uses_target_duration() {
        let playlist = MediaPlaylist {
            target_duration: 6,
            ..MediaPlaylist::default()
        };

        assert_eq!(playlist_poll_interval(&playlist), Duration::from_secs(6));
    }

    #[test]
    fn playlist_poll_interval_has_one_second_minimum() {
        assert_eq!(
            playlist_poll_interval(&MediaPlaylist::default()),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn parse_media_playlist_preserves_end_list() {
        let playlist = parse_media_playlist(
            b"#EXTM3U\n\
              #EXT-X-TARGETDURATION:6\n\
              #EXT-X-MEDIA-SEQUENCE:7\n\
              #EXTINF:6.0,\n\
              7.ts\n\
              #EXT-X-ENDLIST\n",
        )
        .expect("valid media playlist should parse");

        assert!(playlist.end_list);
        assert!(!playlist_should_refresh(&playlist));
        assert_eq!(playlist.segments.len(), 1);
    }

    #[test]
    fn parse_media_playlist_returns_error_for_invalid_content() {
        let error = parse_media_playlist(b"not a media playlist")
            .expect_err("invalid media playlist should return an error");

        assert!(error.to_string().contains("Unable to parse media playlist"));
    }
}

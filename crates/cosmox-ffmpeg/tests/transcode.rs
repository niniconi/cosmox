#![cfg(feature = "ffmpeg-tests")]
use cosmox_ffmpeg::streamer::transcode::{TranscodeTask, Transcoder};

use crate::common::media;

mod common;

#[test]
pub fn test_transcode() {
    let input = media(
        "https://repo.jellyfin.org/test-videos/SDR/AVC/Test%20Jellyfin%201080p%20AVC%203M.mp4",
    )
    .unwrap()
    .to_string_lossy()
    .into_owned();
    Transcoder::transcode_video_with_resolution_change(TranscodeTask {
        input_path: input.as_str(),
        output_path: "/tmp/out.m3u8",
        video: None,
        audio_copy: true,
        use_hardware: true,
    })
    .unwrap();
}

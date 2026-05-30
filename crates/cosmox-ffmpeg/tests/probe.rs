#![cfg(feature = "ffmpeg-tests")]

use cosmox_ffmpeg::probe::get_metadata;

use crate::common::media;

mod common;

#[test]
pub fn probe() {
    let video = media(
        "https://repo.jellyfin.org/test-videos/SDR/AVC/Test%20Jellyfin%201080p%20AVC%203M.mp4",
    )
    .unwrap();

    let metadata = get_metadata(&video).unwrap();
    println!("{metadata:#?}");
}

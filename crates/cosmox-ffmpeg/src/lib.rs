//!
//! ```plaintext
//! ========================================================================================================
//!                                      VIBARY SYSTEM ARCHITECTURE
//! ========================================================================================================

//!     [ CLIENT / FRONTEND ]
//!       (mpv / Browser)
//!          │       ▲
//!          │       │  (1) Request Playback with Capabilities
//!          │       │  (e.g., Client=mpv, Protocol=HTTP-Range, Bandwidth=100Mbps)
//!          ▼       │
//! +──────────────────────────────────────────────────────────────────────────────────────────────────────+
//! |  WEB SERVER / TOKIO ASYNC RUNTIME (Asynchronous Network Plane)                                       |
//! |                                                                                                      |
//! |   +───────────────────────────+                                                                      |
//! |   |  Playback Policy Router   | ──(2) Analyze Match ───────────────────────────────────────────────┐ |
//! |   +───────────────────────────+                                                                    │ |
//! |                 │                                                                                  │ |
//! |                 ├─[ MATCH 1: Direct Play ]──► Route to Async Static File Server ───┐               │ |
//! |                 │                             (Serves raw file via HTTP 206 Range) │               │ |
//! |                 │                                                                  ▼               │ |
//! |                 └─[ MATCH 2 & 3: Transcode/Remux ]                                 │               │ |
//! |                                                                                    │               │ |
//! |   +───────────────────────────+                                                    │               │ |
//! |   |     Segment Provider      | ◄──(7) Stream Bytes via Zero-Copy Async I/O ───────┤               │ |
//! |   |  (Streaming Route Handler)|     (tokio::fs::File -> ReaderStream -> Socket)    │               │ |
//! |   +───────────────────────────+                                                    │               │ |
//! +────────────────────────────────────────────────────────────────────────────────────┼───────────────┼─+
//!                                                                                      │               │
//!                                                                                      │               │ (3) Trigger Task
//!                                                                                      │               ▼
//! +───────────────────────────────────────────────────────────────────────────────────┼─+ +───────────────────────────────+
//! |  HIGH-SPEED MEMORY FILE SYSTEM (VFS)                                              │ | |  BACKGROUND TASK COORDINATOR  |
//! |                                                                                   │ | |  (Asynchronous Session Mgr)   |
//! |   /tmp/vibary/stream_session_123/                                                 │ | +───────────────────────────────+
//! |   ├── master.m3u8  ◄───────────────────────────┐                                  │ |                 │
//! |   ├── init.mp4     ◄───────────────────────────┼──────────┐                       │ |                 │ (4) spawn_child
//! |   ├── seg-001.m4s  ◄── [ LIVE TRANSCODE PROD ] │          │                       │ |                 ▼
//! |   └── seg-002.m4s  ◄───────────────────────────┼────┐     │                       │ | +───────────────────────────────+
//! |                                                │    │     │                       │ | |      tokio::process::Child    |
//! |   [ PHYSICAL HDD / SSD STORAGE ]               │    │     │                       │ | +───────────────────────────────+
//! |   └── /media/movies/Avatar.mkv ────────────────┼────┼─────┼───────────────────────┘ |                 │
//! +────────────────────────────────────────────────┼────┼─────┼─────────────────────────+                 │ (5) Continuous
//!                                                  │    │     │                                           │     Monitoring
//!                                                  │    │     │                                           ▼
//! +────────────────────────────────────────────────┼────┼─────┼───────────────────────────────────────────+
//! |  EXTERNAL COMPUTE PIPELINE (Heavy OS Process)  │    │     │                                           |
//! |                                                │    │     │                                           |
//! |   $ ffmpeg -i Avatar.mkv -c:v libx264 -vf subtitles=sub.ass -f hls -hls_time 4 /tmp/stream_123/ ...   |
//! |                                                                                                       |
//! |   +───────────────+       +───────────────+    │    │     │       +───────────────+                   |
//! |   |   DEMUXER     | ───►  | DECODER (GPU) | ───┼────┼─────┼────►  | SUB ENCODER   |                   |
//! |   | (Extract Raw) |       | (HW/SW Dec)   |    │    │     │       | (Burn/Embed)  |                   |
//! |   +───────────────+       +───────────────+    │    │     │       +───────────────+                   |
//! |                                                │    │     │               │                           |
//! |                                                │    │     │               ▼                           |
//! |   +───────────────+       +───────────────+    │    │     │       +───────────────+                   |
//! |   |  M3U8 WRITER  |       |   MUXER/SEG   | ───┴────┴─────┴─────  | ENCODER (GPU) |                   |
//! |   | (Map Dynamic) | ◄───  | (fMP4/TS Pack)|                       | (H.264/AAC HW)|                   |
//! |   +───────────────+       +───────────────+                       +───────────────+                   |
//! |                                                                                                       |
//! |   * Logs progress to stderr ───► [ Parsed by Tokio Thread to Update Global Session State Machine ]    |
//! +──────────────────────────────────────────────────────────────────────────────────────────────────────+
//! ```

//! ## Implementation Status
//!
//! | Component | Status |
//! |-----------|--------|
//! | FFmpeg transcode pipeline (`transcode.rs`) | Implemented (ffmpeg-next Rust API) |
//! | Media probe (`probe.rs`) | Implemented |
//! | Hardware acceleration detection (`hw_accel.rs`) | Implemented |
//! | Session lifecycle (`session_registry.rs`) | Skeleton (TODO: integrate transcode) |
//! | Tone mapping / Trickplay / Subtitle processing | Design only, not implemented |
//! | **Playback Policy Router** | **Planned** (not implemented) |
//! | **HLS Segment Provider** | **Planned** (not implemented) |
//! | **VFS streaming layer** | **Planned** (not implemented) |
//!
//! ## Architecture Design Notes
//!
//! ### 1. Playback Decision (Playback Policy Router)
//!
//! The decision whether to direct-play or transcode depends on a combination of:
//!
//! - **Container format** (mp4/mkv/avi/ts) — does the client support it?
//! - **Video codec** (H.264/H.265/VP9/AV1) — can the client decode it natively?
//! - **Audio codec** (AAC/AC3/EAC3/DTS/TrueHD/FLAC) — pass-through or transcode?
//! - **Subtitle format** (SRT/ASS/PGS/VOBSUB) — can the client render it? If ASS with complex styles, burn-in required.
//! - **Resolution & bitrate** — does it exceed the client's or network's capacity?
//! - **Client capabilities** — declared via headers or a capabilities profile (e.g. HLS only, supports MPEG-DASH, maximum resolution, codec blacklist).
//!
//! This is not a simple match; it is a constraint-satisfaction problem with fallback chains.
//!
//! ### 2. Audio Pipeline
//!
//! Audio and video transcoding decisions are independent:
//!
//! | Scenario | Audio Strategy | Video Strategy |
//! |----------|---------------|----------------|
//! | Client supports all codecs | **Direct copy** (no CPU cost) | Direct copy or transcode for resolution |
//! | Audio codec unsupported | **Transcode** (e.g. TrueHD -> AAC) | Direct copy if video OK |
//! | Client is audio-only | Transcode audio only | Drop video stream |
//! | AC3/EAC3 passthrough to soundbar | **Direct copy** (keep bitstream) | Transcode video if needed |
//!
//! The transcoder must support separate audio/video decisions, direct stream copy
//! (`-c:a copy`), and multi-track selection.
//!
//! ### 3. Adaptive Bitrate (ABR) and Multi-Resolution
//!
//! For heterogeneous clients (phone on 4G vs. TV on gigabit Ethernet), generating
//! a single HLS variant is insufficient. The server should support:
//!
//! - **Multiple renditions**: e.g. 1080p@10Mbps, 720p@5Mbps, 480p@2Mbps, 360p@1Mbps
//! - **Variant playlist** (`master.m3u8`): lists all renditions; the client selects
//!   the best one based on bandwidth and resolution
//! - **Pre-warming**: when a new session starts, begin encoding the highest-demand
//!   rendition(s) immediately before the client has finished negotiating
//!
//! Each rendition is an independent transcode pipeline, which can be parallelised
//! across CPU cores or separate hardware encoder sessions.
//!
//! ### 4. Two-Stage Pipeline: Transcode then Slice
//!
//! The pipeline consists of **two mandatory and configurable stages**:
//!
//! ```plaintext
//!  Source File ──► [Stage 1: Transcode] ──► Container ──► [Stage 2: Slice] ──► Stream Segments
//!                        │                                   │
//!                   configurable                          configurable
//!                   (mp4/mkv/ts/...)                  (HLS fMP4 / HLS TS / DASH)
//! ```
//!
//! **Stage 1 — Transcode**: Converts the source into a complete container file.
//! Codec, bitrate, resolution, and container format are all configurable.
//! This uses the `ffmpeg-next` Rust bindings (native API), NOT command-line `ffmpeg`.
//!
//! **Stage 2 — Slice**: Takes the container and splits it into streamable segments
//! (fMP4 fragments for HLS/DASH, or MPEG-TS segments for legacy HLS). The slicing
//! strategy (segment duration, playlist format) is also configurable.
//!
//! Both stages must exist — the first produces the render target, the second
//! makes it streamable. They are independent: you can swap either stage's strategy
//! without affecting the other.
//!
//! | Container | Stage 1 Transcode | Stage 2 Slice | Notes |
//! |-----------|-------------------|---------------|-------|
//! | fMP4 (fragmented MP4) | Encode to ISO-BMFF with moof/mdat | Split moof/mdat into fragments | Default, works with HLS + DASH |
//! | MPEG-TS | Encode to MPEG-TS stream | Split on packet boundaries | Legacy HLS, wider device support |
//! | Regular MP4 | Encode to standard MP4 | **Not sliceable** — unsuitable | Must complete before playback |
//! | MKV | Encode to Matroska | **Not sliceable** — unsuitable | No native segment framing |
//!
//! The current `transcode.rs` does both stages in one pass (writes HLS directly).
//! This should be split into two independent stages for full flexibility.
//!
//! ### 5. Transcode Progress Monitoring
//!
//! Progress is tracked through the `ffmpeg-next` Rust API's internal state,
//! NOT by parsing command-line stderr output.
//!
//! Each frame passing through the `receive_frame` / `send_frame` loop updates
//! a frame counter and timestamp. The `Transcoder::log_progress` helper logs
//! the elapsed time, frame count, and current timestamp every 100 frames or
//! every 1 second, whichever comes first.
//!
//! ### 6. Transcode Output Directory (VFS)
//!
//! The so-called "VFS" is not a virtual filesystem — it is simply a temporary
//! directory on a local or tmpfs mount:
//!
//! ```plaintext
//! /tmp/cosmox/transcode/<session_id>/
//!   ├── master.m3u8
//!   ├── init.mp4
//!   └── seg-001.m4s
//! ```
//!
//! Key considerations:
//! - **tmpfs** is recommended (`mount -t tmpfs -o size=4G tmpfs /tmp/cosmox`)
//!   to avoid physical disk I/O for segment writes
//! - **Session lifetime**: segments are generated on the fly and served once;
//!   there is no persistent cache beyond the session's duration
//! - **Cleanup**: the `session_registry::start_reaper_loop` handles stale session
//!   directory removal after heartbeat timeout
//!
//! ### 7. Segment Cache for Hot Content
//!
//! When multiple users watch the same video at the same resolution, the server
//! currently spawns a separate transcode pipeline for each session. An optional
//! optimisation is a **segment-level cache**:
//!
//! - Keyed by `(file inode, timestamp range, encoding parameters)`
//! - Shared across sessions via a read-through cache
//! - Evicted when the source file changes or when the transcode parameters differ
//!
//! This is not yet implemented and is a future optimisation opportunity.

use ffmpeg_next as ffmpeg;

pub mod hw_accel;
pub mod probe;
pub mod streamer;
pub mod subtitle;
pub mod tone_mapping;
pub mod trickplay;

pub struct FfmpegTask;

pub fn init() {
    ffmpeg::init().expect("FFmpeg initialization failed");
    ffmpeg::log::set_level(ffmpeg::log::Level::Info);
}

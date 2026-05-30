//! # Subtitle Processing Design
//!
//! In modern streaming architectures it is recommended to support both
//! server-side and client-side translation pipelines. They complement each other
//! rather than being redundant.
//!
//! ## 1. Server-Side (Generate / Translate Subtitles or Audio)
//!
//! The server (Rust backend + FFmpeg) extracts audio/video streams, calls
//! LLM/Whisper/translation APIs, and produces static subtitle files (`.vtt`/`.ass`)
//! or standalone audio tracks.
//!
//! ### Suitable Scenarios
//!
//! - **Advanced ASS subtitles** with complex styling, positioning, and animation
//!   require server-side pre-rendering into standard subtitle files.
//! - **Automated library ingestion** – newly downloaded media triggers automatic
//!   speech recognition and translation, producing bilingual subtitles ahead of time.
//! - **Multi-language dubbing** – generating and mixing new audio tracks
//!   is computationally heavy and must be done server-side.
//! - **Lightweight clients** (smart TVs, old phones) receive pre-rendered subtitles
//!   or burned-in captions directly in the video stream.
//!
//! ## 2. Client-Side (Dynamic Translation)
//!
//! The backend sends the raw video/subtitle data; the client player
//! (`mpv` plugin, browser JavaScript) intercepts subtitle text at runtime
//! and calls translation APIs on demand.
//!
//! ### Suitable Scenarios
//!
//! - **On-the-fly language switching** – users can switch between Baidu,
//!   DeepL, or local Ollama LLM translations without server involvement.
//! - **Interactive word lookup** – useful for language learning: hover over
//!   a word to get an instant definition or grammar explanation.
//! - **AI live captioning** – for live streams or unscheduled content,
//!   the client can capture audio via Web Audio API and run local
//!   speech-to-text for real-time translation.
//!
//! ## Summary
//!
//! | Aspect | Server-Side | Client-Side |
//! |--------|-------------|-------------|
//! | Nature | Static persistence, batch processing | Dynamic interception, real-time requests |
//! | Role | Generates subtitles when none exist | Refines existing subtitles, interactive features |
//! | Server load | High (runs during ingestion) | Zero |
//!
//! The recommended design is: **server as foundation** – ensure every ingested
//! video has high-quality offline bilingual subtitles; **client for interaction**
//! – expose translation plugin interfaces in the player for real-time word lookup,
//! re-translation, and language learning features.

//! # Tone Mapping Module (HDR to SDR)
//!
//! When a user plays 4K HDR (10-bit BT.2020) content on an SDR screen (phone,
//! browser), naive transcoding produces washed-out, dim, colourless output.
//! FFmpeg's tonemap filter must be inserted into the filter graph to correctly
//! map HDR luminance values to the SDR range.

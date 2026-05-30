//! # Trickplay Module (Scrubbing Previews & Playback Speed)
//!
//! When users scrub through the timeline, they expect thumbnail previews
//! (BIF index) similar to YouTube or Netflix. When playing at 2x speed,
//! audio pitch should remain unchanged. This module will handle:
//!
//! - Generating BIF (Binary Image Format) thumbnail sprites for seek previews.
//! - Implementing tempo-preserving audio scaling for variable playback speed.

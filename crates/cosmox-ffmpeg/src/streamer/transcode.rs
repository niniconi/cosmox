use std::{collections::HashMap, ffi::CString, ptr, time::Instant};

use ffmpeg::{
    Dictionary, Packet, Rational, codec, decoder, encoder, format, frame, media, picture,
};
use ffmpeg_next as ffmpeg;

pub enum QualityPreset {
    VeryFast,
    Medium,
    Slower,
}

/// Hardware acceleration mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HardwareMode {
    /// Use software encoding/decoding only.
    Disabled,
    /// Try hardware acceleration; fall back to software if unavailable.
    Auto,
    /// Require hardware acceleration; fail if unavailable.
    Force,
}

impl From<bool> for HardwareMode {
    fn from(v: bool) -> Self {
        if v { Self::Auto } else { Self::Disabled }
    }
}

const DEFAULT_OPTS: &str =
    "preset=medium,hls_time=6,hls_list_size=0,hls_segment_filename=tmp/seg_%d.ts";

/// Video encoder configuration parameters.
pub struct VideoEncoderConfig {
    /// Codec name, e.g. "libx264", "h264_nvenc".
    pub codec: String,
    /// Target bitrate in bits per second.
    pub bitrate: Option<usize>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub preset: QualityPreset,
}

/// General transcoding task configuration.
pub struct TranscodeTask<'a> {
    pub input_path: &'a str,
    pub output_path: &'a str,
    pub video: Option<VideoEncoderConfig>,
    /// Whether to copy audio stream directly without re-encoding.
    pub audio_copy: bool,
    /// Whether to enable hardware acceleration.
    /// When `true`, uses `HardwareMode::Auto` (try GPU, fall back to software).
    pub use_hardware: bool,
}

pub struct Transcoder {
    ost_index: usize,
    decoder: decoder::Video,
    input_time_base: Rational,
    encoder: encoder::Video,
    logging_enabled: bool,
    frame_count: usize,
    last_log_frame_count: usize,
    starting_time: Instant,
    last_log_time: Instant,
}

fn parse_opts<'a>(s: String) -> Option<Dictionary<'a>> {
    let mut dict = Dictionary::new();
    for keyval in s.split_terminator(',') {
        let tokens: Vec<&str> = keyval.split('=').collect();
        match tokens[..] {
            [key, val] => dict.set(key, val),
            _ => return None,
        }
    }
    dict.set(
        "hls_flags",
        "independent_segments+split_by_time+program_date_time",
    );
    dict.set("hls_playlist_type", "vod");
    dict.set("x264opts", "no-scenecut=1");
    dict.set("force_key_frames", "expr:gte(t,n_forced*6)");
    dict.set("g", "60");
    Some(dict)
}

extern "C" fn get_hw_format(
    _ctx: *mut ffmpeg::sys::AVCodecContext,
    pix_fmts: *const ffmpeg::sys::AVPixelFormat,
) -> ffmpeg::sys::AVPixelFormat {
    let mut p = pix_fmts;
    // Loop through formats supported by the decoder until AV_PIX_FMT_NONE.
    unsafe {
        while *p != ffmpeg::sys::AVPixelFormat::AV_PIX_FMT_NONE {
            if *p == ffmpeg::sys::AVPixelFormat::AV_PIX_FMT_VAAPI {
                return *p;
            }
            p = p.offset(1);
        }
    }
    ffmpeg::sys::AVPixelFormat::AV_PIX_FMT_YUV420P
}

impl Transcoder {
    fn new(
        ist: &format::stream::Stream,
        octx: &mut format::context::Output,
        ost_index: usize,
        opts: Dictionary,
        enable_logging: bool,
        hw_mode: HardwareMode,
    ) -> Result<Self, ffmpeg::Error> {
        // let global_header = octx.format().flags().contains(format::Flags::GLOBAL_HEADER);
        let decoder = ffmpeg::codec::context::Context::from_parameters(ist.parameters())?
            .decoder()
            .video()?;
        let mut decoder_ctx = codec::context::Context::from_parameters(&decoder)?;

        let codec = encoder::find(codec::Id::H264);
        let mut ost = octx.add_stream(codec)?;

        let mut encoder =
            codec::context::Context::new_with_codec(codec.ok_or(ffmpeg::Error::InvalidData)?)
                .encoder()
                .video()?;
        ost.set_parameters(&encoder);
        let mut encoder_ctx = codec::context::Context::from_parameters(&encoder)?;
        encoder_ctx.set_flags(
            codec::flag::Flags::from_bits(unsafe {
                (*encoder_ctx.as_ptr()).flags as u32 & !(codec::flag::Flags::GLOBAL_HEADER.bits())
            })
            .unwrap(),
        );

        encoder.set_height(decoder.height());
        encoder.set_width(decoder.width());
        encoder.set_aspect_ratio(decoder.aspect_ratio());
        encoder.set_format(decoder.format());
        encoder.set_frame_rate(decoder.frame_rate());
        encoder.set_time_base(ist.time_base());

        match hw_mode {
            HardwareMode::Disabled => {}
            HardwareMode::Auto => {
                let _ = Self::init_gpu_decoder(&mut decoder_ctx, &mut encoder_ctx);
            }
            HardwareMode::Force => {
                Self::init_gpu_decoder(&mut decoder_ctx, &mut encoder_ctx)?;
            }
        }

        // if global_header {
        //   encoder.set_flags(codec::Flags::GLOBAL_HEADER);
        // }

        let opened_encoder = encoder
            .open_with(opts)
            .expect("error opening x264 with supplied settings");
        ost.set_parameters(&opened_encoder);
        Ok(Self {
            ost_index,
            decoder,
            input_time_base: ist.time_base(),
            encoder: opened_encoder,
            logging_enabled: enable_logging,
            frame_count: 0,
            last_log_frame_count: 0,
            starting_time: Instant::now(),
            last_log_time: Instant::now(),
        })
    }

    fn init_gpu_decoder(
        decoder_ctx: &mut ffmpeg::codec::context::Context,
        encoder_ctx: &mut ffmpeg::codec::context::Context,
    ) -> Result<(), ffmpeg::Error> {
        unsafe {
            let device_path = CString::new("/dev/dri/renderD128").unwrap();
            let mut hw_device_ctx: *mut ffmpeg::sys::AVBufferRef = ptr::null_mut();

            let err = ffmpeg::sys::av_hwdevice_ctx_create(
                &mut hw_device_ctx,
                ffmpeg::sys::AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI,
                device_path.as_ptr(),
                ptr::null_mut(),
                0,
            );

            if err < 0 {
                return Err(ffmpeg::Error::from(err));
            }

            // Only modify decoder/encoder contexts after device creation succeeds,
            // so Auto mode can cleanly fall back to software.

            let raw_dec_ctx = decoder_ctx.as_mut_ptr();
            let raw_enc_ctx = encoder_ctx.as_mut_ptr();

            (*raw_dec_ctx).get_format = Some(get_hw_format);

            (*raw_enc_ctx).pix_fmt = ffmpeg_sys_next::AVPixelFormat::AV_PIX_FMT_VAAPI;

            (*raw_dec_ctx).hw_device_ctx = ffmpeg::sys::av_buffer_ref(hw_device_ctx);

            ffmpeg::sys::av_buffer_unref(&mut hw_device_ctx);

            if !(*raw_dec_ctx).hw_frames_ctx.is_null() {
                (*raw_enc_ctx).hw_frames_ctx =
                    ffmpeg::sys::av_buffer_ref((*raw_dec_ctx).hw_frames_ctx);
            } else if !(*raw_dec_ctx).hw_device_ctx.is_null() {
                (*raw_enc_ctx).hw_device_ctx =
                    ffmpeg::sys::av_buffer_ref((*raw_dec_ctx).hw_device_ctx);
            }
        }
        Ok(())
    }

    fn send_packet_to_decoder(&mut self, packet: &Packet) {
        self.decoder.send_packet(packet).unwrap();
    }

    fn send_eof_to_decoder(&mut self) {
        self.decoder.send_eof().unwrap();
    }

    fn receive_and_process_decoded_frames(
        &mut self,
        octx: &mut format::context::Output,
        ost_time_base: Rational,
    ) {
        let mut frame = frame::Video::empty();
        while self.decoder.receive_frame(&mut frame).is_ok() {
            self.frame_count += 1;
            let timestamp = frame.timestamp();
            self.log_progress(f64::from(
                Rational(timestamp.unwrap_or(0) as i32, 1) * self.input_time_base,
            ));
            frame.set_pts(timestamp);
            frame.set_kind(picture::Type::None);
            self.send_frame_to_encoder(&frame);
            self.receive_and_process_encoded_packets(octx, ost_time_base);
        }
    }

    fn send_frame_to_encoder(&mut self, frame: &frame::Video) {
        self.encoder.send_frame(frame).unwrap();
    }

    fn send_eof_to_encoder(&mut self) {
        self.encoder.send_eof().unwrap();
    }

    fn receive_and_process_encoded_packets(
        &mut self,
        octx: &mut format::context::Output,
        ost_time_base: Rational,
    ) {
        let mut encoded = Packet::empty();
        while self.encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(self.ost_index);
            // If duration is 0, derive it from the frame rate.
            if encoded.duration() == 0 {
                // Get encoder's time_base
                let enc_time_base = self.encoder.time_base();

                // Frame duration = 1 / fps, expressed in encoder time_base units:
                // duration = time_base.den / (time_base.num * fps)
                // let fps = self.encoder.frame_rate();
                let duration = (enc_time_base.denominator() as f64
                    / (enc_time_base.numerator() as f64 * 30.0))
                    as i64;

                encoded.set_duration(duration);
            }
            encoded.rescale_ts(self.input_time_base, ost_time_base);
            encoded.write_interleaved(octx).unwrap();
        }
    }

    fn log_progress(&mut self, timestamp: f64) {
        if !self.logging_enabled
            || (self.frame_count - self.last_log_frame_count < 100
                && self.last_log_time.elapsed().as_secs_f64() < 1.0)
        {
            return;
        }
        eprintln!(
            "time elpased: \t{:8.2}\tframe count: {:8}\ttimestamp: {:8.2}",
            self.starting_time.elapsed().as_secs_f64(),
            self.frame_count,
            timestamp
        );
        self.last_log_frame_count = self.frame_count;
        self.last_log_time = Instant::now();
    }

    /// Transcodes a video file, changing only its resolution while attempting to
    /// maintain the original encoding format and bitrate.
    ///
    /// # Arguments
    /// - `input_path`: Path to the input video file.
    /// - `output_path`: Path to the output video file.
    /// - `target_width`: Target video width.
    /// - `target_height`: Target video height.
    ///
    /// # Returns
    /// - `Ok(())` if transcoding is successful, or `Err(ffmpeg::Error)` otherwise.
    pub fn transcode_video_with_resolution_change(
        task: TranscodeTask,
    ) -> Result<(), ffmpeg::Error> {
        let input_file = task.input_path;
        let output_file = task.output_path;
        let opts = parse_opts(DEFAULT_OPTS.to_string()).expect("invalid options string");

        eprintln!("options: {:?}", opts);

        // ffmpeg::init().unwrap();
        // log::set_level(log::Level::Info);

        let mut ictx = format::input(&input_file)?;
        let mut octx = format::output_as(&output_file, "hls")?;

        format::context::input::dump(&ictx, 0, Some(input_file));

        let best_video_stream_index = ictx
            .streams()
            .best(media::Type::Video)
            .map(|stream| stream.index());
        let mut stream_mapping: Vec<isize> = vec![0; ictx.nb_streams() as _];
        let mut ist_time_bases = vec![Rational(0, 0); ictx.nb_streams() as _];
        let mut ost_time_bases = vec![Rational(0, 0); ictx.nb_streams() as _];
        let mut transcoders = HashMap::new();
        let mut ost_index = 0;
        for (ist_index, ist) in ictx.streams().enumerate() {
            let ist_medium = ist.parameters().medium();
            if ist_medium != media::Type::Audio
                && ist_medium != media::Type::Video
                && ist_medium != media::Type::Subtitle
            {
                stream_mapping[ist_index] = -1;
                continue;
            }
            stream_mapping[ist_index] = ost_index;
            ist_time_bases[ist_index] = ist.time_base();
            if ist_medium == media::Type::Video {
                // Initialize transcoder for video stream.
                transcoders.insert(
                    ist_index,
                    Transcoder::new(
                        &ist,
                        &mut octx,
                        ost_index as _,
                        opts.to_owned(),
                        Some(ist_index) == best_video_stream_index,
                        HardwareMode::from(task.use_hardware),
                    )
                    .unwrap(),
                );
            } else {
                // Set up for stream copy for non-video stream.
                let mut ost = octx.add_stream(encoder::find(codec::Id::None)).unwrap();
                ost.set_parameters(ist.parameters());
                // We need to set codec_tag to 0 lest we run into incompatible codec tag
                // issues when muxing into a different container format. Unfortunately
                // there's no high level API to do this (yet).
                unsafe {
                    (*ost.parameters().as_mut_ptr()).codec_tag = 0;
                }
            }
            ost_index += 1;
        }

        octx.set_metadata(ictx.metadata().to_owned());
        format::context::output::dump(&octx, 0, Some(output_file));
        octx.write_header().unwrap();

        for (ost_index, _) in octx.streams().enumerate() {
            ost_time_bases[ost_index] = octx.stream(ost_index as _).unwrap().time_base();
        }

        for (stream, mut packet) in ictx.packets() {
            let ist_index = stream.index();
            let ost_index = stream_mapping[ist_index];
            if ost_index < 0 {
                continue;
            }
            let ost_time_base = ost_time_bases[ost_index as usize];

            // if packet.duration() == 0 {
            //   let fps = 30;
            //   let frame_duration =
            //     (ost_time_base.denominator() as f64 / (ost_time_base.numerator() as f64 * fps as f64)) as i64;
            //   packet.set_duration(frame_duration);
            //   eprintln!("set => {}", packet.duration());
            // }
            // else {
            //     eprintln!("=> {}", packet.duration());
            // }
            match transcoders.get_mut(&ist_index) {
                Some(transcoder) => {
                    transcoder.send_packet_to_decoder(&packet);
                    transcoder.receive_and_process_decoded_frames(&mut octx, ost_time_base);
                }
                None => {
                    // Do stream copy on non-video streams.
                    packet.rescale_ts(ist_time_bases[ist_index], ost_time_base);
                    packet.set_position(-1);
                    packet.set_stream(ost_index as _);
                    packet.write_interleaved(&mut octx).unwrap();
                }
            }
        }

        // Flush encoders and decoders.
        for (ost_index, transcoder) in transcoders.iter_mut() {
            let ost_time_base = ost_time_bases[*ost_index];
            transcoder.send_eof_to_decoder();
            transcoder.receive_and_process_decoded_frames(&mut octx, ost_time_base);
            transcoder.send_eof_to_encoder();
            transcoder.receive_and_process_encoded_packets(&mut octx, ost_time_base);
        }

        octx.write_trailer().unwrap();
        Ok(())
    }
}

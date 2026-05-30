use std::path::Path;

use ffmpeg_next as ffmpeg;

#[derive(Default, Debug)]
pub struct Stream {
    pub id: i32,
    pub index: usize,
    pub rate: f64,
    pub duration: i64,
}

#[derive(Debug)]
pub struct Container<'a> {
    pub filename: &'a Path,
    pub metadata: Vec<(String, String)>,
    pub streams: Vec<Stream>,
}

pub fn get_metadata<'a>(path: &'a Path) -> Result<Container<'a>, ffmpeg::Error> {
    let ictx = ffmpeg::format::input(&path)?;

    let streams = ictx
        .streams()
        .map(|s| Stream {
            id: s.id(),
            index: s.index(),
            rate: f64::from(s.rate()),
            duration: s.duration(),
        })
        .collect();

    let metadata = ictx
        .metadata()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    Ok(Container {
        filename: path,
        metadata,
        streams,
    })
}

pub struct ThumbnailConfig {
    /// Timestamp in milliseconds from which to extract the frame.
    pub timestamp_ms: i64,
    /// Output width (aspect ratio preserved if only width is set).
    pub width: Option<u32>,
    /// Output height.
    pub height: Option<u32>,
    /// JPEG quality (1-100).
    pub quality: u8,
}

fn save_rgb_to_file(path: &str, frame: &ffmpeg::frame::Video, _quality: u8) {
    let width = frame.width();
    let height = frame.height();
    let data = frame.data(0);
    let stride = frame.stride(0);

    let mut img = image::RgbImage::new(width, height);
    for y in 0..height {
        let start = (y as usize) * stride;
        let end = start + (width as usize) * 3;
        let row = &data[start..end];
        for (x, chunk) in row.chunks_exact(3).enumerate() {
            let x = x as u32;
            img.put_pixel(x, y, image::Rgb([chunk[0], chunk[1], chunk[2]]));
        }
    }
    img.save(path).expect("Failed to save image");
}

pub fn extract_frame(
    input_path: &str,
    output_path: &str,
    config: ThumbnailConfig,
) -> Result<(), ffmpeg::Error> {
    let mut ictx = ffmpeg::format::input(&input_path)?;
    let input = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or(ffmpeg::Error::StreamNotFound)?;
    let video_stream_index = input.index();

    let mut decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?
        .decoder()
        .video()?;
    let width = decoder.width();
    let height = decoder.height();
    let format = decoder.format();

    // Seek to the target timestamp.
    let timestamp = (config.timestamp_ms as f64 / 1000.0 / f64::from(input.time_base())) as i64;
    ictx.seek(timestamp, timestamp..)?;

    let mut frame = ffmpeg::frame::Video::empty();
    let mut got_frame = false;

    // Read and decode one frame at the target position.
    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet)?;
            if decoder.receive_frame(&mut frame).is_ok() {
                got_frame = true;
                break;
            }
        }
    }

    if !got_frame {
        return Err(ffmpeg::Error::InvalidData);
    }

    // Scale the frame if requested.
    let w = config.width.unwrap_or(width);
    let h = config.height.unwrap_or(height);

    let mut scaler = ffmpeg::software::scaling::Context::get(
        format,
        width,
        height,
        ffmpeg::format::Pixel::RGB24,
        w,
        h,
        ffmpeg::software::scaling::Flags::BILINEAR,
    )?;

    let mut rgb_frame = ffmpeg::frame::Video::new(ffmpeg::format::Pixel::RGB24, w, h);
    scaler.run(&frame, &mut rgb_frame)?;

    save_rgb_to_file(output_path, &rgb_frame, config.quality);
    Ok(())
}

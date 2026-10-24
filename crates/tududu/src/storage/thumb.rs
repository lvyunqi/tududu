//! 生成缩略图。列表页用它，避免一张两兆的原图直接铺满屏幕。

use std::io::Cursor;

use image::imageops::FilterType;
use image::{ExtendedColorType, ImageEncoder, ImageFormat, ImageReader};

/// 缩略图的最长边。
pub const THUMB_EDGE: u32 = 512;

/// 解码前先看尺寸，挡住夸张的大图，防止一张图把内存吃光。
const MAX_PIXELS: u64 = 40_000_000;

/// 生成 JPEG 缩略图。解码失败或者图太大就返回 None，不影响原图入库。
pub fn make(bytes: &[u8]) -> Option<Vec<u8>> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;

    let (width, height) = reader.into_dimensions().ok()?;
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return None;
    }

    let decoded = image::load_from_memory_with_format(bytes, format_of(bytes)?).ok()?;
    let scaled = decoded.resize(THUMB_EDGE, THUMB_EDGE, FilterType::Triangle);
    let rgb = scaled.to_rgb8();
    let (out_width, out_height) = (rgb.width(), rgb.height());

    let mut out: Vec<u8> = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 78);
    encoder
        .write_image(rgb.as_raw(), out_width, out_height, ExtendedColorType::Rgb8)
        .ok()?;

    Some(out)
}

/// 按文件头选解码格式，和上传时的白名单保持一致。
fn format_of(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Some(ImageFormat::Png);
    }

    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(ImageFormat::Jpeg);
    }

    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(ImageFormat::WebP);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_image_payload() {
        assert!(make(b"not an image").is_none());
        assert!(format_of(b"GIF89a").is_none());
    }

    #[test]
    fn thumbnail_is_smaller_than_source() {
        let mut source = image::RgbImage::new(1200, 800);
        for (x, y, pixel) in source.enumerate_pixels_mut() {
            *pixel = image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]);
        }

        let mut encoded: Vec<u8> = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 90)
            .write_image(source.as_raw(), 1200, 800, ExtendedColorType::Rgb8)
            .expect("编码测试图失败");

        let thumb = make(&encoded).expect("生成缩略图失败");
        assert!(!thumb.is_empty());
        assert!(thumb.len() < encoded.len());
    }
}

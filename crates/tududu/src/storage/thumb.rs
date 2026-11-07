//! 生成缩略图与预览图。
//!
//! 列表用 512 的缩略图，点开看 1280 的预览图，只有下载才给原图。
//! 这样页面加载快，原图也不至于被随便拖走。

use std::io::Cursor;

use image::imageops::FilterType;
use image::{ExtendedColorType, ImageEncoder, ImageFormat, ImageReader};

/// 列表用的最长边。
pub const THUMB_EDGE: u32 = 512;

/// 点开查看用的最长边。
pub const PREVIEW_EDGE: u32 = 1280;

/// 解码前先看尺寸，挡住夸张的大图，防止一张图把内存吃光。
const MAX_PIXELS: u64 = 40_000_000;

/// 生成 JPEG 缩略图。解码失败或者图太大就返回 None，不影响原图入库。
pub fn make(bytes: &[u8]) -> Option<Vec<u8>> {
    render(bytes, THUMB_EDGE, 78)
}

/// 生成预览图，尺寸大一些，但也比原图小很多。
pub fn make_preview(bytes: &[u8]) -> Option<Vec<u8>> {
    render(bytes, PREVIEW_EDGE, 82)
}

fn render(bytes: &[u8], edge: u32, quality: u8) -> Option<Vec<u8>> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;

    let (width, height) = reader.into_dimensions().ok()?;
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return None;
    }

    // 本来就没超过目标尺寸的话，不放大
    if width <= edge && height <= edge {
        return Some(bytes.to_vec());
    }

    let decoded = image::load_from_memory_with_format(bytes, format_of(bytes)?).ok()?;
    let scaled = decoded.resize(edge, edge, FilterType::Lanczos3);
    let rgb = scaled.to_rgb8();
    let (out_width, out_height) = (rgb.width(), rgb.height());

    let mut out: Vec<u8> = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
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

    fn sample(width: u32, height: u32) -> Vec<u8> {
        let mut source = image::RgbImage::new(width, height);
        for (x, y, pixel) in source.enumerate_pixels_mut() {
            *pixel = image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]);
        }

        let mut encoded: Vec<u8> = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 92)
            .write_image(source.as_raw(), width, height, ExtendedColorType::Rgb8)
            .expect("编码测试图失败");

        encoded
    }

    #[test]
    fn rejects_non_image_payload() {
        assert!(make(b"not an image").is_none());
        assert!(make_preview(b"not an image").is_none());
        assert!(format_of(b"GIF89a").is_none());
    }

    #[test]
    fn thumbnail_is_smaller_than_source() {
        let source = sample(1600, 1200);
        let thumb = make(&source).expect("生成缩略图失败");

        assert!(!thumb.is_empty());
        assert!(thumb.len() < source.len());
    }

    #[test]
    fn preview_is_between_thumb_and_source() {
        let source = sample(2400, 1600);
        let thumb = make(&source).expect("生成缩略图失败");
        let preview = make_preview(&source).expect("生成预览失败");

        assert!(thumb.len() < preview.len());
        assert!(preview.len() < source.len());
    }

    #[test]
    fn small_image_is_not_upscaled() {
        let source = sample(320, 240);
        let preview = make_preview(&source).expect("生成预览失败");

        assert_eq!(preview.len(), source.len());
    }
}

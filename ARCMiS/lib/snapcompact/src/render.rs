//! Render one text transcript into dense PNG frames with the 8x13 BDF font.
//! Mirrors oh-my-pi's `render.ts`: fixed glyph pitch, black ink on white,
//! frame width fixed (1568px default), height hugs the rows actually drawn.

use bdf_parser::BdfFont;
use image::ImageBuffer;
use image::Rgba;

/// Pixels per glyph column (8px wide font on an 8px advance, plus 2px gutter).
pub const GLYPH_PITCH_X: u32 = 10;
/// Pixels per glyph row (13px tall font plus 3px leading).
pub const GLYPH_PITCH_Y: u32 = 16;
/// Default frame width in pixels (oh-my-pi `8on22-bw` for ollama).
pub const DEFAULT_FRAME_WIDTH: u32 = 1568;
/// Default frame height in pixels.
pub const DEFAULT_FRAME_HEIGHT: u32 = 1568;
/// Left margin in pixels.
pub const MARGIN_X: u32 = 8;
/// Top margin in pixels.
pub const MARGIN_Y: u32 = 4;

/// Render `text` into one or more frames of the given size.
///
/// Lines wrap at the frame width; frames fill top-to-bottom; a new frame
/// starts when the current one is full. Every frame is black ink on white.
#[must_use]
pub fn render_frames(
    text: &str,
    font: &BdfFont,
    frame_width: u32,
    frame_height: u32,
) -> Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let rows_per_frame = ((frame_height - 2 * MARGIN_Y) / GLYPH_PITCH_Y).max(1) as usize;
    let lines: Vec<&str> = text.lines().collect();
    let lines = if lines.is_empty() {
        vec![""]
    } else {
        lines
    };

    let mut frames = Vec::new();
    for chunk in lines.chunks(rows_per_frame) {
        let frame = render_one(chunk, font, frame_width, frame_height);
        frames.push(frame);
    }
    if frames.is_empty() {
        frames.push(render_one(&[""], font, frame_width, frame_height));
    }
    frames
}

/// Render one frame from an exact list of lines.
fn render_one(lines: &[&str], font: &BdfFont, frame_width: u32, frame_height: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut frame = ImageBuffer::from_pixel(frame_width, frame_height, Rgba([255, 255, 255, 255]));
    for (row, line) in lines.iter().enumerate() {
        let y = MARGIN_Y + (row as u32) * GLYPH_PITCH_Y;
        for (column, ch) in line.chars().enumerate() {
            let x = MARGIN_X + (column as u32) * GLYPH_PITCH_X;
            if x + GLYPH_PITCH_X > frame_width {
                break;
            }
            draw_glyph(&mut frame, font, ch, x, y);
        }
    }
    frame
}

/// Draw one glyph's bitmap at (x, y) as the top-left of its cell.
fn draw_glyph(frame: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, font: &BdfFont, ch: char, x: u32, y: u32) {
    // The 8x13 font covers ASCII; unknown glyphs render as space (blank).
    let Some(glyph) = font.glyphs.get(ch) else {
        return;
    };
    let bbx = glyph.bounding_box;
    let width = bbx.size.x.max(0) as usize;
    let height = bbx.size.y.max(0) as usize;
    // BDF y offset: distance from the baseline up to the bitmap bottom.
    // Screen y of the bitmap top = baseline_y - offset.y - height + 1.
    let baseline_y = y as i64 + 12; // 13px font: baseline sits 12px below cell
                                    // top.
    let top = baseline_y - i64::from(bbx.offset.y) - height as i64 + 1;
    for gy in 0..height {
        for gx in 0..width {
            if glyph.pixel(gx, gy) {
                let px = x as i64 + i64::from(bbx.offset.x) + gx as i64;
                let py = top + gy as i64;
                if px >= 0 && py >= 0 {
                    let (pu, pv) = (px as u32, py as u32);
                    if pu < frame.width() && pv < frame.height() {
                        frame.put_pixel(pu, pv, Rgba([0, 0, 0, 255]));
                    }
                }
            }
        }
    }
}

/// Encode frames as PNG bytes.
#[must_use]
pub fn encode_png(frames: &[ImageBuffer<Rgba<u8>, Vec<u8>>]) -> Vec<Vec<u8>> {
    frames
        .iter()
        .map(|frame| {
            let mut bytes = Vec::new();
            let cursor = std::io::Cursor::new(&mut bytes);
            image::DynamicImage::ImageRgba8(frame.clone())
                .write_to(cursor, image::ImageFormat::Png)
                .expect("PNG encode of an in-memory frame cannot fail");
            bytes
        })
        .collect()
}

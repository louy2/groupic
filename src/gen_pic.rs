#![allow(dead_code)]

use std::{fs, path::Path};

use glyph_brush_layout::{
    ab_glyph::{Font, FontRef, PxScale, ScaleFont},
    FontId, GlyphPositioner, Layout, SectionGeometry, SectionGlyph, SectionText,
};
use image::{imageops::resize, GenericImage, ImageBuffer, Pixel, Rgba, RgbaImage};
use num::{integer::Roots, Integer};

const FONT_DATA: &[u8] = include_bytes!("../NotoSansJP-Medium.otf");
const EMOJI_FONT_DATA: &[u8] = include_bytes!("../NotoColorEmoji.ttf");
const DISCORD_COLOR: Rgba<u8> = Rgba([48, 48, 54, 255]);

pub fn generate_group_pic<I, O, S>(
    avatars_dir: I,
    out_group_pic_path: O,
    num_of_avatars_in_a_row: Option<u32>,
    header_text: S,
) where
    I: AsRef<Path>,
    O: AsRef<Path>,
    S: AsRef<str>,
{
    // configure the group pic
    let header_h = 64;
    let header_text = header_text.as_ref();
    let header_font_size = 54.;
    let mask_radius = 64;
    let avatars_dir = avatars_dir.as_ref();

    // calculate the rest of the configuration
    let num_of_avatars = fs::read_dir(avatars_dir).unwrap().count() as u32;
    let num_of_avatars_in_a_row =
        num_of_avatars_in_a_row.unwrap_or_else(|| core::cmp::max(num_of_avatars.sqrt(), 5));
    let num_of_rows = num_of_avatars.div_ceil(&num_of_avatars_in_a_row);
    let group_pic_w = 128 * num_of_avatars_in_a_row;
    let group_pic_h = header_h + 128 * num_of_rows;

    // prepare the image buffer
    let mut group_pic = ImageBuffer::from_pixel(group_pic_w, group_pic_h, DISCORD_COLOR);
    // #[cfg(debug_assertions)]
    // dbg!(group_pic.dimensions());

    // render the header
    render_header_glyph_brush(&mut group_pic, header_h, header_text, header_font_size);

    // mask and tile the avatars
    for (i, avatar_path) in fs::read_dir(avatars_dir).unwrap().enumerate() {
        let avatar_path = avatar_path.unwrap().path();
        let mut avatar_img = image::open(&avatar_path).unwrap().into_rgba8();
        if avatar_img.dimensions() != (128, 128) {
            avatar_img = resize(&avatar_img, 128, 128, image::imageops::FilterType::Lanczos3);
        }
        for (x, y, p) in avatar_img.enumerate_pixels_mut() {
            if (x as i64 - 64) * (x as i64 - 64) + (y as i64 - 64) * (y as i64 - 64)
                >= mask_radius * mask_radius
            {
                p.0.copy_from_slice(&DISCORD_COLOR.0);
            }
        }
        let x_offset = i as u32 % num_of_avatars_in_a_row * 128;
        let y_offset = i as u32 / num_of_avatars_in_a_row * 128 + header_h;
        // println!(
        //     "{:#?}: {:?} {:?}",
        //     avatar_path,
        //     avatar_img.dimensions(),
        //     (x_offset, y_offset)
        // );
        group_pic
            .copy_from(&avatar_img, x_offset, y_offset)
            .unwrap();
    }

    group_pic.save(out_group_pic_path.as_ref()).unwrap();
}

fn render_header_glyph_brush(
    group_pic: &mut RgbaImage,
    header_h: u32,
    header_text: &str,
    header_font_size: f32,
) {
    let group_pic_w = group_pic.width();
    let noto = FontRef::try_from_slice(FONT_DATA).expect("error loading font");
    // let noto_emoji = FontRef::try_from_slice(EMOJI_FONT_DATA).expect("error loading emoji font");
    let fonts = &[noto.clone()];
    // let sections: Vec<SectionText> = vec![];
    // for (i, grapheme) in unic::segment::GraphemeIndices::new(header_text) {

    // }
    let glyphs = Layout::default().calculate_glyphs(
        fonts,
        &SectionGeometry {
            screen_position: (0., 0.),
            bounds: (group_pic_w as f32, header_h as f32),
        },
        &[SectionText {
            text: header_text,
            scale: PxScale::from(header_font_size as f32),
            font_id: FontId(0),
        }],
    );
    let scaled = noto.as_scaled(header_font_size as f32);
    let layout_h = (scaled.ascent() - scaled.descent()).ceil() as u32;
    let layout_w = {
        let min_x = noto
            .outline_glyph(glyphs.first().unwrap().glyph.clone())
            .unwrap()
            .px_bounds()
            .min
            .x;
        let max_x = noto
            .outline_glyph(glyphs.last().unwrap().glyph.clone())
            .unwrap()
            .px_bounds()
            .min
            .x;
        (max_x - min_x) as u32
    };
    let x_offset = (group_pic_w - layout_w) / 2;
    let y_offset = (header_h - layout_h) / 2;
    for SectionGlyph { glyph, .. } in glyphs {
        let font = &noto;
        // if let Some(gi) = font.glyph_raster_image(glyph.id, header_font_size) {
        //     dbg!(gi.format);
        //     let d = image::io::Reader::new(Cursor::new(gi.data))
        //         .with_guessed_format()
        //         .unwrap()
        //         .decode()
        //         .unwrap();
        //     group_pic
        //         .copy_from(&d, glyph.position.x as u32, glyph.position.y as u32)
        //         .unwrap();
        //     continue;
        // }
        if let Some(q) = font.outline_glyph(glyph) {
            let b = q.px_bounds();
            q.draw(|x, y, c| {
                group_pic
                    .get_pixel_mut(x_offset + x + b.min.x as u32, y_offset + y + b.min.y as u32)
                    .blend(&image::Rgba([240, 240, 240, (c * 255.) as u8]))
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageBuffer;
    use rand::prelude::*;
    use std::fs;
    use std::path::Path;

    fn generate_random_test_avatars_in_dir_with_size<P>(dir: P, size: u32)
    where
        P: AsRef<Path>,
    {
        let mut rng = rand::thread_rng();
        for i in 0..99 {
            let gray: u8 = rng.gen();
            let img = ImageBuffer::from_pixel(size, size, image::Luma([gray]));
            let img_path = dir.as_ref().join(format!("{}.png", i));
            img.save(img_path).unwrap();
        }
    }

    #[test]
    fn generate_random_test_avatars() {
        let temp_dir = Path::new("tmp");
        if !temp_dir.exists() {
            fs::create_dir(temp_dir).unwrap();
        }
        let avatar_dir = Path::new("tmp/test_avatars");
        if !avatar_dir.exists() {
            fs::create_dir(avatar_dir).unwrap();
        }
        generate_random_test_avatars_in_dir_with_size(avatar_dir, 128);
    }

    #[test]
    fn generate_full_group_pic() {
        generate_group_pic(
            "tmp/test_avatars",
            "tmp/example_group_pic.png",
            Some(5),
            "niji3rd-live-day1",
        );
    }

    #[test]
    fn only_one_avatar() {
        generate_group_pic(
            "tmp/test_one_avatar",
            "tmp/one_avatar_group_pic.png",
            Some(5),
            "niji3rd-live-day1",
        );
    }

    #[test]
    fn only_one_avatar_with_kana_kanji() {
        generate_group_pic(
            "tmp/test_one_avatar",
            "tmp/one_avatar_with_kana_kanji.png",
            Some(5),
            "ラブライブ!虹ヶ咲3rdライブ1日目",
        );
    }

    #[test]
    fn square_group_pic() {
        generate_group_pic(
            "tmp/test_avatars",
            "tmp/example_group_pic.png",
            None,
            "niji3rd-live-day1",
        );
    }

    /// Doesn't support emoji yet
    #[test]
    fn only_one_avatar_with_emoji() {
        generate_group_pic(
            "tmp/test_one_avatar",
            "tmp/one_avatar_with_emoji.png",
            Some(5),
            "🔥👀🌾🍛",
        );
    }
}

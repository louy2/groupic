#![allow(dead_code)]

use image::{ImageBuffer, Rgba, RgbaImage};

const FONT_DATA: &[u8] = include_bytes!("../NotoSansDisplay-SemiBold.ttf");
const DISCORD_COLOR: Rgba<u8> = Rgba([48, 48, 54, 255]);

fn new_imagebuffer_with_discord_bg(width: u32, height: u32) -> RgbaImage {
    ImageBuffer::from_pixel(width, height, DISCORD_COLOR)
}



#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImage, ImageBuffer, Pixel};
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
    fn stitch_5_avatars_into_1_row() -> std::io::Result<()> {
        let mut row = image::RgbaImage::new(128 * 5, 128);

        let avatar_dir = Path::new("tmp/test_avatars");
        for (i, avatar_entry) in fs::read_dir(avatar_dir)?.take(5).enumerate() {
            let avatar_img = image::open(avatar_entry?.path()).unwrap();
            row.copy_from(&avatar_img, i as u32 * 128, 0).unwrap();
        }

        row.save("tmp/row.png").unwrap();

        Ok(())
    }

    #[test]
    fn stitch_all_avatars_together() -> std::io::Result<()> {
        let avatar_dir = Path::new("tmp/test_avatars");
        let num_of_avatars = fs::read_dir(avatar_dir)?.count();
        let num_of_avatars_in_a_row = 5;
        let num_of_rows = num_of_avatars / num_of_avatars_in_a_row
            + if num_of_avatars % num_of_avatars_in_a_row == 0 {
                0
            } else {
                1
            };

        let mut together = image::RgbaImage::new(128 * 5, 128 * num_of_rows as u32);
        // println!("{:?}", together.dimensions());

        for (i, avatar_entry) in fs::read_dir(avatar_dir)?.enumerate() {
            let avatar_path = avatar_entry?.path();
            let avatar_img = image::open(&avatar_path).unwrap();
            let x_offset = (i % num_of_avatars_in_a_row) as u32 * 128;
            let y_offset = (i / num_of_avatars_in_a_row) as u32 * 128;
            // println!(
            //     "{:#?}: {:?} {:?}",
            //     avatar_path,
            //     avatar_img.dimensions(),
            //     (x_offset, y_offset)
            // );
            together.copy_from(&avatar_img, x_offset, y_offset).unwrap();
        }

        together.save("tmp/together.png").unwrap();

        Ok(())
    }

    #[test]
    fn mask_avatar_with_circle() {
        // circle mask color and radius
        let discord_color = image::Rgb([48, 48, 54]);
        let radius = 64;

        // avatar image to mask
        let avatar_path = Path::new("avatar.png");
        let mut avatar_img = image::open(avatar_path).unwrap().into_rgb8();

        // mask the avatar cover parts outside circle with
        for (x, y, p) in avatar_img.enumerate_pixels_mut() {
            if (x as i64 - 64) * (x as i64 - 64) + (y as i64 - 64) * (y as i64 - 64)
                >= radius * radius
            {
                p.0.copy_from_slice(&discord_color.0);
            }
        }

        avatar_img.save("masked_avatar.png").unwrap();
    }

    #[test]
    fn render_text_header() {
        // prepare header image buffer
        let header_w = 128 * 5;
        let header_h = 64;
        let discord_color = image::Rgba([48, 48, 54, 255]);
        let mut header_img = ImageBuffer::from_pixel(header_w, header_h, discord_color);

        // load font
        let font = rusttype::Font::try_from_bytes(super::FONT_DATA).expect("error loading font");
        let font_size = rusttype::Scale::uniform(54.0);

        let text = "niji3rd-live-day1";

        // layout the glyphs
        let v_metrics = font.v_metrics(font_size);
        let layout: Vec<_> = font
            .layout(text, font_size, rusttype::point(0.0, v_metrics.ascent))
            .collect();
        let layout_h = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
        let layout_w = {
            let min_x = layout.first().unwrap().pixel_bounding_box().unwrap().min.x;
            let max_x = layout.last().unwrap().pixel_bounding_box().unwrap().min.x;
            (max_x - min_x) as u32
        };
        // println!("layout_h= {:?}, layout_w= {:?}", layout_h, layout_w)

        // center the render in header
        let x_offset = (header_w - layout_w) / 2;
        let y_offset = (header_h - layout_h) / 2;

        // render the text into header image
        for glyph in layout {
            let bounding_box = glyph.pixel_bounding_box().unwrap();
            glyph.draw(|x, y, v| {
                header_img
                    .get_pixel_mut(
                        x_offset + x + bounding_box.min.x as u32,
                        y_offset + y + bounding_box.min.y as u32,
                    )
                    .blend(&image::Rgba([240, 240, 240, f32::floor(v * 255.0) as u8]))
            })
        }

        header_img.save("header.png").unwrap();
    }

    #[test]
    fn generate_full_group_pic() {
        // configure the group pic
        let num_of_avatars_in_a_row = 5_u32;
        let header_h = 64;
        let header_text = "niji3rd-live-day1";
        let header_font_size = 54;
        let mask_radius = 64;
        let avatars_dir = Path::new("tmp/test_avatars");

        // calculate the rest of the configuration
        let num_of_avatars = fs::read_dir(avatars_dir).unwrap().count() as u32;
        let num_of_rows = num_of_avatars / num_of_avatars_in_a_row
            + if num_of_avatars % num_of_avatars_in_a_row == 0 {
                0
            } else {
                1
            };
        let group_pic_w = 128 * num_of_avatars_in_a_row;
        let group_pic_h = header_h + 128 * num_of_rows;

        // prepare the image buffer
        let mut group_pic = ImageBuffer::from_pixel(group_pic_w, group_pic_h, DISCORD_COLOR);
        // println!("{:?}", group_pic.dimensions());

        // render the header
        let font = rusttype::Font::try_from_bytes(super::FONT_DATA).expect("error loading font");
        let scale = rusttype::Scale::uniform(header_font_size as f32);
        let v_metrics = font.v_metrics(scale);
        let layout: Vec<_> = font
            .layout(header_text, scale, rusttype::point(0.0, v_metrics.ascent))
            .collect();
        let layout_h = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
        let layout_w = {
            let min_x = layout.first().unwrap().pixel_bounding_box().unwrap().min.x;
            let max_x = layout.last().unwrap().pixel_bounding_box().unwrap().min.x;
            (max_x - min_x) as u32
        };
        let x_offset = (group_pic_w - layout_w) / 2;
        let y_offset = (header_h - layout_h) / 2;
        for glyph in layout {
            let bounding_box = glyph.pixel_bounding_box().unwrap();
            glyph.draw(|x, y, v| {
                group_pic
                    .get_pixel_mut(
                        x_offset + x + bounding_box.min.x as u32,
                        y_offset + y + bounding_box.min.y as u32,
                    )
                    .blend(&image::Rgba([240, 240, 240, (v * 255.0) as u8]))
            })
        }

        // mask and tile the avatars
        for (i, avatar_path) in fs::read_dir(avatars_dir).unwrap().enumerate() {
            let avatar_path = avatar_path.unwrap().path();
            let mut avatar_img = image::open(&avatar_path).unwrap().into_rgba8();
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

        group_pic.save("example_group_pic.png").unwrap();
    }
}

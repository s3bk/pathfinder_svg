use svg_dom::{Svg};
use svg_draw::{DrawSvg};
use svg_text::{FontCollection, Font};
use std::sync::Arc;
use pathfinder_rasterize::Rasterizer;
use pathfinder_color::ColorF;
use std::path::Path;
use image::{io::Reader, Rgba};

fn main() {
    env_logger::init();
    let fonts = Arc::new(FontCollection::from_fonts(vec![
        Font::load(include_bytes!("../../resources/latinmodern-math.otf")),
        Font::load(include_bytes!("../../resources/NotoNaskhArabic-Regular.ttf")),
        Font::load(include_bytes!("../../resources/NotoSerifBengali-Regular.ttf")),
    ]));

    let test_data = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../test_data"));
    let svgs = test_data.join("svg").read_dir().unwrap();
    let pngs = test_data.join("png");
    for e in svgs.filter_map(Result::ok) {
        if !e.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let name = e.file_name();

        let data = std::fs::read(e.path()).unwrap();

        let svg = Svg::from_data(&data).unwrap();
        let scene = DrawSvg::new(svg, fonts.clone()).compose();
        let image = Rasterizer::new().rasterize(scene, Some(ColorF::white()));

        let mut png_path = pngs.join(&name);
        png_path.set_extension("png");

        let reference = Reader::open(png_path).unwrap().decode().unwrap().to_rgba8();
        
        for (y, (ref_row, im_row)) in reference.rows().zip(image.rows()).enumerate() {
            for (x, (Rgba(ref_px), Rgba(im_px))) in ref_row.zip(im_row).enumerate() {
                let delta = ref_px.iter().zip(im_px).map(|(&r, &i)| ((r as i16) - (i as i16)).abs()).max().unwrap();
                if delta > 2 {
                    image.save("mismatch.png").unwrap();
                    panic!("pixel mismatch at {:?} x={}, y={}: expected {:?}, found {:?}", name, x, y, ref_px, im_px);
                }
            }
        }

        println!("{:?} OK", name);
    }
}
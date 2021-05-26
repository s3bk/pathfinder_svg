#[macro_use] extern crate log;

use pathfinder_renderer::{
    scene::{Scene}
};

#[macro_use]
mod macros;

mod prelude {
    pub use pathfinder_renderer::scene::Scene;
    pub use pathfinder_geometry::{
        vector::{Vector2F, vec2f},
        transform2d::Transform2F,
        rect::RectF,
    };
    pub use svg_dom::prelude::*;
    pub use crate::{
        DrawItem, Resolve, Interpolate, Compose,
        draw::{DrawOptions, DrawContext},
    };
    pub use svgtypes::{Length, LengthUnit};
}

mod path;
mod rect;
mod polygon;
mod ellipse;
mod attrs;
mod gradient;
mod resolve;
mod filter;
mod g;
mod draw;
mod svg;
#[cfg(feature="text")]
mod text;
mod animate;
mod paint;

pub use prelude::*;

#[cfg(feature="text")]
use svg_text::FontCollection;

use std::sync::Arc;

pub trait Resolve {
    type Output;
    fn resolve(&self, options: &DrawOptions) -> Self::Output;
    fn try_resolve(&self, options: &DrawOptions) -> Option<Self::Output> {
        Some(self.resolve(options))
    }
}

pub trait DrawItem {
    fn draw_to(&self, scene: &mut Scene, options: &DrawOptions);
    fn bounds(&self, options: &DrawOptions) -> Option<RectF>;
}

pub trait Interpolate: Clone {
    fn lerp(self, to: Self, x: f32) -> Self;
    fn scale(self, x: f32) -> Self;
}
impl<T> Interpolate for Option<T> where T: Interpolate {
    fn lerp(self, to: Self, x: f32) -> Self {
        match (self, to) {
            (Some(a), Some(b)) => Some(a.lerp(b, x)),
            _ => None
        }
    }
    fn scale(self, x: f32) -> Self {
        self.map(|v| v.scale(x))
    }
}

#[cfg(not(feature="text"))]
impl DrawItem for TagText {
    fn draw_to(&self, scene: &mut Scene, options: &DrawOptions) {
    }
    fn bounds(&self, options: &DrawOptions) -> Option<RectF> {
        None
    }
}

pub trait Compose {
    fn compose(self, rhs: Self) -> Self;
}
impl<T: Compose> Compose for Option<T> {
    fn compose(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Some(a), Some(b)) => Some(a.compose(b)),
            (a, b) => a.or(b)
        }
    }
}

macro_rules! draw_items {
    ($name:ident { $($variant:ident($data:ty), )* }) => {
        impl DrawItem for $name {
            fn draw_to(&self, scene: &mut Scene, options: &DrawOptions) {
                match *self {
                    $( $name::$variant ( ref tag ) => tag.draw_to(scene, options), )*
                    _ => {}
                }
            }
            fn bounds(&self, options: &DrawOptions) -> Option<RectF> {
                match *self {
                    $( $name::$variant ( ref tag ) => tag.bounds(options), )*
                    _ => None
                }
            }
        }
    }
}

draw_items!(
    Item {
        Path(TagPath),
        G(TagG),
        Rect(TagRect),
        Polygon(TagPolygon),
        Polyline(TagPolyline),
        Line(TagLine),
        Ellipse(TagEllipse),
        Circle(TagCircle),
        Svg(TagSvg),
        Use(TagUse),
        Text(TagText),
    }
);

pub struct DrawSvg {
    svg: Svg,

    #[cfg(feature="text")]
    fallback_fonts: Option<Arc<FontCollection>>,
}
impl DrawSvg {
    pub fn new_without_fonts(svg: Svg) -> DrawSvg {
        DrawSvg {
            svg: svg,
            
            #[cfg(feature="text")]
            fallback_fonts: None
        }
    }
    #[cfg(feature="text")]
    pub fn new(svg: Svg, fallback_fonts: Arc<FontCollection>) -> DrawSvg {
        DrawSvg {
            svg,
            fallback_fonts: Some(fallback_fonts)
        }
    }
    pub fn compose(&self) -> Scene {
        self.compose_with_transform(Transform2F::default())
    }

    pub fn compose_with_transform(&self, transform: Transform2F) -> Scene {
        let ctx = self.ctx();
        let mut options = DrawOptions::new(&ctx);
        options.transform = transform;
        //options.view_box = Some(RectF::new(Vector2F::zero(), Vector2F::new(10., 10.)));
        self.compose_with_options(&options)
    }

    pub fn compose_with_options(&self, options: &DrawOptions) -> Scene {
        let mut scene = Scene::new();
        
        if let Some(vb) = self.view_box() {
            scene.set_view_box(options.transform * vb);
        }
        self.svg.root.draw_to(&mut scene, options);
        scene
    }

    pub fn compose_with_viewbox(&self, view_box: RectF) -> Scene {
        let ctx = self.ctx();
        let options = DrawOptions::new(&ctx);
        let mut scene = Scene::new();
        scene.set_view_box(options.transform * view_box);
        self.svg.root.draw_to(&mut scene, &options);
        scene
    }

    pub fn compose_to_with_transform(&self, scene: &mut Scene, transform: Transform2F) {
        let ctx = self.ctx();
        let mut options = DrawOptions::new(&ctx);
        options.transform = transform;
        self.svg.root.draw_to(scene, &options);
    }

    /// get the viewbox (computed if missing)
    pub fn view_box(&self) -> Option<RectF> {
        let ctx = self.ctx();
        let options = DrawOptions::new(&ctx);
        
        if let Item::Svg(TagSvg { view_box: Some(r), width, height, .. }) = &*self.svg.root {
            if let Some(size) = Vector(
                width.unwrap_or(r.width),
                height.unwrap_or(r.height)
            ).try_resolve(&options) {
                return Some(RectF::new(Vector2F::zero(), size));
            }
        }
        self.svg.root.bounds(&options)
    }

    pub fn ctx(&self) -> DrawContext {
        #[cfg(feature="text")]
        if let Some(ref f) = self.fallback_fonts {
            DrawContext::new(&self.svg, f.clone())
        } else {
            DrawContext::new_without_fonts(&self.svg)
        }

        #[cfg(not(feature="text"))]
        DrawContext::new_without_fonts(&self.svg)
    }
}

use font::SvgGlyph;
pub fn draw_glyph(glyph: &SvgGlyph, scene: &mut Scene, transform: Transform2F) {
    let ctx = DrawContext::new_without_fonts(&*glyph.svg);
    let mut options = DrawOptions::new(&ctx);
    options.transform = transform * Transform2F::from_scale(Vector2F::new(1.0, -1.0));
    glyph.item.draw_to(scene, &options);
}

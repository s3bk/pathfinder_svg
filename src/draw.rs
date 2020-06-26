use crate::prelude::*;
use crate::{Svg, Paint, ClipPathAttr, TagClipPath};
use crate::animate::{Time};
use pathfinder_content::{
    outline::{Outline},
    stroke::{OutlineStrokeToFill, StrokeStyle, LineCap, LineJoin},
    fill::{FillRule},
};
use pathfinder_renderer::{
    scene::{Scene, DrawPath, ClipPath, ClipPathId},
    paint::Paint as PaPaint,
};
use pathfinder_color::ColorU;
use svgtypes::{Length, Color};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct DrawContext<'a> {
    pub svg: &'a Svg,

    #[cfg(feature="debug")]
    pub debug_font: Arc<FontCollection>,
    #[cfg(feature="debug")]
    pub debug: bool,

    pub dpi: f32,
}
impl<'a> DrawContext<'a> {
    pub fn new(svg: &'a Svg) -> Self {
        DrawContext {
            svg,
            dpi: 75.0,

            #[cfg(feature="debug")]
            debug_font: Arc::new(FontCollection::debug()),
            #[cfg(feature="debug")]
            debug: false,
        }
    }
    pub fn resolve(&self, id: &str) -> Option<&Arc<Item>> {
        self.svg.named_items.get(id)
    }
    pub fn resolve_href(&self, href: &str) -> Option<&Arc<Item>> {
        if href.starts_with("#") {
            self.resolve(&href[1..])
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct DrawOptions<'a> {
    pub ctx: &'a DrawContext<'a>,

    pub fill: Paint,
    pub fill_rule: FillRule,
    pub fill_opacity: f32,

    pub stroke: Paint,
    pub stroke_style: StrokeStyle,
    pub stroke_opacity: f32,

    pub opacity: f32,

    pub transform: Transform2F,

    pub clip_path: ClipPathAttr,
    pub clip_rule: FillRule,

    pub view_box: Option<RectF>,

    pub time: Time,
}
impl<'a> DrawOptions<'a> {
    pub fn new(ctx: &'a DrawContext<'a>) -> DrawOptions<'a> {
        DrawOptions {
            ctx,
            opacity: 1.0,
            fill: Paint::black(),
            fill_rule: FillRule::EvenOdd,
            fill_opacity: 1.0,
            stroke: Paint::None,
            stroke_opacity: 1.0,
            stroke_style: StrokeStyle {
                line_width: 1.0,
                line_cap: LineCap::Butt,
                line_join: LineJoin::Bevel,
            },
            transform: Transform2F::from_scale(10.),
            clip_path: ClipPathAttr::None,
            clip_rule: FillRule::EvenOdd,
            view_box: None,
            time: Time::start(),
        }
    }
    pub fn bounds(&self, rect: RectF) -> Option<RectF> {
        let has_fill = matches!(*self,
            DrawOptions { ref fill, fill_opacity, .. }
            if fill.is_visible() && fill_opacity > 0.);
        let has_stroke = matches!(*self,
            DrawOptions { ref stroke, stroke_opacity, .. }
            if stroke.is_visible() && stroke_opacity > 0.
        );

        if has_stroke {
            Some(self.transform * rect.dilate(self.stroke_style.line_width))
        } else if has_fill {
            Some(self.transform * rect)
        } else {
            None
        }
    }
    fn resolve_paint(&self, paint: &Paint, opacity: f32) -> Option<PaPaint> {
        let opacity = opacity * self.opacity;
        let alpha = (255. * opacity) as u8;
        match *paint {
            Paint::Color(ref c) => Some(PaPaint::from_color(c.color_u(opacity))),
            Paint::Ref(ref id) => match self.ctx.svg.named_items.get(id).map(|arc| &**arc) {
                Some(Item::LinearGradient(ref gradient)) => Some(PaPaint::from_gradient(gradient.build(self, opacity))),
                Some(Item::RadialGradient(ref gradient)) => Some(PaPaint::from_gradient(gradient.build(self, opacity))),
                r => {
                    dbg!(id, r);
                    None
                }
            }
            _ => None
        }
    }
    pub fn debug_outline(&self, scene: &mut Scene, path: &Outline, color: ColorU) {
        dbg!(path);
        let paint_id = scene.push_paint(&PaPaint::from_color(color));
        scene.push_draw_path(DrawPath::new(path.clone(), paint_id));
    }
    fn clip_path_id(&self, scene: &mut Scene) -> Option<ClipPathId> {
        if let ClipPathAttr::Ref(ref id) = self.clip_path {
            if let Some(Item::ClipPath(p)) = self.ctx.resolve(id).map(|t| &**t) {
                let outline = p.build(self);
                //self.debug_outline(scene, &outline, ColorU::new(0, 255, 0, 50));

                let mut clip_path = ClipPath::new(outline);
                clip_path.set_fill_rule(self.clip_rule);
                return Some(scene.push_clip_path(clip_path));
            } else {
                println!("clip path missing: {}", id);
            }
        }
        None
    }
    pub fn draw(&self, scene: &mut Scene, path: &Outline) {
        let clip_path_id = self.clip_path_id(scene);
        
        if let Some(ref fill) = self.resolve_paint(&self.fill, self.fill_opacity) {
            let outline = path.clone().transformed(&self.transform);
            let paint_id = scene.push_paint(fill);
            let mut draw_path = DrawPath::new(outline, paint_id);
            draw_path.set_fill_rule(self.fill_rule);
            draw_path.set_clip_path(clip_path_id);
            scene.push_draw_path(draw_path);
        }
        if let Some(ref stroke) = self.resolve_paint(&self.stroke, self.stroke_opacity) {
            if self.stroke_style.line_width > 0. {
                let paint_id = scene.push_paint(stroke);
                let mut stroke = OutlineStrokeToFill::new(path, self.stroke_style);
                stroke.offset();
                let path = stroke.into_outline();
                let mut draw_path = DrawPath::new(path.transformed(&self.transform), paint_id);
                draw_path.set_clip_path(clip_path_id);
                scene.push_draw_path(draw_path);
            }
        }
    }
    pub fn transform(&mut self, transform: Transform2F) {
        self.transform = self.transform * transform;
    }
    pub fn apply(&self, attrs: &Attrs) -> DrawOptions<'a> {
        let mut stroke_style = self.stroke_style;
        if let Some(length) = attrs.stroke_width {
            stroke_style.line_width = length.num as f32;
        }
        let new = DrawOptions {
            clip_path: attrs.clip_path.clone().unwrap_or_else(|| self.clip_path.clone()),
            clip_rule: attrs.clip_rule.unwrap_or(self.clip_rule),
            opacity: self.opacity * attrs.opacity.unwrap_or(1.0),
            transform: self.transform * attrs.transform.get(self),
            fill: attrs.fill.get(self),
            fill_rule: attrs.fill_rule.unwrap_or(self.fill_rule),
            fill_opacity: attrs.fill_opacity.unwrap_or(self.fill_opacity),
            stroke: attrs.stroke.get(self),
            stroke_style,
            stroke_opacity: attrs.stroke_opacity.unwrap_or(self.stroke_opacity),
            #[cfg(feature="debug")]
            debug_font: self.debug_font.clone(),
            .. *self
        };
        debug!("fill {:?} + {:?} -> {:?}", self.fill, attrs.fill, new.fill);
        debug!("stroke {:?} + {:?} -> {:?}", self.stroke, attrs.stroke, new.stroke);
        new
    }
    pub fn resolve_length(&self, length: Length) -> Option<f32> {
        let scale = match length.unit {
            LengthUnit::None => 1.0,
            LengthUnit::Cm => self.ctx.dpi * (1.0 / 2.54),
            LengthUnit::Em => unimplemented!(),
            LengthUnit::Ex => unimplemented!(),
            LengthUnit::In => self.ctx.dpi,
            LengthUnit::Mm => self.ctx.dpi * (1.0 / 25.4),
            LengthUnit::Pc => unimplemented!(),
            LengthUnit::Percent => return None,
            LengthUnit::Pt => self.ctx.dpi * (1.0 / 75.),
            LengthUnit::Px => 1.0
        };
        Some(length.num as f32 * scale)
    }
    pub fn resolve_length_along(&self, length: Length, axis: Axis) -> Option<f32> {
        let scale = match length.unit {
            LengthUnit::None => 1.0,
            LengthUnit::Cm => self.ctx.dpi * (1.0 / 2.54),
            LengthUnit::Em => unimplemented!(),
            LengthUnit::Ex => unimplemented!(),
            LengthUnit::In => self.ctx.dpi,
            LengthUnit::Mm => self.ctx.dpi * (1.0 / 25.4),
            LengthUnit::Pc => unimplemented!(),
            LengthUnit::Percent => return match axis {
                Axis::X => self.view_box.map(|r| r.width()),
                Axis::Y => self.view_box.map(|r| r.height()),
            },
            LengthUnit::Pt => self.ctx.dpi * (1.0 / 75.),
            LengthUnit::Px => 1.0
        };
        Some(length.num as f32 * scale)
    }
    pub fn resolve_vector(&self, v: Vector) -> Vector2F {
        let x = self.resolve_length_along(v.0, Axis::X).unwrap();
        let y = self.resolve_length_along(v.1, Axis::Y).unwrap();
        vec2f(x, y)
    }

    pub fn resolve_rect(&self, rect: &Rect) -> RectF {
        RectF::new(self.resolve_vector(rect.origin()), self.resolve_vector(rect.size()))
    }

    pub fn apply_viewbox(&mut self, size: Vector, view_box: &Rect) {
        let view_box = self.resolve_rect(view_box);
        let size = self.resolve_vector(size);
        self.transform = self.transform
            * Transform2F::from_scale(view_box.size().inv() * size)
            * Transform2F::from_translation(-view_box.origin());
        
        self.view_box = Some(view_box);
    }
}

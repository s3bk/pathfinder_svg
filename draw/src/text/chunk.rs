use unicode_bidi::{Level, LevelRun, BidiInfo};
use svg_text::{FontCollection, Layout};
use svg_dom::TextFlow;

/// basic unit of text
pub struct Chunk {
    text: String,
    runs: Vec<(Level, LevelRun)>
}
impl Chunk {
    pub fn new(text: &str, direction: TextFlow) -> Chunk {
        let level = match direction {
            TextFlow::LeftToRight => Level::ltr(),
            TextFlow::RightToLeft => Level::rtl(),
        };
        let bidi_info = BidiInfo::new(text, Some(level));
        let para = &bidi_info.paragraphs[0];
        let line = para.range.clone();
        let (levels, runs) = bidi_info.visual_runs(para, line);
        let runs = runs.into_iter().map(|run| (levels[run.start], run)).collect();
        Chunk {
            text: text.into(),
            runs
        }
    }
    pub fn layout(&self, font: &FontCollection) -> ChunkLayout {
        let mut offset = 0.0;
        let mut parts = Vec::with_capacity(self.runs.len());
        for (level, run) in self.runs.iter() {
            let layout = font.layout_run(&self.text[run.clone()], level.is_rtl());

            let advance = layout.metrics.advance;
            let (run_offset, next_offset) = match level.is_rtl() {
                false => (offset, offset + advance),
                true => (offset - advance, offset - advance),
            };
            parts.push((run.start, run_offset, layout));
            offset = next_offset;
        }

        ChunkLayout { parts, advance: offset }
    }
}
pub struct ChunkLayout {
    pub parts: Vec<(usize, f32, Layout)>,
    pub advance: f32,
}
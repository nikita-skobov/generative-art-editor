use color::Hsl;
use macroquad::prelude::*;
use egui_macroquad::egui::{self, Ui};
use ::rand::prelude::*;
use rand_chacha::ChaCha8Rng;

mod dependency_resolution;
mod draw;
mod color;

use draw::{BlockContext, DraggableBlock, BlockConnectionNode, OutputResult, FONT_SIZE, FONT_SIZE_F32};
use draw::ConnectionType::*;

pub const BLOCK_WIDTH_PER_INPUT: f32 = 50.0;
pub const BLOCK_HEIGHT: f32 = 40.0;
pub const TIMELINE_ITEM_HEIGHT: f32 = 30.0;
pub const ERR_FONT_SIZE: u16 = 20;
pub const ERR_FONT_SIZE_F32: f32 = ERR_FONT_SIZE as f32;

pub fn screen_size() -> (f32, f32) {
    (screen_width(), screen_height())
}

#[derive(Debug, Clone)]
pub enum InputValue {
    Number(f64),
    Point((f32, f32)),
    Color(Color),
    Selection((usize, Vec<String>)),
    ListNumbers(Vec<f64>),
    ListPoints(Vec<(f32, f32)>),
}

impl From<(f32, f32)> for InputValue {
    fn from(orig: (f32, f32)) -> Self {
        InputValue::Point(orig)
    }
}

impl From<f64> for InputValue {
    fn from(x: f64) -> Self {
        InputValue::Number(x)
    }
}

impl From<f32> for InputValue {
    fn from(x: f32) -> Self {
        InputValue::Number(x as f64)
    }
}

impl From<Color> for InputValue {
    fn from(x: Color) -> Self {
        InputValue::Color(x)
    }
}

impl From<&[&str]> for InputValue {
    fn from(x: &[&str]) -> Self {
        InputValue::Selection((0, x.iter().map(|s| s.to_string()).collect()))
    }
}

impl InputValue {
    pub fn as_f32(&self) -> f32 {
        match self {
            InputValue::Number(x) => *x as _,
            x => {
                macroquad::logging::error!("Expected f32, found {:?}", x);
                0.0
            }
        }
    }
    pub fn as_f64(&self) -> f64 {
        match self {
            InputValue::Number(x) => *x,
            x => {
                macroquad::logging::error!("Expected f32, found {:?}", x);
                0.0
            }
        }
    }
    pub fn as_point(&self) -> (f32, f32) {
        match self {
            InputValue::Point(x) => *x,
            x => {
                macroquad::logging::error!("Expected Point, found {:?}", x);
                (0.0, 0.0)
            }
        }
    }
    pub fn as_list_points(&self) -> &Vec<(f32, f32)> {
        match self {
            InputValue::ListPoints(x) => x,
            x => {
                macroquad::logging::error!("Expected Point, found {:?}", x);
                static X: Vec<(f32, f32)> = vec![];
                &X
            }
        }
    }
    pub fn as_color(&self) -> Color {
        match self {
            InputValue::Color(x) => *x,
            x => {
                macroquad::logging::error!("Expected Color, found {:?}", x);
                Color::new(0.0, 0.0, 0.0, 0.0)
            }
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            InputValue::Selection((i, options)) => {
                options[*i].as_str()
            }
            x => {
                macroquad::logging::error!("Expected string options, found {:?}", x);
                ""
            }
        }
    }
}

pub struct BlockRunContext {
    pub screen_w: f32,
    pub screen_h: f32,
    pub percentage: f32,
    pub rng: ChaCha8Rng,
}

impl BlockRunContext {
    fn get_screen_space(&self) -> (f32, f32) {
        (self.screen_w, self.screen_h)
    }
}

pub struct Timeline {
    pub bar_pos: f32,
    pub max_height: f32,
    pub min_height: f32,
    /// a percentage (0 - 1) of how much vertical
    /// screen space to take up
    pub percentage_height: f32,
    /// must be at least 5s
    pub total_time_secs: f32,
    pub running: bool,
}
impl Timeline {
    pub fn new(percentage_height: f32) -> Self {
        Self { bar_pos: 0.0, max_height: 300.0, min_height: 80.0, percentage_height, total_time_secs: 30.0, running: false }
    }
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }
    pub fn min_height(mut self, min_height: f32) -> Self {
        self.min_height = min_height;
        self
    }
    pub fn dimensions(&self) -> (f32, f32, f32, f32) {
        let (s_width, s_height) = screen_size();
        let mut height = s_height * self.percentage_height;
        if height > self.max_height {
            height = self.max_height;
        }
        if height < self.min_height {
            height = self.min_height;
        }
        let y = s_height - height;
        (0.0, y, s_width, height)
    }
    pub fn handle_input(&mut self, open_item: &mut Option<usize>, timeline_items: &[TimelineItem]) {
        if is_key_pressed(KeyCode::Space) {
            self.running = !self.running;
        }

        let (mx, my) = mouse_position();
        if !is_mouse_button_pressed(MouseButton::Left) { return }

        for (i, item) in timeline_items.iter().enumerate().rev() {
            if mx >= item.x && mx < item.x + item.length && my >= item.y && my < item.y + TIMELINE_ITEM_HEIGHT {
                // if item is open, and it was clicked again, we set it to be closed.
                if let Some(index) = open_item {
                    if *index == i {
                        *open_item = None;
                        return;
                    }
                }
                // otherwise, open it:
                *open_item = Some(i);
                return;
            }
        }
        // if no timeline items were clicked, then check if we clicked inside the timeline window
        let (_, y, _, _) = self.dimensions();
        if my > y {
            self.bar_pos = mx;
        }
    }
    pub fn run(&mut self, timeline_items: &[TimelineItem], screen_space: (f32, f32), error_queue: &mut ErrorQueue, seed: &mut u64) {
        let (_, _, width, _) = self.dimensions();
        let step_per_1s = width / self.total_time_secs;
        let step_per_frame = step_per_1s / 60.0; // TODO: is this right?...

        // TODO: calculate which timeline items it's touching, and render them
        let mut should_run_items = vec![];
        for item in timeline_items {
            if self.bar_pos >= item.x && self.bar_pos < item.x + item.length {
                let percentage = (self.bar_pos - item.x) / item.length;
                should_run_items.push((item.y, percentage, item));
            }
        }
        // sort the items by their height. things higher up in the timeline
        // get rendered last (ie: above)
        should_run_items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        // now they are sorted in order where the first items are the lowest in the timeline:
        for (_, percentage, item) in should_run_items {
            let mut ctx = BlockRunContext {
                screen_w: screen_space.0,
                screen_h: screen_space.1,
                percentage,
                rng: ChaCha8Rng::seed_from_u64(*seed),
            };
            if !error_queue.has_errors() {
                if let Err(e) = item.blocks.run(&mut ctx) {
                    self.running = false;
                    // if this is the first error message,
                    // add an extra error message that explains how
                    // to clear errors.
                    if error_queue.errors.len() == 0 {
                        let e2 = format!("Error during evaluation. Pausing preview. Close all error messages to resume");
                        error_queue.errors.push(ErrorMessage { e: e2 });
                    }
                    error_queue.errors.push(ErrorMessage { e });
                }
            }
        }

        if self.running {
            self.bar_pos += step_per_frame;
            if self.bar_pos > width {
                self.bar_pos = 0.0;
            }
        }
    }
    pub fn draw(&self, timeline_items: &[TimelineItem]) {
        let (x, y, w, h) = self.dimensions();
        draw_rectangle(x, y, w, h, BEIGE);
        let percentage_of_5s = 5.0 / self.total_time_secs;
        let width_per_5s = w * percentage_of_5s;
        let width_per_1s = width_per_5s / 5.0;
        let mut current_mark = 0.0;
        let mut current_time = 0;
        let s_height = screen_height();
        while current_mark < w {
            draw_line(current_mark, y, current_mark, s_height, 1.0, BLACK);
            draw_text(&format!("{current_time}s"), current_mark + 2.0, s_height - 2.0, 16.0, BLACK);
            current_time += 5;
            for _ in 0..5 {
                current_mark += width_per_1s;
                draw_line(current_mark, y, current_mark, s_height, 1.0, GRAY);
            }
        }
        for item in timeline_items {
            draw_rectangle(item.x, item.y, item.length, TIMELINE_ITEM_HEIGHT, item.color);
        }
        draw_line(self.bar_pos, y, self.bar_pos, s_height, 1.0, RED);
    }
}

pub struct TimelineItem {
    pub x: f32,
    pub y: f32,
    pub length: f32,
    pub blocks: BlockContext,
    pub color: Color,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SubWindowShown {
    BlockSelection,
    ValueEditing,
}

pub struct EditorWindow {
    pub width: f32,
    pub bottom_margin: f32,
    pub window_shown: SubWindowShown,
}
impl EditorWindow {
    pub fn new() -> Self {
        Self {
            window_shown: SubWindowShown::BlockSelection,
            width: 350.0,
            bottom_margin: 12.0,
        }
    }
    pub fn dimensions(&self, timeline: &Timeline) -> (f32, f32, f32, f32) {
        let s_width = screen_width();
        let (_, timeline_y, _, _) = timeline.dimensions();
        (s_width - self.width, 0.0, self.width, timeline_y - self.bottom_margin)
    }
    pub fn draw(
        &mut self,
        timeline: &Timeline,
        item: Option<&mut TimelineItem>,
        seed: &mut u64,
        global_rng: &mut ChaCha8Rng,
        available_blocks: &[(fn () -> DraggableBlock, &str)],
    ) {
        let (x, y, w, h) = self.dimensions(timeline);
        egui_macroquad::ui(|egui_ctx| {
            let mut visuals = egui::Visuals::dark();
            visuals.window_shadow.extrusion = 0.0;
            visuals.popup_shadow.extrusion = 0.0;
            egui_ctx.set_visuals(visuals);
            egui::Window::new("")
                .collapsible(false)
                .title_bar(false)
                .fixed_size((w, h))
                .fixed_pos((x, y))
                .default_size((w, h))
                .resizable(false)
                .show(egui_ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.selectable_value(&mut self.window_shown, SubWindowShown::BlockSelection, "Blocks");
                                ui.selectable_value(&mut self.window_shown, SubWindowShown::ValueEditing, "Edit Values");
                            });
                            ui.separator();

                            match &self.window_shown {
                                SubWindowShown::BlockSelection => {
                                    if let Some(item) = item {
                                        ui.label("Click on a block to add it to the canvas");
                                        ui.separator();
                                        for (block_add_fn, block_name) in available_blocks {
                                            if ui.button(*block_name).clicked() {
                                                let mut b = block_add_fn();
                                                let random_x = global_rng.gen_range(0.0..w);
                                                let random_y = global_rng.gen_range(0.0..h);
                                                b.x = random_x;
                                                b.y = random_y;
                                                item.blocks.add_block(b);
                                            }
                                        }
                                    } else {
                                        ui.label("Select a timeline item first to add blocks");
                                    }
                                }
                                SubWindowShown::ValueEditing => {
                                    if let Some(item) = item {
                                        let (_, _, width, _) = timeline.dimensions();
                                        let width_per_second = width / timeline.total_time_secs;
                                        self.draw_block_set(ui, width_per_second, item, seed);
                                    }
                                }
                            }
                        });
                });
        });
    }
    pub fn draw_block_set(&self, ui: &mut Ui, width_per_second: f32, timeline_item: &mut TimelineItem, seed: &mut u64) {
        let mut duration = timeline_item.length / width_per_second;
        egui::Grid::new("my_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .show(ui, |ui| {
                ui.label("x");
                ui.add(egui::DragValue::new(&mut timeline_item.x).speed(0.2));
                ui.end_row();
                ui.label("y");
                ui.add(egui::DragValue::new(&mut timeline_item.y).speed(0.2));
                ui.end_row();
                ui.label("duration (s)");
                ui.add(egui::DragValue::new(&mut duration).speed(0.2));
                ui.end_row();
                ui.label("color");
                let c = &mut timeline_item.color;
                let mut rgb = [c.r, c.g, c.b];
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    c.r = rgb[0];
                    c.g = rgb[1];
                    c.b = rgb[2];
                }
                ui.end_row();
                ui.label("random seed");
                ui.add(egui::DragValue::new(seed).speed(1.0));
            });
        ui.separator();
        timeline_item.length = duration * width_per_second;

        let block_set = &mut timeline_item.blocks;
        for (i, block) in block_set.blocks.iter_mut().enumerate() {
            let block = match block {
                Some(b) => b,
                None => continue,
            };
            ui.heading(&block.name);
            egui::Grid::new(&format!("{i}_{}", block.name))
                .num_columns(2)
                .spacing([40.0, 4.0])
                .show(ui, |ui| {
                    for input in block.inputs.iter_mut() {
                        ui.label(&input.name);
                        match &mut input.value {
                            InputValue::Number(x) => {
                                ui.add(egui::DragValue::new(x).speed(1.0));
                            }
                            InputValue::Color(c) => {
                                let mut rgb = [c.r, c.g, c.b];
                                if ui.color_edit_button_rgb(&mut rgb).changed() {
                                    c.r = rgb[0];
                                    c.g = rgb[1];
                                    c.b = rgb[2];
                                }
                            }
                            InputValue::Selection((selected, alternatives)) => {
                                egui::ComboBox::from_id_source(format!("{}{}", block.name, i)).show_index(
                                    ui,
                                    selected,
                                    alternatives.len(),
                                    |i| alternatives[i].to_owned()
                                );
                            }
                            InputValue::Point((x, y)) => {
                                // TODO: how to edit a pt?
                                ui.add(egui::DragValue::new(x).speed(1.0));
                                ui.label(&format!("{}_y", input.name));
                                ui.add(egui::DragValue::new(y).speed(1.0));
                            }
                            // the rest are all only editable dynamically, so
                            // no need to show them in the manual editor
                            _ => {
                                let mut txt = "DYNAMICONLY";
                                let val = egui::TextEdit::singleline(&mut txt).interactive(false);
                                ui.add_enabled(false, val);
                            }
                        }
                        ui.end_row();
                    }
                });
            ui.separator();
        }
    }
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + std::f32::consts::E.powf(-x))
}

#[derive(Default, Debug)]
pub struct ErrorQueue {
    pub errors: Vec<ErrorMessage>,
}
impl ErrorQueue {
    pub fn has_errors(&self) -> bool {
        self.errors.len() > 0
    }
    pub fn draw(&mut self) {
        let mut remove = None;
        let mut y = 0.0;
        for (i, err) in self.errors.iter().enumerate() {
            let measured = measure_text(&err.e, None, ERR_FONT_SIZE, 1.0);
            draw_rectangle(0.0, y, measured.width + 30.0, measured.height, RED);
            draw_text(&err.e, 0.0, y + measured.offset_y, ERR_FONT_SIZE_F32, WHITE);
            draw_text("X", measured.width + 10.0, y + measured.offset_y, ERR_FONT_SIZE_F32, WHITE);
            if is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mouse_position();
                if mx >= measured.width + 10.0 && mx < measured.width + 30.0
                    && my >= y && my < y + measured.height
                {
                    remove = Some(i);
                }
            }
            y += measured.height;
        }
        if let Some(remove_index) = remove {
            self.errors.remove(remove_index);
        }
    }
}

#[derive(Debug, Default)]
pub struct ErrorMessage {
    pub e: String,
}

pub struct CircleBlock;
impl CircleBlock {
    const NAME: &'static str = "Circle";

    pub fn run(
        inputs: &Vec<&InputValue>,
        _ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let x = &inputs[0].as_f32();
        let y = &inputs[1].as_f32();
        let radius = &inputs[2].as_f32();
        let color = &inputs[3].as_color();
        draw_circle(*x, *y, *radius, *color);
        None
    }

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block2 = DraggableBlock::default();
        draggable_block2.inputs = vec![
            BlockConnectionNode::new("cx", Inputs),
            BlockConnectionNode::new("cy", Inputs),
            BlockConnectionNode::new("radius", Inputs),
            BlockConnectionNode::new_with_input_type("color", BLACK.into(), Inputs),
        ];
        draggable_block2.name = format!("{} {}", draggable_block2.id, Self::NAME);
        draggable_block2.run_fn = Self::run;
        draggable_block2.calculate_width();
        draggable_block2
    }
}

pub struct SquareBlock;
impl SquareBlock {
    const NAME: &'static str = "Square";

    pub fn run(
        inputs: &Vec<&InputValue>,
        _ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let x = &inputs[0].as_f32();
        let y = &inputs[1].as_f32();
        let size = &inputs[2].as_f32();
        let color = &inputs[3].as_color();
        draw_rectangle_lines(*x, *y, *size, *size, 2.0, *color);
        None
    }

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block2 = DraggableBlock::default();
        draggable_block2.inputs = vec![
            BlockConnectionNode::new("x0", Inputs),
            BlockConnectionNode::new("y0", Inputs),
            BlockConnectionNode::new("size", Inputs),
            BlockConnectionNode::new_with_input_type("color", BLACK.into(), Inputs),
        ];
        draggable_block2.name = format!("{} {}", draggable_block2.id, Self::NAME);
        draggable_block2.run_fn = Self::run;
        draggable_block2.calculate_width();
        draggable_block2
    }
}


pub struct FlattenPointsBlock;
impl FlattenPointsBlock {
    const NAME: &'static str = "FlattenPoints";

    pub fn run(
        inputs: &Vec<&InputValue>,
        _ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        match inputs[0] {
            InputValue::ListPoints(_) => {
                // TODO: do something smarter than clone.
                Some(vec![OutputResult::SingleValue(inputs[0].clone())])
            }
            _ => None,
        }
    }

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block2 = DraggableBlock::default();
        draggable_block2.inputs = vec![
            BlockConnectionNode::new_with_input_type("pts", InputValue::Point((0.0, 0.0)), Inputs),
        ];
        draggable_block2.outputs = vec![
            BlockConnectionNode::new_with_input_type("pts", InputValue::ListPoints(vec![]), Outputs),
        ];
        draggable_block2.flatten_inputs = true;
        draggable_block2.name = format!("{} {}", draggable_block2.id, Self::NAME);
        draggable_block2.run_fn = Self::run;
        draggable_block2.calculate_width();
        draggable_block2
    }
}



pub struct PointConnectionBlock;
impl PointConnectionBlock {
    const NAME: &'static str = "PointConnection";

    pub fn run(
        inputs: &Vec<&InputValue>,
        _ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let pts = inputs[0].as_list_points();
        // macroquad::logging::info!("{:?}", pts);
        let mut previous_pt: Option<&(f32, f32)> = None;
        for pt in pts.iter() {
            if let Some((prev_x, prev_y)) = previous_pt {
                draw_line(*prev_x, *prev_y, pt.0, pt.1, 2.0, RED);
                previous_pt = Some(pt);
            } else {
                previous_pt = Some(pt);
            }
        }
        None
    }

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block2 = DraggableBlock::default();
        draggable_block2.inputs = vec![
            BlockConnectionNode::new_with_input_type("pts", InputValue::ListPoints(vec![]), Inputs),
        ];
        // draggable_block2.outputs = vec![
        //     BlockConnectionNode::new_with_input_type("pts", InputValue::ListPoints(vec![]), Outputs),
        // ];
        draggable_block2.name = format!("{} {}", draggable_block2.id, Self::NAME);
        draggable_block2.run_fn = Self::run;
        draggable_block2.calculate_width();
        draggable_block2
    }
}


pub struct RandomPointBlock;
impl RandomPointBlock {
    const NAME: &'static str = "RandomPoint";

    pub fn run(
        inputs: &Vec<&InputValue>,
        ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let pt0 = *&inputs[0].as_point();
        let pt1 = *&inputs[1].as_point();
        let pt2 = *&inputs[2].as_point();
        let pt3 = *&inputs[3].as_point();
        if ctx.rng.gen_bool(0.5) {
            Some(vec![OutputResult::SingleValue(pt0.into()), OutputResult::SingleValue(pt2.into())])
        } else {
            Some(vec![OutputResult::SingleValue(pt1.into()), OutputResult::SingleValue(pt3.into())])
        }
    }

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block2 = DraggableBlock::default();
        draggable_block2.inputs = vec![
            BlockConnectionNode::new_with_input_type("pt0", InputValue::Point((0.0, 0.0)), Inputs),
            BlockConnectionNode::new_with_input_type("pt1", InputValue::Point((0.0, 0.0)), Inputs),
            BlockConnectionNode::new_with_input_type("pt2", InputValue::Point((0.0, 0.0)), Inputs),
            BlockConnectionNode::new_with_input_type("pt3", InputValue::Point((0.0, 0.0)), Inputs),
        ];
        draggable_block2.outputs = vec![
            BlockConnectionNode::new_with_input_type("ptA", InputValue::Point((0.0, 0.0)), Outputs),
            BlockConnectionNode::new_with_input_type("ptB", InputValue::Point((0.0, 0.0)), Outputs),
        ];
        draggable_block2.name = format!("{} {}", draggable_block2.id, Self::NAME);
        draggable_block2.run_fn = Self::run;
        draggable_block2.calculate_width();
        draggable_block2
    }
}


pub struct LineBlock;
impl LineBlock {
    const NAME: &'static str = "Line";

    pub fn run(
        inputs: &Vec<&InputValue>,
        _ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let x1 = &inputs[0].as_f32();
        let y1 = &inputs[1].as_f32();
        let x2 = &inputs[2].as_f32();
        let y2 = &inputs[3].as_f32();
        let color = &inputs[4].as_color();
        draw_line(*x1, *y1, *x2, *y2, 2.0, *color);
        None
    }

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block2 = DraggableBlock::default();
        draggable_block2.inputs = vec![
            BlockConnectionNode::new("x1", Inputs),
            BlockConnectionNode::new("y1", Inputs),
            BlockConnectionNode::new("x2", Inputs),
            BlockConnectionNode::new("y2", Inputs),
            BlockConnectionNode::new_with_input_type("color", BLACK.into(), Inputs),
        ];
        draggable_block2.name = format!("{} {}", draggable_block2.id, Self::NAME);
        draggable_block2.run_fn = Self::run;
        draggable_block2.calculate_width();
        draggable_block2
    }
}


pub struct PtExtractBlock;
impl PtExtractBlock {
    const NAME: &'static str = "PtExtract";

    pub fn run(
        inputs: &Vec<&InputValue>,
        _ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let (x, y) = *&inputs[0].as_point();
        Some(vec![OutputResult::SingleValue(x.into()), OutputResult::SingleValue(y.into())])
    }

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block2 = DraggableBlock::default();
        draggable_block2.inputs = vec![
            BlockConnectionNode::new_with_input_type("pt", InputValue::Point((0.0, 0.0)), Inputs),
        ];
        draggable_block2.outputs = vec![
            BlockConnectionNode::new("x", Outputs),
            BlockConnectionNode::new("y", Outputs),
        ];
        draggable_block2.name = format!("{} {}", draggable_block2.id, Self::NAME);
        draggable_block2.run_fn = Self::run;
        draggable_block2.calculate_width();
        draggable_block2
    }
}


pub struct PtCombineBlock;
impl PtCombineBlock {
    const NAME: &'static str = "PtCombine";

    pub fn run(
        inputs: &Vec<&InputValue>,
        _ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let x = *&inputs[0].as_f32();
        let y = *&inputs[1].as_f32();
        Some(vec![OutputResult::SingleValue((x, y).into())])
    }

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block2 = DraggableBlock::default();
        draggable_block2.inputs = vec![
            BlockConnectionNode::new("x", Inputs),
            BlockConnectionNode::new("y", Inputs),
        ];
        draggable_block2.outputs = vec![
            BlockConnectionNode::new_with_input_type("pt", (0.0, 0.0).into(), Outputs),
        ];
        draggable_block2.name = format!("{} {}", draggable_block2.id, Self::NAME);
        draggable_block2.run_fn = Self::run;
        draggable_block2.calculate_width();
        draggable_block2
    }
}


pub struct HslColorBlock;
impl HslColorBlock {
    const NAME: &'static str = "HslColor";

    pub fn run(
        inputs: &Vec<&InputValue>,
        _ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let h = &inputs[0].as_f32();
        let s = &inputs[1].as_f32();
        let l = &inputs[1].as_f32();
        let hsl = Hsl::new(*h, *s, *l);
        let (r, g, b) = hsl.hsl_to_rgb();
        let c = Color::from_rgba(r, g, b, 255);
        Some(vec![OutputResult::SingleValue(c.into())])
    }
    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block2 = DraggableBlock::default();
        draggable_block2.inputs = vec![
            BlockConnectionNode::new_with_input_type("hue", 0.0.into(), Inputs),
            BlockConnectionNode::new_with_input_type("saturation", 0.5.into(), Inputs),
            BlockConnectionNode::new_with_input_type("lightness", 0.5.into(), Inputs),
        ];
        draggable_block2.outputs = vec![
            BlockConnectionNode::new_with_input_type("color", WHITE.into(), Outputs)
        ];
        draggable_block2.name = format!("{} {}", draggable_block2.id, Self::NAME);
        draggable_block2.run_fn = Self::run;
        draggable_block2.calculate_width();
        draggable_block2
    }
}

pub struct RandOffSetBlock;
impl RandOffSetBlock {
    const NAME: &'static str = "RandomOffset";

    pub fn run(
        inputs: &Vec<&InputValue>,
        ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let s = &inputs[0].as_f32();
        let low = *&inputs[1].as_f32();
        let high = *&inputs[2].as_f32();
        let range = low..high;
        let val = if range.is_empty() {
            low
        } else {
            ctx.rng.gen_range(low..high)
        };
        let c = *s + val;
        Some(vec![OutputResult::SingleValue(c.into())])
    }
    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block2 = DraggableBlock::default();
        draggable_block2.inputs = vec![
            BlockConnectionNode::new("source", Inputs),
            BlockConnectionNode::new_with_input_type("low", (-10.0).into(), Inputs),
            BlockConnectionNode::new_with_input_type("high", 10.0.into(), Inputs),
        ];
        draggable_block2.outputs = vec![
            BlockConnectionNode::new("value", Outputs)
        ];
        draggable_block2.name = format!("{} {}", draggable_block2.id, Self::NAME);
        draggable_block2.run_fn = Self::run;
        draggable_block2.calculate_width();
        draggable_block2
    }
}

pub struct IterationBlock;
impl IterationBlock {
    const NAME: &'static str = "Iterate";

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block = DraggableBlock::default();
        draggable_block.inputs = vec![
            BlockConnectionNode::new_with_input_type("pass", 0.0.into(), Inputs),    
            BlockConnectionNode::new_with_input_type("start", 0.0.into(), Inputs),
            BlockConnectionNode::new_with_input_type("end", 100.0.into(), Inputs),
            BlockConnectionNode::new_with_input_type("by", 10.0.into(), Inputs)
        ];
        draggable_block.outputs = vec![
            BlockConnectionNode::new("pass", Outputs),    
            BlockConnectionNode::new("value", Outputs),
        ];
        draggable_block.name = format!("{} {}", draggable_block.id, Self::NAME);
        draggable_block.run_fn = Self::run;
        draggable_block.calculate_width();
        draggable_block
    }
    pub fn run(
        inputs: &Vec<&InputValue>,
        ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let pass = inputs[0].as_f32();
        let start = *&inputs[1].as_f32();
        let end = *&inputs[2].as_f32();
        let by = *&inputs[3].as_f32();
        let mut out1 = vec![];
        let mut out2 = vec![];
        let mut value = start;
        while value <= end {
            out1.push(InputValue::Number(pass as _));
            out2.push(InputValue::Number(value as _));
            value += by;
        }
    
        Some(vec![OutputResult::Iteration(out1), OutputResult::Iteration(out2)])
    }
}


pub struct GridBlock;
impl GridBlock {
    const NAME: &'static str = "Grid";

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block = DraggableBlock::default();
        draggable_block.inputs = vec![
            BlockConnectionNode::new_with_input_type("rows", 10.0.into(), Inputs),
            BlockConnectionNode::new_with_input_type("cols", 10.0.into(), Inputs)
        ];
        draggable_block.outputs = vec![
            BlockConnectionNode::new("xi", Outputs),
            BlockConnectionNode::new("yi", Outputs),
        ];
        draggable_block.name = format!("{} {}", draggable_block.id, Self::NAME);
        draggable_block.run_fn = Self::run;
        draggable_block.calculate_width();
        draggable_block
    }
    pub fn run(
        inputs: &Vec<&InputValue>,
        ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let rows = &inputs[0].as_f32();
        let cols = &inputs[1].as_f32();
        let (s_width, s_height) = ctx.get_screen_space();
        let height_per_row = s_height / rows;
        let width_per_col = s_width / cols;
        let rows = *rows as u32;
        let cols = *cols as u32;
        let mut y = height_per_row / 2.0;
        let mut out1 = vec![];
        let mut out2 = vec![];
        for _ in 0..rows {
            let mut x = width_per_col / 2.0;
            for _ in 0..cols {
                out1.push(InputValue::Number(x as _));
                out2.push(InputValue::Number(y as _));
                x += width_per_col;
            }
            y += height_per_row;
        }
    
        Some(vec![OutputResult::Iteration(out1), OutputResult::Iteration(out2)])
    }
}

pub struct SquareGridBlock;
impl SquareGridBlock {
    const NAME: &'static str = "SquareGrid";

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block = DraggableBlock::default();
        draggable_block.inputs = vec![
            BlockConnectionNode::new_with_input_type("dimension", 10.0.into(), Inputs),
        ];
        draggable_block.outputs = vec![
            BlockConnectionNode::new_with_input_type("pt0", InputValue::Point((0.0, 0.0)), Outputs),
            BlockConnectionNode::new_with_input_type("pt1", InputValue::Point((0.0, 0.0)), Outputs),
            BlockConnectionNode::new_with_input_type("pt2", InputValue::Point((0.0, 0.0)), Outputs),
            BlockConnectionNode::new_with_input_type("pt3", InputValue::Point((0.0, 0.0)), Outputs),
        ];
        draggable_block.name = format!("{} {}", draggable_block.id, Self::NAME);
        draggable_block.run_fn = Self::run;
        draggable_block.calculate_width();
        draggable_block
    }
    pub fn run(
        inputs: &Vec<&InputValue>,
        ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let dimension = &inputs[0].as_f32();
        let (s_width, s_height) = ctx.get_screen_space();
        // we fix it into a square, so use the min size
        let screen_size = s_width.min(s_height);
        let size_per_tile = screen_size / dimension;
        let mut out1 = vec![];
        let mut out2 = vec![];
        let mut out3 = vec![];
        let mut out4 = vec![];
        let dim_u32 = *dimension as u32;
        let mut y = 0.0;
        for _ in 0..dim_u32 {
            let mut x = 0.0;
            for _ in 0..dim_u32 {
                out1.push(InputValue::Point((x as _, y as _)));
                out2.push(InputValue::Point((x + size_per_tile, y as _)));
                out3.push(InputValue::Point((x + size_per_tile, y + size_per_tile)));
                out4.push(InputValue::Point((x, y + size_per_tile)));
                x += size_per_tile;
            }
            y += size_per_tile;
        }
    
        Some(vec![
            OutputResult::Iteration(out1),
            OutputResult::Iteration(out2),
            OutputResult::Iteration(out3),
            OutputResult::Iteration(out4),
        ])
    }
}

pub struct ClockBlock;
impl ClockBlock {
    const NAME: &'static str = "Clock";

    pub fn to_draggable_block() -> DraggableBlock {
        let mut draggable_block3 = draw::DraggableBlock::default();
        draggable_block3.inputs = vec![
            draw::BlockConnectionNode::new_with_input_type("smoothing", 
                [
                    "none",
                    "sigmoid",
                ][..].into(),
                draw::ConnectionType::Inputs
            ),
            draw::BlockConnectionNode::new_with_input_type("sigmoid sensitivity", 6.0.into(), draw::ConnectionType::Inputs),
            draw::BlockConnectionNode::new_with_input_type("scale_by", 10.0.into(), draw::ConnectionType::Inputs),
        ];
        draggable_block3.outputs = vec![
            draw::BlockConnectionNode::new("time", draw::ConnectionType::Outputs),
        ];
        draggable_block3.name = format!("{} {}", draggable_block3.id, Self::NAME);
        draggable_block3.run_fn = Self::run;
        draggable_block3.calculate_width();
        draggable_block3
    }
    pub fn run(
        inputs: &Vec<&InputValue>,
        ctx: &mut BlockRunContext,
    ) -> Option<Vec<OutputResult>> {
        let mut time = ctx.percentage;
        // default is linear, so use time as is
        if inputs[0].as_str() == "sigmoid" {
            let sigmoid_sensitivity = inputs[1].as_f32();
            time = sigmoid((time * sigmoid_sensitivity) - (sigmoid_sensitivity / 2.0));
        }
    
        // this allows the user to do arbitrary scaling
        // ie: to use time for stuff other than [0, 1]
        time *= inputs[2].as_f32();
    
        Some(vec![OutputResult::SingleValue(time.into())])
    }
}

#[macroquad::main("BasicShapes")]
async fn main() {
    // macroquad::logging::info!("{}", rng.gen_range(0..100));
    let mut window = EditorWindow::new();
    let mut timeline = Timeline::new(0.25);
    let available_blocks = [
        (ClockBlock::to_draggable_block as fn() -> DraggableBlock, ClockBlock::NAME),
        (GridBlock::to_draggable_block, GridBlock::NAME),
        (CircleBlock::to_draggable_block, CircleBlock::NAME),
        (HslColorBlock::to_draggable_block, HslColorBlock::NAME),
        (RandOffSetBlock::to_draggable_block, RandOffSetBlock::NAME),
        (SquareBlock::to_draggable_block, SquareBlock::NAME),
        (SquareGridBlock::to_draggable_block, SquareGridBlock::NAME),
        (LineBlock::to_draggable_block, LineBlock::NAME),
        (RandomPointBlock::to_draggable_block, RandomPointBlock::NAME),
        (PtExtractBlock::to_draggable_block, PtExtractBlock::NAME),
        (IterationBlock::to_draggable_block, IterationBlock::NAME),
        (FlattenPointsBlock::to_draggable_block, FlattenPointsBlock::NAME),
        (PointConnectionBlock::to_draggable_block, PointConnectionBlock::NAME),
        (PtCombineBlock::to_draggable_block, PtCombineBlock::NAME),
    ];
    let block_context = draw::BlockContext::new([]);
    let mut errors = ErrorQueue::default();
    // TODO: each item should have its own rand seed, and then no need to pass
    // it to window
    let timeline_item = TimelineItem {
        x: 100.0,
        y: 700.0,
        length: 150.0,
        blocks: block_context,
        color: RED,
    };
    let mut timeline_items = vec![timeline_item];
    let mut open_item: Option<usize> = None;
    let mut rand_seed: u64 = 101;
    let mut global_rng = ChaCha8Rng::seed_from_u64(rand_seed);
    loop {
        clear_background(WHITE);

        timeline.handle_input(&mut open_item, &timeline_items);

        let (x, _, _, h) = window.dimensions(&timeline);
        timeline.run(&timeline_items, (x, h), &mut errors, &mut rand_seed);
        if let Some(index) = open_item {
            if let Some(item) = timeline_items.get_mut(index) {
                window.draw(&timeline, Some(item), &mut rand_seed, &mut global_rng, &available_blocks[..]);
            } else {
                window.draw(&timeline, None, &mut rand_seed, &mut global_rng, &available_blocks[..]);
            }
        } else {
            window.draw(&timeline, None, &mut rand_seed, &mut global_rng, &available_blocks[..]);
        }

        // the timeline + art gets rendered below
        timeline.draw(&timeline_items);
        if let Some(item_index) = open_item {
            // timeline_items[item_index].blocks.draw(100.0, 100.0);
            let block_context = &mut timeline_items[item_index].blocks;
            block_context.update();
            block_context.draw();
        }

        // egui gets rendered on top
        egui_macroquad::draw();
        errors.draw();
        next_frame().await
    }
}

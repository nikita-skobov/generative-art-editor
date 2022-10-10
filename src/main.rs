use macroquad::prelude::*;
use egui_macroquad::egui::{self, Ui, color_picker::Alpha, Color32};
use std::{collections::HashMap, ops::Index};

pub const BLOCK_WIDTH_PER_INPUT: f32 = 50.0;
pub const BLOCK_HEIGHT: f32 = 40.0;
pub const TIMELINE_ITEM_HEIGHT: f32 = 30.0;

pub fn screen_size() -> (f32, f32) {
    (screen_width(), screen_height())
}

pub struct JoinedSlice<'a> {
    pub a: Result<&'a [Input], &'a [&'a Input]>,
    pub b: &'a [Input],
    pub c: &'a [Input],
}
impl<'a> JoinedSlice<'a> {
    pub fn new(a: &'a [Input], b: &'a [Input]) -> Self {
        Self { a: Ok(a), b, c: &[] }
    }
    pub fn new_ex(a: &'a [&'a Input], b: &'a [Input]) -> Self {
        Self { a: Err(a), b, c: &[] }
    }
    pub fn new_ex3(a: &'a [&'a Input], b: &'a [Input], c: &'a [Input]) -> Self {
        Self { a: Err(a), b, c }
    }
}
impl<'a> Index<usize> for JoinedSlice<'a> {
    type Output = Input;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            match self.a {
                Ok(a) => if index < a.len() {
                    return a.get_unchecked(index)
                }
                Err(a) => if index < a.len() {
                    return a.get_unchecked(index)
                }
            }
            if index < self.b.len() {
                return self.b.get_unchecked(index);
            }
            // actually unsafe:
            self.c.get_unchecked(index)
        }
    }
}

#[derive(Debug)]
pub enum InputValue {
    Number(f32),
    Color(Color),
    Selection((usize, Vec<String>)),
}

impl From<f32> for InputValue {
    fn from(x: f32) -> Self {
        InputValue::Number(x)
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
            InputValue::Number(x) => *x,
            x => {
                macroquad::logging::error!("Expected f32, found {:?}", x);
                0.0
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

pub struct Input {
    pub name: String,
    pub value: InputValue,
}

pub struct Block {
    pub inputs: Vec<Input>,
    pub num_outputs: usize,
    pub name: String,
    pub color: Color,
    pub run_fn: for<'a> fn(JoinedSlice<'a>, &BlockRunContext, &[Block]),
}

pub struct BlockSet {
    pub blocks: Vec<Block>,
}

pub fn get_next_block(next_blocks: &[Block]) -> Option<(&Block, &[Block])> {
    if let Some(first_block) = next_blocks.first() {
        let next = if let Some(next) = next_blocks.get(1..) {
            next
        } else { &[] };
        Some((first_block, next))
    } else {
        None
    }
}

impl BlockSet {
    pub fn draw(&self, x: f32, mut y: f32) {
        for b in self.blocks.iter() {
            b.draw(x, y);
            y += BLOCK_HEIGHT;
        }
    }
    pub fn run(&self, ctx: &BlockRunContext) {
        if let Some((first, next)) = get_next_block(&self.blocks) {
            let first_input = &first.inputs;
            let joined = JoinedSlice::new(first_input, &[]);
            (first.run_fn)(joined, ctx, next);
        }
    }
}

impl Block {
    pub fn draw(&self, x: f32, y: f32) {
        let num_parts = self.inputs.len().max(self.num_outputs);
        let total_width = num_parts as f32 * BLOCK_WIDTH_PER_INPUT;
        let orig_y = y;
        let orig_x = x;
        draw_rectangle(orig_x, orig_y, total_width, BLOCK_HEIGHT, self.color);
        let measured = measure_text(&self.name, None, 26, 1.0);
        let y = y + ((BLOCK_HEIGHT - measured.height) / 2.0);
        draw_text(&self.name, x + 2.0, y + measured.offset_y, 26.0, WHITE);
        let mut x = x + (BLOCK_WIDTH_PER_INPUT / 2.0);
        let triangle_width = 6.0;
        for i in 0..self.inputs.len() {
            let v1 = Vec2::new(x, orig_y);
            let v2 = Vec2::new(x + (triangle_width * 2.0), orig_y);
            let v3 = Vec2::new(x + triangle_width, orig_y + triangle_width);
            draw_triangle(v1, v2, v3, WHITE);
            let v1 = Vec2::new(x, orig_y + BLOCK_HEIGHT - triangle_width);
            let v2 = Vec2::new(x + (triangle_width * 2.0), orig_y + BLOCK_HEIGHT - triangle_width);
            let v3 = Vec2::new(x + triangle_width, orig_y + BLOCK_HEIGHT);
            draw_triangle(v1, v2, v3, WHITE);
            x += BLOCK_WIDTH_PER_INPUT;
        }
        draw_rectangle_lines(orig_x, orig_y, total_width, BLOCK_HEIGHT, 1.0, BLACK);
    }
}

pub struct BlockRunContext {
    pub screen_w: f32,
    pub screen_h: f32,
    pub percentage: f32,
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
    pub fn run(&mut self, timeline_items: &[TimelineItem], screen_space: (f32, f32)) {
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
            let ctx = BlockRunContext {
                screen_w: screen_space.0,
                screen_h: screen_space.1,
                percentage,
            };
            item.blocks.run(&ctx);
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
    pub blocks: BlockSet,
    pub color: Color,
}

pub struct EditorWindow {
    pub width: f32,
    pub bottom_margin: f32,
}
impl EditorWindow {
    pub fn new() -> Self {
        Self {
            width: 350.0,
            bottom_margin: 12.0,
        }
    }
    pub fn dimensions(&self, timeline: &Timeline) -> (f32, f32, f32, f32) {
        let s_width = screen_width();
        let (_, timeline_y, _, _) = timeline.dimensions();
        (s_width - self.width, 0.0, self.width, timeline_y - self.bottom_margin)
    }
    pub fn draw(&self, timeline: &Timeline, item: Option<&mut TimelineItem>) {
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
                            if let Some(item) = item {
                                let (_, _, width, _) = timeline.dimensions();
                                let width_per_second = width / timeline.total_time_secs;
                                self.draw_block_set(ui, width_per_second, item);
                            }
                        });
                });
        });
    }
    pub fn draw_block_set(&self, ui: &mut Ui, width_per_second: f32, timeline_item: &mut TimelineItem) {
        let mut duration = timeline_item.length / width_per_second;
        ui.columns(2, |cols| {
            cols[0].label("x");
            cols[1].add(egui::DragValue::new(&mut timeline_item.x).speed(0.2));
            cols[0].label("y");
            cols[1].add(egui::DragValue::new(&mut timeline_item.y).speed(0.2));
            cols[0].label("duration (s)");
            cols[1].add(egui::DragValue::new(&mut duration).speed(0.2));
        });
        ui.separator();
        timeline_item.length = duration * width_per_second;

        let block_set = &mut timeline_item.blocks;
        for (i, block) in block_set.blocks.iter_mut().enumerate() {
            ui.heading(&block.name);
            ui.columns(2, |cols| {
                for input in block.inputs.iter_mut() {
                    cols[0].label(&input.name);
                    match &mut input.value {
                        InputValue::Number(x) => {
                            cols[1].add(egui::DragValue::new(x).speed(1.0));
                        }
                        InputValue::Color(c) => {
                            let mut color = Color32::from_rgb((c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8);
                            let alpha = Alpha::Opaque;
                            if egui::color_picker::color_picker_color32(&mut cols[1], &mut color, alpha) {
                                *c = Color::from_rgba(color.r(), color.g(), color.b(), 255);
                            }
                        }
                        InputValue::Selection((selected, alternatives)) => {
                            egui::ComboBox::from_id_source(format!("{}{}", block.name, i)).show_index(
                                &mut cols[1],
                                selected,
                                alternatives.len(),
                                |i| alternatives[i].to_owned()
                            );
                        }
                    }
                }
            });
            ui.separator();
        }
    }
}

fn run_grid<'a>(input: JoinedSlice<'a>, ctx: &BlockRunContext, next_blocks: &[Block]) {
    let (first, next) = if let Some(x) = get_next_block(next_blocks) {
        x
    } else { return };

    let rows = input[0].value.as_f32();
    let cols = input[1].value.as_f32();
    let (s_width, s_height) = ctx.get_screen_space();
    let height_per_row = s_height / rows;
    let width_per_col = s_width / cols;
    let rows = rows as u32;
    let cols = cols as u32;
    let mut y = height_per_row / 2.0;
    for _ in 0..rows {
        let mut x = width_per_col / 2.0;
        for _ in 0..cols {
            let inputs = [Input { name: "".into(), value: x.into() }, Input { name: "".into(), value: y.into() }];
            let first_inputs = &first.inputs[..];
            let joined = JoinedSlice::new(&inputs, first_inputs);
            (first.run_fn)(joined, ctx, next);
            x += width_per_col;
        }
        y += height_per_row;
    }
}

fn run_circle<'a>(input: JoinedSlice<'a>, ctx: &BlockRunContext, next_blocks: &[Block]) {
    draw_circle_lines(
        input[0].value.as_f32(),
        input[1].value.as_f32(),
        input[2].value.as_f32(),
        3.0,
        input[3].value.as_color(),
    );
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + std::f32::consts::E.powf(-x))
}

fn run_pass_time2<'a>(input: JoinedSlice<'a>, ctx: &BlockRunContext, next_blocks: &[Block]) {
    let (first, next) = if let Some(x) = get_next_block(next_blocks) {
        x
    } else {
        return
    };

    let mut time = ctx.percentage;
    // default is linear, so use time as is
    if input[2].value.as_str() == "sigmoid" {
        time = sigmoid((time * 6.0) - 3.0);
    }

    // this allows the user to do arbitrary scaling
    // ie: to use time for stuff other than [0, 1]
    time *= input[3].value.as_f32();

    let first_inputs = &first.inputs[..];
    let time_input = [
        Input { name: "".into(), value: 0.0.into() },
        Input { name: "".into(), value: 0.0.into() },
        Input { name: "".into(), value: time.into() }
    ];
    let previous_inputs = [
        &input[0],
        &input[1]
    ];
    let joined = JoinedSlice::new_ex3(&previous_inputs, &time_input, first_inputs);
    (first.run_fn)(joined, ctx, next);
}

#[macroquad::main("BasicShapes")]
async fn main() {
    let window = EditorWindow::new();
    let mut timeline = Timeline::new(0.25);
    let b = Block {
        inputs: vec![
            Input { name: "rows".into(), value: 10.0.into() },
            Input { name: "cols".into(), value: 10.0.into() },
        ],
        num_outputs: 2,
        run_fn: run_grid,
        name: "Grid".into(),
        color: ORANGE,
    };
    let b1 = Block {
        name: "PassTime2".into(),
        color: ORANGE,
        inputs: vec![
            Input { name: "a".into(), value: 0.0.into() },
            Input { name: "b".into(), value: 0.0.into() },
            Input { name: "algorithm".into(), value: [
                "linear",
                "sigmoid",
                ][..].into()
            },
            Input { name: "multiply".into(), value: 10.0.into() },
        ],
        num_outputs: 3,
        run_fn: run_pass_time2,
    };
    let b2 = Block {
        inputs: vec![
            Input { name: "cx".into(), value: 0.0.into() },
            Input { name: "cy".into(), value: 0.0.into() },
            Input { name: "radius".into(), value: 10.0.into() },
            Input { name: "color".into(), value: BLACK.into() },
        ],
        num_outputs: 0,
        run_fn: run_circle,
        name: "Circle".into(),
        color: BLUE,
    };
    let b3 = Block {
        inputs: vec![
            Input { name: "cx".into(), value: 300.0.into() },
            Input { name: "cy".into(), value: 300.0.into() },
            Input { name: "radius".into(), value: 100.0.into() },
            Input { name: "color".into(), value: BLACK.into() },
        ],
        num_outputs: 0,
        run_fn: run_circle,
        name: "Circle".into(),
        color: BLUE,
    };
    let block_set = BlockSet {
        blocks: vec![b, b1, b2],
    };
    let timeline_item = TimelineItem {
        x: 100.0,
        y: 700.0,
        length: 150.0,
        blocks: block_set,
        color: RED,
    };
    let timeline_item2 = TimelineItem {
        x: 120.0,
        y: 710.0,
        length: 200.0,
        blocks: BlockSet { blocks: vec![
            b3
        ] },
        color: ORANGE,
    };
    let mut timeline_items = vec![timeline_item, timeline_item2];
    let mut open_item: Option<usize> = None;
    loop {
        clear_background(WHITE);

        timeline.handle_input(&mut open_item, &timeline_items);

        let (x, _, _, h) = window.dimensions(&timeline);
        timeline.run(&timeline_items, (x, h));
        if let Some(index) = open_item {
            if let Some(item) = timeline_items.get_mut(index) {
                window.draw(&timeline, Some(item));
            } else {
                window.draw(&timeline, None);
            }
        } else {
            window.draw(&timeline, None);
        }

        // the timeline + art gets rendered below
        timeline.draw(&timeline_items);
        if let Some(item_index) = open_item {
            timeline_items[item_index].blocks.draw(100.0, 100.0);
        }

        // egui gets rendered on top
        egui_macroquad::draw();
        next_frame().await
    }
}

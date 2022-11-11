use std::collections::HashMap;

use macroquad::prelude::*;

pub const BLOCK_HEIGHT: f32 = 32.0;
pub const CONNECTION_SIZE: f32 = 10.0;
pub const CONNECTION_SPACING: f32 = 28.0;
pub const FONT_SIZE: u16 = 32;
pub const FONT_SIZE_F32: f32 = FONT_SIZE as f32;

pub trait Boundable {
    fn get_bounds(&self) -> (f32, f32, f32, f32);
}
impl Boundable for (f32, f32, f32, f32) {
    fn get_bounds(&self) -> (f32, f32, f32, f32) {
        *self
    }
}
impl Boundable for &DraggableBlock {
    fn get_bounds(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.width, BLOCK_HEIGHT)
    }
}

pub fn mouse_within_bounds<B: Boundable>(b: B) -> bool {
    mouse_within_bounds_offset(b).is_some()
}

pub fn mouse_within_bounds_offset<B: Boundable>(b: B) -> Option<(f32, f32)> {
    let (mx, my) = mouse_position();
    let (x, y, w, h) = b.get_bounds();
    let within_bounds = mx >= x && mx < x + w && my >= y && my < y + h;
    if within_bounds {
        Some((mx - x, my - y))
    } else {
        None
    }
}

pub struct BlockConnectionNode {
    pub id: Id,
    pub parent_id: Id,
    pub name: String,
    pub value: InputValue,
    pub connection_type: ConnectionType,
    pub is_being_hovered: bool,
    pub is_dragging_line: bool,
}

impl BlockConnectionNode {
    pub fn new<S: AsRef<str>>(s: S, connection_type: ConnectionType) -> Self {
        Self {
            value: InputValue::Number(0.0),
            connection_type,
            id: get_id(),
            parent_id: Id(0),
            name: s.as_ref().into(),
            is_being_hovered: false,
            is_dragging_line: false,
        }
    }
    pub fn new_with_input_type<S: AsRef<str>>(s: S, input_type: InputValue, connection_type: ConnectionType) -> Self {
        Self {
            value: input_type,
            connection_type,
            id: get_id(),
            parent_id: Id(0),
            name: s.as_ref().into(),
            is_being_hovered: false,
            is_dragging_line: false,
        }
    }
    pub fn get_text(&self) -> String {
        format!("({}) {}", self.id.0, self.name)
    }
    pub fn draw(&self, x: f32, y: f32) {
        let color = if self.is_being_hovered { GREEN } else { GRAY };
        draw_rectangle(x, y, CONNECTION_SIZE, CONNECTION_SIZE, color);
        if self.is_being_hovered {
            let padding = 2.0;
            let x = x - padding;
            let y = y + padding;
            let measured = measure_text(&self.get_text(), None, FONT_SIZE, 1.0);
            draw_rectangle(x - measured.width - padding, y - padding, measured.width + padding + padding, measured.height + padding + padding, WHITE);
            draw_rectangle_lines(x - measured.width - padding, y - padding, measured.width + padding + padding, measured.height + padding + padding, 1.0, BLACK);
            draw_text(&self.get_text(), x - measured.width, y + measured.offset_y, FONT_SIZE_F32, BLACK);
        }
        if self.is_dragging_line {
            let (mx, my) = mouse_position();
            draw_line(x, y, mx, my, 1.0, BLACK);
        }
    }
    /// returns if connections have changed
    pub fn update(&mut self, x: f32, y: f32, block_context: &mut BlockContext) -> bool {
        let mut connections_changed = false;
        let bounds = (x, y, CONNECTION_SIZE, CONNECTION_SIZE);
        if mouse_within_bounds(bounds) {
            self.is_being_hovered = true;
            if is_mouse_button_pressed(MouseButton::Left) {
                if block_context.can_drag(self.id) {
                    self.is_dragging_line = true;
                    // if this node is an input, and you try to drag from it
                    // it should break the existing connection. because inputs
                    // can only have 1 connection
                    if let ConnectionType::Inputs = self.connection_type {
                        block_context.remove_connection(self.id);
                        connections_changed = true;
                    }
                }
            }
        } else {
            self.is_being_hovered = false;
        }
        if is_mouse_button_released(MouseButton::Left) {
            block_context.release_drag(self.id);
            if self.is_dragging_line {
                block_context.can_connect(self.parent_id, self.id, self.connection_type, &self.value, (x, y));
                connections_changed = true;
            }
            self.is_dragging_line = false;
        }

        connections_changed
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct Id(usize);

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", self.0)
    }
}

pub fn get_id() -> Id {
    static mut CURRENT_ID: usize = 0;
    let new_id = unsafe {
        let out = Id(CURRENT_ID);
        CURRENT_ID += 1;
        out
    };
    new_id
}

impl From<Id> for Node<Id> {
    fn from(orig: Id) -> Self {
        Self {
            name: Default::default(),
            depends_on: vec![],
            is_dependent_of: vec![],
            value: orig,
        }
    }
}

#[derive(Debug, Clone)]
pub enum OutputResult {
    SingleValue(InputValue),
    Iteration(Vec<InputValue>),
}

#[derive(Debug)]
pub enum InputResult<'a> {
    SingleValue(&'a InputValue),
    SingleValueOwned(InputValue),
    Iteration(&'a Vec<InputValue>),
}

pub struct BlockContext {
    /// the id of the current thing being dragged
    pub currently_dragging: Option<usize>,
    pub blocks: Vec<Option<DraggableBlock>>,
    pub connections: HashMap<(Id, Id), ((f32, f32), (f32, f32))>,
    /// maps an input id to its output id on the other side
    pub input_output: HashMap<Id, Id>,
    /// key is the id of the input
    /// value is the id of the block of the output
    pub inputs: HashMap<Id, Id>,
    pub graph: Graph<Id>,
    /// the indices are within the graph
    pub graph_order: Vec<usize>,
    /// easy way to get a block from the blocks vec via its id
    pub block_ids: HashMap<Id, usize>,
}

fn run_fn_noop(_inputs: &Vec<&InputValue>, _ctx: &mut BlockRunContext) -> Option<Vec<OutputResult>> {
    None
}

impl BlockContext {
    pub fn new<const N: usize>(
        blocks: [DraggableBlock; N],
    ) -> Self {
        let mut out_blocks: Vec<Option<DraggableBlock>> = Vec::with_capacity(blocks.len());
        let mut graph = Graph::default();
        let mut i = 0;
        let mut block_ids = HashMap::new();
        for mut b in blocks {
            let b_id = b.id;
            for input in b.inputs.iter_mut() {
                input.parent_id = b_id;
            }
            for output in b.outputs.iter_mut() {
                output.parent_id = b_id;
            }
            out_blocks.push(Some(b));
            graph.add(b_id);
            block_ids.insert(b_id, i);
            i += 1;
        }
        let graph_order = graph.calculate_order_indices();
        Self {
            currently_dragging: None,
            blocks: out_blocks,
            connections: HashMap::new(),
            inputs: HashMap::new(),
            graph,
            graph_order,
            block_ids,
            input_output: HashMap::new(),
        }
    }

    pub fn add_block(&mut self, mut b: DraggableBlock) {
        let b_id = b.id;
        for input in b.inputs.iter_mut() {
            input.parent_id = b_id;
        }
        for output in b.outputs.iter_mut() {
            output.parent_id = b_id;
        }
        let block_index = self.blocks.len();
        self.blocks.push(Some(b));
        self.graph.add(b_id);
        self.block_ids.insert(b_id, block_index);
        self.graph_order = self.graph.calculate_order_indices();
    }

    pub fn run(&self, ctx: &mut BlockRunContext) -> Result<(), String> {
        let mut previous_outputs: HashMap<Id, OutputResult> = HashMap::new();
        for graph_index in self.graph_order.iter() {
            let node = &self.graph.nodes[*graph_index];
            let id = node.value;
            let block_index = self.block_ids[&id];
            let block = &self.blocks[block_index];
            let block = match block {
                Some(b) => b,
                None => continue,
            };
            let block_id = block.id;
            // macroquad::logging::info!("Rendering {}", block.name);
            let mut has_iteration: Option<(Id, usize)> = None;
            let num_inputs = block.inputs.len();
            // fill in the input for this next run function.
            let mut this_input: Vec<InputResult> = vec![];
            for input in block.inputs.iter() {
                // first, check if we depend on any previous input:
                if let Some(output_id) = self.input_output.get(&input.id) {
                    // output_id is the id of the output value that we depend on.
                    // find the value of the previous iteration for this output id
                    if let Some(previous_value) = previous_outputs.get(output_id) {
                        match previous_value {
                            OutputResult::SingleValue(v) => {
                                this_input.push(InputResult::SingleValue(v));
                            }
                            OutputResult::Iteration(v) => {
                                has_iteration = match has_iteration {
                                    // the length of each dependency's iteration should match
                                    Some((past_id, past_v)) => {
                                        if v.len() != past_v {
                                            return Err(
                                                format!("Block {} depends on multiple iterations whose lengths dont match. Id({}) {} != Id({}) {}", block.id.0, past_id.0, past_v, input.id.0, v.len())
                                            );
                                        }
                                        Some((input.id, v.len()))
                                    }
                                    None => Some((input.id, v.len())),
                                };
                                this_input.push(InputResult::Iteration(v));
                            }
                        }
                    } else {
                        return Err(
                            format!("My block {:?} depends on output node {:?}, but failed to find a value from the previous output map", block_id, output_id)
                        );
                    }
                } else {
                    // if there is none, then use the default value
                    this_input.push(InputResult::SingleValue(&input.value));
                }
            }

            let mut result_outputs = previous_outputs.clone();
            let (_, mut num_iterations) = has_iteration.unwrap_or((Id(0), 1));
            // flatten previous inputs to 1 item if this block wants them flattened
            if block.flatten_inputs {
                let mut this_input_clone = vec![];
                for input in this_input.drain(..) {
                    match input {
                        // if it's an iteration, 'unpack' it
                        InputResult::Iteration(x) => {
                            // if its empty, just use listNumbers, doesn't matter since no one will read it.
                            if x.is_empty() {
                                this_input_clone.push(InputResult::SingleValueOwned(InputValue::ListNumbers(vec![])));
                                continue;
                            }
                            // otherwise, we need to know the type of the inner items
                            let first = x.first().unwrap();
                            match first {
                                InputValue::Number(_) => {
                                    let mut out = vec![];
                                    for val in x.iter() {
                                        out.push(val.as_f64());
                                    }
                                    this_input_clone.push(InputResult::SingleValueOwned(InputValue::ListNumbers(out)));
                                }
                                InputValue::Point(_) => {
                                    let mut out = vec![];
                                    for val in x.iter() {
                                        out.push(val.as_point());
                                    }
                                    this_input_clone.push(InputResult::SingleValueOwned(InputValue::ListPoints(out)));
                                }
                                // TODO: give user error if they tried to flatten a non-flattenable type
                                InputValue::Color(_) => todo!(),
                                InputValue::Selection(_) => todo!(),
                                InputValue::ListNumbers(_) => todo!(),
                                InputValue::ListPoints(_) => todo!(),
                            }
                        }
                        // if single value, we just put it as is.
                        x => this_input_clone.push(x),
                    }
                }
                this_input = this_input_clone;
                num_iterations = 1;
            }
            for i in 0..num_iterations {
                let mut input_vec = Vec::with_capacity(num_inputs);
                for input in this_input.iter() {
                    match input {
                        InputResult::SingleValue(v) => {
                            input_vec.push(*v);
                        }
                        InputResult::SingleValueOwned(v) => {
                            input_vec.push(v);
                        }
                        InputResult::Iteration(values) => {
                            // safe to do because we know each iteration
                            // has exactly num_iteration values
                            input_vec.push(&values[i]);
                        }
                    }
                }
                let res = (block.run_fn)(&input_vec, ctx);
                if let Some(mut result) = res {
                    // fill in the previous output map with this block's
                    // values.
                    // the ids correspond to the indices of the inner result vec
                    for (result_index, result_value) in result.drain(..).enumerate() {
                        let result_id = block.outputs[result_index].id;
                        // if one exists before, we will need to append to it
                        if let Some(previous_val) = result_outputs.get_mut(&result_id) {
                            match previous_val {
                                OutputResult::SingleValue(v) => {
                                    let local_v = std::mem::replace(v, InputValue::Number(0.0));
                                    let mut iteration_values = vec![local_v];
                                    match result_value {
                                        OutputResult::SingleValue(val) => {
                                            iteration_values.push(val);
                                        }
                                        OutputResult::Iteration(i_vals) => {
                                            iteration_values.extend(i_vals);
                                        }
                                    }
                                    *previous_val = OutputResult::Iteration(iteration_values);
                                }
                                OutputResult::Iteration(iteration_vals) => {
                                    match result_value {
                                        OutputResult::SingleValue(v) => {
                                            iteration_vals.push(v);
                                        }
                                        OutputResult::Iteration(i_v) => {
                                            iteration_vals.extend(i_v);
                                        }
                                    }
                                }
                            }
                        } else {
                            // otherwise just insert it
                            result_outputs.insert(result_id, result_value);
                        }
                    }
                }
            }
            previous_outputs = result_outputs;
        }
        Ok(())
    }

    pub fn update(&mut self) {
        let mut connections_changed = false;
        for i in 0..self.blocks.len() {
            let mut b = self.blocks[i].take();
            if let Some(block) = &mut b {
                if block.update(self) {
                    connections_changed = true;
                }
            }
            self.blocks[i] = b;
        }
        // if there were any connection changes, recalculate graph
        if connections_changed {
            self.graph.reset();
            // first, need to add all of our blocks:
            for b in self.blocks.iter() {
                if let Some(b) = b {
                    self.graph.add(b.id);
                }
            }
            // next, for each block, find everything it depends on
            for b in self.blocks.iter() {
                let block = match b {
                    Some(b) => b,
                    None => continue,
                };
                for input in block.inputs.iter() {
                    // if there's an input connection of one of my input ids
                    // that means i depend on the parent of that output
                    if let Some(parent_id) = self.inputs.get(&input.id) {
                        self.graph.add_dependency(block.id, *parent_id);
                    }
                }
            }
            self.graph_order = self.graph.calculate_order_indices();
            // TODO: check if it's valid

            // TODO: remove debugging
            macroquad::logging::info!("New order:");
            for graph_index in self.graph_order.iter() {
                let node = &self.graph.nodes[*graph_index];
                let id = node.value;
                let block_index = self.block_ids[&id];
                let block = &self.blocks[block_index];
                if let Some(block) = block {
                    macroquad::logging::info!("{}", block.name);
                }
            }
        }
    }
    pub fn draw(&mut self) {
        for (_, (pta, ptb)) in self.connections.iter() {
            let (x1, y1) = *pta;
            let (x2, y2) = *ptb;
            draw_line(x1, y1, x2, y2, 1.0, BLACK);
        }
        for b in self.blocks.iter() {
            if let Some(block) = b {
                block.draw();
            }
        }
    }
    pub fn update_connection_positions(&mut self, ids: Vec<Id>, diff_x: f32, diff_y: f32) {
        for ((id_a, id_b), (pt_a, pt_b)) in self.connections.iter_mut() {
            // ids are all of the ids from a single block
            // so its not possible for both id_a and id_b to be
            // in the ids array, so we can do an else if here
            if ids.contains(id_a) {
                pt_a.0 += diff_x;
                pt_a.1 += diff_y;
            } else if ids.contains(id_b) {
                pt_b.0 += diff_x;
                pt_b.1 += diff_y;
            }
        }
    }
    pub fn remove_connection(&mut self, id: Id) {
        let mut remove_key = None;
        for (ids, _) in self.connections.iter() {
            if ids.0 == id || ids.1 == id {
                remove_key = Some(*ids);
                break;
            }
        }
        if let Some(key) = remove_key {
            self.inputs.remove(&key.0);
            self.inputs.remove(&key.1);
            self.input_output.remove(&key.0);
            self.input_output.remove(&key.1);
            self.connections.remove(&key);
        }
    }
    pub fn can_connect(&mut self, my_parent: Id, my_id: Id, my_type: ConnectionType, my_value_type: &InputValue, my_pos: (f32, f32)) {
        macroquad::logging::info!("Trying to connect!");
        for b in self.blocks.iter_mut() {
            if let Some(block) = b {
                let mut found_connection = None;
                // check if the position where the mouse currently is
                // matches the opposite connection type of the current block.
                // ie: if my_type is input, only allow connections to outputs
                // and vice versa
                block.iter_connections_opposite(my_type, |x, y, connection| {
                    // if the type does not match, do not allow the connection
                    match (my_value_type, &connection.value) {
                        (InputValue::Number(_), InputValue::Number(_)) |
                        (InputValue::Point(_), InputValue::Point(_)) |
                        (InputValue::Color(_), InputValue::Color(_)) |
                        (InputValue::Selection(_), InputValue::Selection(_)) => {},
                        (InputValue::ListNumbers(_), InputValue::ListNumbers(_)) => {},
                        (InputValue::ListPoints(_), InputValue::ListPoints(_)) => {},
                        _ => return,
                    };
                    let bounds = (x, y, CONNECTION_SIZE, CONNECTION_SIZE);
                    if mouse_within_bounds(bounds) {
                        found_connection = Some((connection.parent_id, (my_id, connection.id), (my_pos, (x, y))));
                    }
                });
                if let Some((connection_parent, ids, pts)) = found_connection {
                    let (input, output, parent) = match my_type {
                        Inputs => (ids.0, ids.1, connection_parent), // i am the input
                        Outputs => (ids.1, ids.0, my_parent), // the other node is the input
                    };
                    // prevent connecting to an existing input.
                    // each input can only have 1
                    if self.inputs.contains_key(&input) {
                        break;
                    }
                    macroquad::logging::info!("Connected!");
                    self.inputs.insert(input, parent);
                    self.input_output.insert(input, output);
                    self.connections.insert(ids, pts);
                    break;
                }
            }
        }
    }
    pub fn can_drag(&mut self, id: Id) -> bool {
        if self.currently_dragging.is_none() {
            self.currently_dragging = Some(id.0);
            return true;
        }
        false
    }
    pub fn release_drag(&mut self, id: Id) {
        if let Some(previous_address) = self.currently_dragging {
            if previous_address == id.0 {
                self.currently_dragging = None;
            }
        }
    }
}

pub struct DraggableBlock {
    pub id: Id,
    pub name: String,
    pub name_y_offset: f32,
    pub color: Color,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub flatten_inputs: bool,
    pub being_dragged_from: Option<(f32, f32)>,
    pub inputs: Vec<BlockConnectionNode>,
    pub outputs: Vec<BlockConnectionNode>,
    pub run_fn: fn(inputs: &Vec<&InputValue>, ctx: &mut BlockRunContext) -> Option<Vec<OutputResult>>,
}

impl Default for DraggableBlock {
    fn default() -> Self {
        Self {
            id: get_id(),
            name: "".into(),
            name_y_offset: 0.0,
            color: BLUE,
            x: 0.0,
            y: 0.0,
            width: 100.0,
            flatten_inputs: false,
            being_dragged_from: None,
            inputs: vec![],
            outputs: vec![],
            run_fn: run_fn_noop,
        }
    }
}

#[derive(Clone, Copy)]
pub enum ConnectionType {
    Inputs,
    Outputs,
}
use ConnectionType::*;

use crate::{InputValue, dependency_resolution::{Graph, Node}, BlockRunContext};

impl DraggableBlock {
    pub fn iter_connections(&self, connection_type: ConnectionType, mut cb: impl FnMut(f32, f32, &BlockConnectionNode)) {
        let mut x = self.x;
        let mut y = self.y - CONNECTION_SIZE;
        let iterator = match connection_type {
            Inputs => self.inputs.iter(),
            Outputs => {
                y += BLOCK_HEIGHT + CONNECTION_SIZE;
                self.outputs.iter()
            },
        };
        for input_connection in iterator {
            cb(x, y, input_connection);
            x += CONNECTION_SIZE + CONNECTION_SPACING;
        }
    }
    pub fn iter_connections_opposite(&self, connection_type: ConnectionType, mut cb: impl FnMut(f32, f32, &BlockConnectionNode)) {
        let mut x = self.x;
        let mut y = self.y - CONNECTION_SIZE;
        let iterator = match connection_type {
            Outputs => self.inputs.iter(),
            Inputs => {
                y += BLOCK_HEIGHT + CONNECTION_SIZE;
                self.outputs.iter()
            },
        };
        for input_connection in iterator {
            cb(x, y, input_connection);
            x += CONNECTION_SIZE + CONNECTION_SPACING;
        }
    }
    pub fn iter_connections_mut(&mut self, connection_type: ConnectionType, mut cb: impl FnMut(f32, f32, &mut BlockConnectionNode)) {
        let mut x = self.x;
        let mut y = self.y - CONNECTION_SIZE;
        let iterator = match connection_type {
            Inputs => self.inputs.iter_mut(),
            Outputs => {
                y += BLOCK_HEIGHT + CONNECTION_SIZE;
                self.outputs.iter_mut()
            },
        };
        for input_connection in iterator {
            cb(x, y, input_connection);
            x += CONNECTION_SIZE + CONNECTION_SPACING;
        }
    }

    pub fn get_text(&self) -> &str {
        self.name.as_str()
    }

    pub fn calculate_width(&mut self) {
        let max = self.inputs.len().max(self.outputs.len());
        let text_measured = measure_text(&self.get_text(), None, FONT_SIZE, 1.0);
        self.name_y_offset = text_measured.offset_y;
        self.width = (max as f32) * (CONNECTION_SIZE + CONNECTION_SPACING);
        if text_measured.width > self.width {
            self.width = text_measured.width;
        }
    }
    pub fn draw(&self) {
        let DraggableBlock { color, x, y, width, .. } = *self;
        draw_rectangle(x, y, width, BLOCK_HEIGHT, color);
        draw_text(&self.get_text(), x, y + self.name_y_offset, FONT_SIZE_F32, BLACK);
        self.iter_connections(Inputs, |x, y, input| input.draw(x, y));
        self.iter_connections(Outputs, |x, y, input| input.draw(x, y));
    }
    /// returns true if there were any connection changes
    pub fn update(&mut self, block_context: &mut BlockContext) -> bool {
        if let Some((x_off, y_off)) = mouse_within_bounds_offset(&*self) {
            if self.being_dragged_from.is_none() && is_mouse_button_down(MouseButton::Left) {
                if block_context.can_drag(self.id) {
                    self.being_dragged_from = Some((x_off, y_off));
                }
            }
        }
        if is_mouse_button_released(MouseButton::Left) {
            block_context.release_drag(self.id);
            self.being_dragged_from = None;
        }
        if let Some((x_off, y_off)) = self.being_dragged_from {
            let (mx, my) = mouse_position();
            let old_x = self.x;
            let old_y = self.y;
            self.x = mx - x_off;
            self.y = my - y_off;
            // if my position changed, find all of the connections i have
            // with other blocks and update my part of the positions
            if old_x != self.x || old_y != self.y {
                let diff_x = self.x - old_x;
                let diff_y = self.y - old_y;
                let mut ids = Vec::with_capacity(self.inputs.len() + self.outputs.len());
                for i in self.inputs.iter() { ids.push(i.id) }
                for i in self.outputs.iter() { ids.push(i.id) }
                block_context.update_connection_positions(ids, diff_x, diff_y);
            }
        }
        let mut needs_update = false;
        self.iter_connections_mut(Inputs, |x, y, input| {
            if input.update(x, y, block_context) {
                needs_update = true;
            }
        });
        self.iter_connections_mut(Outputs, |x, y, input| {
            if input.update(x, y, block_context) {
                needs_update = true;
            }
        });
        needs_update
    }
}

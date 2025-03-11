use std::{mem::offset_of, str, fmt};
use log::info;
use std::sync::{Arc, Mutex};

use crate::shapes::shape::DrawShape;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StateCommandMode {
    Create,
    Delete,
    Modify,
}

#[derive(Debug, Clone)]
pub struct StateCommand{
    pub state: StateCommandMode,
    pub content: Vec<Arc<Mutex<Box<dyn DrawShape>>>>
}
impl StateCommand{
    pub fn new(state: StateCommandMode, content: Vec<Arc<Mutex<Box<dyn DrawShape>>>>) -> Self {
        Self {
            state,
            content
        }
    }
}

pub struct UndoRedoManager {
    undo_stack: Vec<StateCommand>,
    redo_stack: Vec<StateCommand>,
}

impl UndoRedoManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn push(&mut self, command: StateCommand) {
        self.undo_stack.push(command);
    }

    pub fn undo(&mut self) {
        if let Some(previous_state) = self.undo_stack.pop() {
            self.redo_stack.push(previous_state);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next_state) = self.redo_stack.pop() {
            self.undo_stack.push(next_state);
        }
    }
}

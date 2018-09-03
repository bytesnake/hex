use std::io;
use std::thread;
use std::cell::RefCell;
use std::sync::Arc;

use std::sync::mpsc::{Sender, Receiver, channel};

use termion::{event, input::TermRead};

use tui::backend::MouseBackend;
use tui::layout::{Direction, Group, Rect, Size};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, Item, List, Paragraph, Widget};
use tui::Terminal;

use client;

pub enum Packet {
    NewPlaylist(Vec<String>),
    Output(String)
}

enum Event {
    Input(event::Key),
}

pub struct TextInterface {
    sender: Sender<Packet>,
    receiver: Receiver<Packet>,
    playlist: Vec<String>,
    logs: Vec<String>,
    size: Rect
}

impl TextInterface {
    pub fn new() -> TextInterface {
        let (sender, receiver) = channel();

        TextInterface {
            sender: sender,
            receiver: receiver,
            playlist: Vec::new(),
            logs: Vec::new(),
            size: Rect::default()
        }
    }

    pub fn run(&mut self, sender: Sender<client::Packet>) {
        // Channels
        let (tx, rx) = channel();
        let input_tx = tx.clone();

        // Input
        thread::spawn(move || {
            let stdin = io::stdin();
            for c in stdin.keys() {
                let evt = c.unwrap();
                input_tx.send(Event::Input(evt)).unwrap();
                if evt == event::Key::Char('q') {
                    break;
                }
            }
        });

        let backend = MouseBackend::new().unwrap();
        let mut terminal = Terminal::new(backend).unwrap();

        // First draw call
        terminal.clear().unwrap();
        terminal.hide_cursor().unwrap();
        self.size = terminal.size().unwrap();

        //self.draw(&mut terminal, &app);

        loop {
            let size = terminal.size().unwrap();
            if self.size != size {
                terminal.resize(size).unwrap();
                self.size = size;
            }
        }
    }

    pub fn sender(&self) -> Sender<Packet> {
        self.sender.clone()
    }
}

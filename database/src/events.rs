use rusqlite::{Error, Result};

#[derive(Debug)]
pub struct Event {
    origin: String,
    action: Action
}

#[derive(Debug, Clone)]
pub enum Action {
    Connect(f32),
    PlaySong(String),
    AddSong(String),
    DeleteSong(String)
}

impl Action {
    pub fn with_origin(self, origin: String) -> Event {
        Event {
            origin: origin,
            action: self
        }
    }
}

impl Event {
    pub fn action(&self) -> Action {
        self.action.clone()
    }

    pub fn origin(&self) -> String {
        self.origin.clone()
    }

    pub fn tag(&self) -> &str {
        match self.action {
            Action::Connect(_) => "connect",
            Action::PlaySong(_) => "playsong",
            Action::AddSong(_) => "addsong",
            Action::DeleteSong(_) => "deletesong"
        }
    }

    pub fn data(&self) -> String {
        match &self.action {
            Action::Connect(ref x) => x.to_string(),
            Action::PlaySong(ref x) => x.clone(),
            Action::AddSong(ref x) => x.clone(),
            Action::DeleteSong(ref x) => x.clone()
        }
    }

    pub fn from(origin: String, tag: String, data: String) -> Result<Event> {
        let action = match tag.as_ref() {
            "connect" => Action::Connect(data.parse::<f32>().map_err(|_| Error::InvalidQuery)?),
            "playsong" => Action::PlaySong(data),
            "addsong" => Action::AddSong(data),
            "deletesong" => Action::DeleteSong(data),
            _ => return Err(Error::InvalidQuery)
        };

        Ok(Event {
            origin: origin,
            action: action
        })
    }

}

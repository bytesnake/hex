//! Occuring events in the database
//!
//! This module contains all definition for logging. Any action like connect, play, add and delete is logged 
//! by the server and saved to the database. Furthermore the origin of these action is logged too
//! and wrapped inside `Event`
//! 

#[cfg(feature="rusqlite")]
use rusqlite::{Error, Result};

use crate::objects::TrackKey;

/// An Event occurs from an origin and contains an action. The origin is most of the time an IP
/// address.
#[derive(Debug)]
#[cfg_attr(feature="serde", derive(Serialize, Deserialize))]
pub struct Event {
    origin: String,
    action: Action
}

/// All possible actions
#[derive(Debug, Clone)]
#[cfg_attr(feature="serde", derive(Serialize, Deserialize))]
pub enum Action {
    Connect(f32),
    PlaySong(TrackKey),
    AddSong(TrackKey),
    DeleteSong(TrackKey)
}

impl Action {
    /// Tag the `Action` with an origin and return an `Event`
    pub fn with_origin(self, origin: String) -> Event {
        Event {
            origin: origin,
            action: self
        }
    }
}

impl Event {
    /// Get a copy of the underlying action
    pub fn action(&self) -> Action {
        self.action.clone()
    }

    /// Get a copy of the origin
    pub fn origin(&self) -> String {
        self.origin.clone()
    }

    /// Convert the action tag to string
    pub fn tag(&self) -> &str {
        match self.action {
            Action::Connect(_) => "connect",
            Action::PlaySong(_) => "playsong",
            Action::AddSong(_) => "addsong",
            Action::DeleteSong(_) => "deletesong"
        }
    }

    /// Converts the underlying data to string
    pub fn data_to_string(&self) -> String {
        match &self.action {
            Action::Connect(ref x) => x.to_string(),
            Action::PlaySong(ref x) => x.to_string(),
            Action::AddSong(ref x) => x.to_string(),
            Action::DeleteSong(ref x) => x.to_string()
        }
    }

    #[cfg(feature="rusqlite")]
    /// Convenient function to create an `Event`
    pub fn from(origin: String, tag: String, data: String) -> Result<Event> {
        let action = match tag.as_ref() {
            "connect" => Action::Connect(data.parse::<f32>().map_err(|_| Error::InvalidQuery)?),
            "playsong" => Action::PlaySong(TrackKey::from_str(&data)),
            "addsong" => Action::AddSong(TrackKey::from_str(&data)),
            "deletesong" => Action::DeleteSong(TrackKey::from_str(&data)),
            _ => return Err(Error::InvalidQuery)
        };

        Ok(Event {
            origin: origin,
            action: action
        })
    }

}

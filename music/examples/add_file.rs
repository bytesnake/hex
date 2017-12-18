extern crate music_librar;
extern crate cursive;

use std::fs::File;
use std::io::Read;
use std::env;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

use music_librar::database::UploadFile;
use music_librar::database::Connection;

use cursive::Cursive;
use cursive::direction::Orientation;
use cursive::align::HAlign;                                                     
use cursive::event::EventResult;
use cursive::traits::*;
use cursive::views::{Dialog, OnEventView, SelectView, TextView, LinearLayout, IdView};
use cursive::{view, views};

struct Data {
    id: RefCell<String>,
    title: RefCell<String>,
    album: RefCell<String>,
    artist: RefCell<String>
}

impl Data {
    pub fn new() -> Data {
        Data { id: RefCell::new("".into()), title: RefCell::new("".into()), album: RefCell::new("".into()), artist: RefCell::new("".into()) }
    }

    pub fn set_id(&self, a: &str) {
        *self.id.borrow_mut() = a.into();
    }

    pub fn set_title(&self, a: &str) {
        *self.title.borrow_mut() = a.into();
    }

    pub fn set_album(&self, a: &str) {
        *self.album.borrow_mut() = a.into();
    }

    pub fn set_artist(&self, a: &str) {
        *self.artist.borrow_mut() = a.into();
    }

    pub fn get(&self) -> (String, String, String, String) {
        (self.id.borrow().clone(), self.title.borrow().clone(), self.album.borrow().clone(), self.artist.borrow().clone())
    }

}


fn main() {
    // obtain the filename 
    let filename = env::args().skip(1).next().unwrap();

    // open the audio file and read into a vector
    let mut buf = Vec::new();
    let mut file = File::open(&filename).unwrap();
    file.read_to_end(&mut buf).unwrap();

    // determine metadata
    let audio_file = Rc::new(UploadFile::new(&buf, filename.split(".").last().unwrap()).unwrap());

    // 
    let data = Rc::new(Data::new());

    let mut siv = Cursive::new();

    let titles = audio_file.get_titles_ids();
    let first_title = match titles.first() {
        Some(x) => x.clone(),
        None => panic!("Nothing found!")
    };

    let mut select_title: SelectView<(String, String)> = SelectView::new().h_align(HAlign::Center);
    select_title.add_all(
        titles
            .into_iter().map(|(a,b)| (format!("{} ({})", a, b), (a,b)))
    );

    let mut select_album: SelectView<String> = SelectView::new().h_align(HAlign::Center);
    let mut select_artist: SelectView<String> = SelectView::new().h_align(HAlign::Center);

    let (first_album, first_artist) = audio_file.get_album_artist(&first_title.1, &first_title.0);
    select_album.add_all_str(first_album);
    select_artist.add_all_str(first_artist);

    let d = data.clone();
    let af = audio_file.clone();
    select_title.set_on_select(move |siv: &mut Cursive, res: &(String, String)| {
        d.set_id(&res.1);
        d.set_title(&res.0);

        let (album, artist) = af.get_album_artist(&res.1, &res.0);

        siv.call_on_id("album", |view: &mut views::SelectView| {
            view.clear();
            view.add_all_str(
                album
            );
        });

        siv.call_on_id("artist", |view: &mut views::SelectView| {
            view.clear();
            view.add_all_str(
                artist
            );
        });

    });

    let d = data.clone();
    select_album.set_on_select(move |siv: &mut Cursive, res: &String| {
        d.set_album(res);

    });

    let af = audio_file.clone();
    select_artist.set_on_submit(move |siv: &mut Cursive, res: &str| {
        data.set_artist(res);

        let (id, title, artist, album) = data.get();

        siv.pop_layer();
        siv.add_layer(
            Dialog::around(TextView::new(format!("ID: {}\nTitle: {}\nAlbum: {}\nArtist: {}\n", id, title, album, artist))).title("Added track!")
        );

        let conn = Connection::new();

        conn.insert_track(af.to_entity(id, title, artist, album));

    });
    

    siv.add_layer(
        LinearLayout::new(Orientation::Horizontal)
            .child(Dialog::around(select_title.with_id("title")).title("Select title").fixed_width(20))
            .child(Dialog::around(select_album.with_id("album")).title("Select album").fixed_width(20))
            .child(Dialog::around(select_artist.with_id("artist")).title("Select artist").fixed_width(20))


    );

    siv.run();
}

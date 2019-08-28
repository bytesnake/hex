#[macro_use]
extern crate failure;

use std::env;
use std::thread;
use std::path::PathBuf;
use std::time::Duration;
use telebot::{Bot, functions::ParseMode, error::ErrorKind as ErrorTelegram};
use futures::{Future, Stream, IntoFuture, future::Either};
use hex_database::{Instance, GossipConf};
use telebot::functions::FunctionSendMessage;
use telebot::functions::FunctionSendAudio;
use telebot::functions::FunctionEditMessageText;
use telebot::functions::FunctionGetFile;
use telebot::objects::Message;
use telebot::objects::Audio;

use telebot::file;
use hyper::Uri;
use failure::{Fail, Error};

use hex_conf::Conf;

use crate::download::DownloadProgress;

mod download;
mod upload;
mod external;
mod error;

fn run_bot(instance: &Instance, conf: Conf, path: PathBuf) {
    let key = env::var("TELEGRAM_BOT_KEY").unwrap();

    let mut bot = Bot::new(&key).timeout(200);

    let view = instance.view();
    let view2 = instance.view();
    let view3 = instance.view();
    let view4 = instance.view();
    let view6 = instance.view();

    let external = external::ExternalMusic::new(view4, path.clone(), conf.spotify.clone().unwrap());
    //let spotify2 = spotify.clone();

    let path2 = path.clone();
    let search = bot.new_cmd("/suche")
        .and_then(move |(bot, msg)| {
            let Message { text, chat, .. } = msg;       

            view.search_limited(&text.unwrap(), 0)
                .map_err(|x| format_err!("{:?}", x))
                .map(|x| {
                    let mut result = x.into_iter().take(10)
                        .map(|x| format!("*{}* (_{}_) von {}", 
                            x.title.unwrap_or("unbekannt".into()), 
                            x.album.unwrap_or("unbekannt".into()), 
                            x.interpret.unwrap_or("unbekannt".into())))
                        .collect::<Vec<String>>().join("\n");

                    if result.is_empty() {
                        result = "Kein Ergebnis!".into();
                    }

                    result
                }).into_future()
                .and_then(move |result| bot.message(chat.id, result).parse_mode(ParseMode::Markdown).send())
        })
        .for_each(|_| Ok(()));

    let external = bot.new_cmd("/external")
        .and_then(move |(bot, msg)| {
            let Message { text, chat, .. } = msg;

            match text {
                Some(ref x) if !x.is_empty() => {
                    external.add_playlist(&x);

                    bot.message(chat.id, "Habe playlist hinzugefügt!".into()).send()
                },
                _ => {
                    let current_playlist = external.current_playlist();

                    if let Some(current_playlist) = current_playlist {
                        bot.message(chat.id, format!("Nehme den Song {} in der Playlist {} auf", current_playlist.1, current_playlist.0)).send()
                    } else {
                        bot.message(chat.id, "Nehme gerade keine Playlist auf!".into()).send()
                    }
                }
            }
        })
        .for_each(|_| Ok(()));

    let download = bot.new_cmd("/lade")
        .and_then(move |(bot, msg)| {
            let result = view2.search_limited(&msg.text.unwrap(), 0).unwrap();

            bot.message(msg.chat.id, format!("download 0/{}", result.len())).send()
                .map(|(bot, msg)| (result, bot, msg))
        })
        .and_then(move |(result, bot, msg)| {
            let result_len = result.len();
            let download = download::State::new(&view6, result, path.clone());
            let bot2 = bot.clone();
            let Message {chat, message_id, ..} = msg;
            let chat_id = chat.id;


            let tmp = download.recv
                .map_err(|_| ErrorTelegram::Unknown.into())
                .and_then(move |x| {
                    let DownloadProgress { result, num } = x;

                    match result {
                        Ok((path, track)) => {
                            Either::A(bot
                                .audio(chat_id)
                                .duration(track.duration as i64)
                                .file(file::File::Disk { path: path })
                                .performer(track.interpret.unwrap_or("unbekannt".into()))
                                .title(track.title.unwrap_or("unbekannt".into()))
                                .send()
                                .map_err(|x| { eprintln!("{:?}", x); x }).map(move |_| num)
                            )
                        },
                        Err(err) => {
                            Either::B(
                                bot.message(chat_id, format!("Konnte Song nicht laden = {:?}", err)).send().map_err(|x| { eprintln!("{:?}", x); x }).map(move |_| num)
                            )
                        }
                    }
                })
                .and_then(move |num| bot2.edit_message_text(format!("download {}/{}", num+1, result_len)).chat_id(chat_id).message_id(message_id).send())

                .for_each(|_| Ok(()));

            tokio::spawn(tmp.map_err(|_| ()));

            Ok(())
        })
        .for_each(|_| Ok(()));

    let stream = bot.get_stream(None)
        .filter_map(|(bot, x)| x.message.map(|x| (bot,x)))
        .filter_map(|(bot, x)| {
            let Message { audio, chat, .. } = x;

            audio.map(|x| (bot, x, chat))
        })
        .filter(|(_, x, _)| x.mime_type.as_ref().map(|x| x.starts_with("audio")) == Some(true))
        .and_then(|(bot, x, chat)| {
            let Audio { file_id, mime_type, .. } = x;

            let ext;
            if let Some(mime_type) = mime_type {
                ext = match mime_type.as_str() {
                    "audio/mpeg" => "mp3",
                    "audio/aac" => "aac",
                    "audio/wav" => "wav",
                    "audio/ogg" => "oga",
                    "audio/webm" => "weba",
                    _ => ""
                };
            } else {
                ext = "";
            }

            let file_name = format!("{}.{}", file_id, ext);

            bot.get_file(file_id).send().map(|(bot, y)| (bot, y, file_name, chat))
        })
        .filter_map(|(bot, msg, file_name, chat)| msg.file_path.map(|x| (bot, x, file_name, chat)))
        .and_then(move |(bot, file_path, file_name, chat)| {
            let path2 = path2.clone();
            let download_link = format!("https://api.telegram.org/file/bot{}/{}", key, file_path).parse::<Uri>().unwrap();

            let track = bot.inner.get(download_link)
                .and_then(|x| {
                    x.into_body().concat2()
                })
                .map_err(|x| Error::from(x.context(ErrorTelegram::Hyper)))
                .and_then(move |x| upload::Upload::new(file_name, x.to_vec(), path2)
                          .into_future()
                          .map_err(|x| Error::from(x.context(ErrorTelegram::Channel)))).wait();

            let answ = match track {
                Ok(track) => { 
                    match view3.add_track(track.clone()) {
                        Ok(_) => format!("Habe {} gespeichert", track.title.unwrap()),
                        Err(e) => format!("Fehler ist aufgetreten: {:?}", e)
                    }
                },
                Err(e) => { format!("Fehler is aufgetreten: {:?}", e) }
            };

            bot.message(chat.id, answ).send()
        })
        .for_each(|_| Ok(()));

    let chain = stream.into_future().join(search.join(download.join(external)))
        .inspect(|err| eprintln!("Eventloop crashed = {:?}", err))
        .map_err(|_| ()).map(|_| ());

    tokio::run(chain);
}

fn main() {
    env_logger::init();

    let (conf, path) = hex_conf::Conf::new().unwrap();

    let mut gossip = GossipConf::new();

    if let Some(ref peer) = conf.peer {
        gossip = gossip.addr((conf.host, peer.port)).id(peer.id()).network_key(peer.network_key());
    }

    let instance = Instance::from_file(path.join("music.db"), gossip);

    loop {
        run_bot(&instance, conf.clone(), path.clone());
        thread::sleep(Duration::from_millis(2000));
    }
}

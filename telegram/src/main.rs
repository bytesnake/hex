use std::env;
use telebot::{Bot, functions::ParseMode, error::ErrorKind as ErrorTelegram};
use futures::{Future, Stream};
use hex_database::{Instance, GossipConf};
use telebot::functions::FunctionSendMessage;
use telebot::functions::FunctionSendAudio;
use telebot::functions::FunctionEditMessageText;
use telebot::objects::Message;
use telebot::file::File;
use crate::download::DownloadProgress;

mod download;
mod error;

fn main() {
    let mut bot = Bot::new(&env::var("TELEGRAM_BOT_KEY").unwrap());

    let (conf, path) = hex_conf::Conf::new().unwrap();

    let gossip = GossipConf::new();

    /*if let Some(ref peer) = conf.peer {
        gossip = gossip.addr((conf.host, peer.port)).id(peer.id()).network_key(peer.network_key());
    }*/

    let instance = Instance::from_file(path.join("music.db"), gossip);

    let view1 = instance.view();
    let view2 = instance.view();
    let search = bot.new_cmd("/search")
        .and_then(move |(bot, msg)| {
            let mut result = view1.search_limited(&msg.text.unwrap(), 0).unwrap()
                .into_iter().take(10)
                .map(|x| format!("*{}* (_{}_) von {}", x.title.unwrap_or("unbekannt".into()), x.album.unwrap_or("unbekannt".into()), x.interpret.unwrap_or("unbekannt".into())))
                .collect::<Vec<String>>().join("\n");

            if result.is_empty() {
                result = "Kein Ergebnis!".into();
            }

            bot.message(msg.chat.id, result).parse_mode(ParseMode::Markdown).send()
        })
        .for_each(|_| Ok(()));

    let download = bot.new_cmd("/download")
        .and_then(move |(bot, msg)| {
            let result = view2.search_limited(&msg.text.unwrap(), 0).unwrap();

            bot.message(msg.chat.id, format!("download 0/{}", result.len())).send()
                .map(|(bot, msg)| (result, bot, msg))
        })
        .and_then(move |(result, bot, msg)| {
            let result_len = result.len();
            let download = download::State::new(result, 2, path.clone());
            let bot2 = bot.clone();
            let Message {chat, message_id, ..} = msg;
            let chat_id = chat.id;

            download.recv
                .map_err(|_| ErrorTelegram::Unknown.into())
                .and_then(move |x| {
                    let DownloadProgress { path, track, num } = x;

                    bot
                        .audio(chat_id)
                        .duration(track.duration as i64)
                        .file(File::Disk { path: path })
                        .performer(track.interpret.unwrap_or("unbekannt".into()))
                        .title(track.title.unwrap_or("unbekannt".into()))
                        .send()
                        .map_err(|x| { eprintln!("{:?}", x); x }).map(move |_| num)
                }
                )
                .and_then(move |num| bot2.edit_message_text(format!("download {}/{}", num+1, result_len)).chat_id(chat_id).message_id(message_id).send())

                .for_each(|_| Ok(()))
        })
        .for_each(|_| Ok(()));

    bot.run_with(search.join(download));
}

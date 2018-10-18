import {h, Component} from 'preact';
import {Icon} from 'preact-mdl';
import { route } from 'preact-router';

import style from './style.less';
import {List} from './list.js';
import Protocol from 'Lib/protocol';

const Mode = {
    Closed: 0,
    Small: 1,
    Full: 2,
};

function matchYoutubeUrl(url) {
    var p = /^(?:https?:\/\/)?(?:m\.|www\.)?(?:youtu\.be\/|youtube\.com\/(?:embed\/|v\/|watch\?v=|watch\?.+&v=))((\w|-){11})(?:\S+)?$/;
    if(url.match(p)){
        return url.match(p)[1];
    }
    return false;
}

export default class Upload extends Component {
    state = {
        show: Mode.Closed,
        link_empty: true
    };

    open = () => {
        if(this.state.show == Mode.Closed)
            this.setState({show: Mode.Small});
        else
            this.setState({show: Mode.Closed});
    }

    close = () => {
        this.setState({show: Mode.Closed});
    }

    upload_link = () => {
        let val = this.input.value;

        Protocol.upload_youtube(val);
    }

    filesDropped = (e) => {
        e.stopPropagation();
        e.preventDefault();

        var files = [];
        for(const entry of e.target.files) {
            let file = "webkitGetAsEntry" in entry ? entry.webkitGetAsEntry() : entry;

            var name;
            switch(file.type) {
                case "audio/mpeg":
                    name = "mp3"; break;
                case "audio/x-wav":
                    name = "wav"; break;
                case "audio/mp4":
                    name = "mp4"; break;
                default:
                    continue;
            }
            files.push([file.name, name, file.slice()]);
        }

        Protocol.upload_files(files)
    }
    input_changed = (e) => {
        let elm = e.target;

        if(elm.value) {
            const url = matchYoutubeUrl(elm.value);
            if(url) {
                elm.classList.add(style.green_border);
                elm.classList.remove(style.red_border);

                if(e.keyCode == 13) {
                    console.log("Start downloading url: " + url);
                    
                    this.upload_link();
                }

            } else {
                elm.classList.add(style.red_border);
                elm.classList.remove(style.green_border);
            }

            this.setState({link_empty: false});
        } else {
            elm.classList.remove(style.green_border);
            elm.classList.remove(style.red_border);
            this.setState({link_empty: true});
        }

    }

    create_playlist = (e) => {
        Protocol.add_playlist("New playlist")
        .then(obj => {
            return Promise.all(this.list.state.tracks.map(x => Protocol.add_to_playlist(obj.key, x.track_key))).then(_ => obj.key);
        })
        .then(key => route('/playlist/' + key));
    }

    render(props, {show, link_empty}) {
        return (
            <div>
                <Icon icon="file upload" onClick={this.open} />

                {show != Mode.Closed && (
                <div class={style.upload}>
                    <List ref={x => this.list = x} />
                    <div class={style.control}>
                        <Icon icon="playlist add" onClick={this.create_playlist} class={style.link_button} />
                        {!link_empty && (
                            <Icon icon="add circle outline" class={style.link_button} onClick={this.upload_link} />
                        )}
                        {link_empty && (
                            <Icon icon="note add" class={style.link_button} onClick={x => this.file_input.click()} />
                        )}
                        <input class={style.link_input} ref={x => this.input = x} onKeyUp={this.input_changed} />
                        <input type="file" style="display: none" webkitdirectory allowdir mozdirectory onChange={this.filesDropped} ref={x => this.file_input = x}/>
                    </div>
                </div>
                )}
            </div>
        );
    }
}

import {Component, h} from 'preact';
import style from './style.less';
import {guid} from '../../lib/uuid.js';
import Protocol from '../../lib/protocol';
import TrackView from './track';

function matchYoutubeUrl(url) {
    var p = /^(?:https?:\/\/)?(?:m\.|www\.)?(?:youtu\.be\/|youtube\.com\/(?:embed\/|v\/|watch\?v=|watch\?.+&v=))((\w|-){11})(?:\S+)?$/;
    if(url.match(p)){
        return url.match(p)[1];
    }
    return false;
}

function get_title(url) {
    fetch("https://youtube.com/get_video_info?video_id=" + encodeURIComponent(url))
        .then(x => x.formData())
        .then(x => { console.log(x); return x; });
}


export default class Youtube extends Component {
    state = {
        uuid: null,
        downloading: false,
        progress: 0.0,
        track: null
    };

    finished(uuid) {
        clearInterval(this.interval);
        Protocol.youtube_finish(uuid)
            .then(track => {
                this.setState({ downloading: false, track: track });

                console.log("KEY: ");
                console.log(track);
            })
    }

    watch_upload(url) {
        const uuid = guid();
        this.setState({ downloading: true, uuid });

        console.log(uuid);

        if(this.interval)
            clearInterval(this.interval);

        let self = this;
        this.interval = setInterval(function() {
            Protocol.youtube_upload(uuid, url).then(x => { 
                if(!x.progress) return;

                if(x.progress == 1.0)
                    self.finished(uuid);
                else
                    self.setState({ progress: x.progress });
            });
        }, 1000);
    }

    change = (e) => {
        let elm = e.target;

        if(elm.value) {
            const url = matchYoutubeUrl(elm.value);
            if(url) {
                elm.classList.add(style.green_border);
                elm.classList.remove(style.red_border);

                if(e.keyCode == 13) {
                    console.log("Start downloading url: " + url);
                    
                    this.watch_upload(url);
                }

            } else {
                elm.classList.add(style.red_border);
                elm.classList.remove(style.green_border);
            }
        } else {
            elm.classList.remove(style.green_border);
            elm.classList.remove(style.red_border);
        }
    }

    render({onClose},{uuid, downloading, progress, track}) {
        if(track) {
            return (<TrackView tracks={track} finished={onClose} />);
        } else return (
            <div class={style.youtube_inner} onClick={e => e.stopPropagation()} >
                <b>Youtube downloader</b>
                <input type="text" ref={x => this.input = x} onKeyUp={this.change} />
            </div>);
    }
}

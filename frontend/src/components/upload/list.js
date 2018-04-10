import {h, Component} from 'preact';
import style from './style.less';
import Protocol from '../../lib/protocol.js';
import {Icon} from 'preact-mdl';

class TrackItem extends Component {
    state = {
        show: false
    };

    format(kind, progress) {
        const p = Math.floor(progress*100);
        if(kind == "converting_opus")
            if(progress == 1.0) 
                return "Finished";
            else
                return p+"% (convert to opus)";
        if(kind == "converting_ffmpeg")
            return p +"% (convert to wav)";
        if(kind == "youtube_download")
            return p + "% (download from youtube)";
    }

    render({idx, desc, kind, progress, track_key}, {show}) {
        return (
            <div class={style.track_item} onClick={x => this.setState({show: false})}>
                <span>{idx}.</span>
                <b>{desc}</b>
                <span class={style.upload_status}>{this.format(kind, progress)}</span>
                <Icon icon="arrow drop down" onClick={x => this.setState({show: true})} />
                {show && (
                    <div class={style.track_desc}>
                    </div>
                )}
            </div>
        );

    }
}

export class List extends Component {
    state = {
        tracks: []
    };

    componentDidMount() {
        let self = this;

        this.interval = setInterval(function() {
            Protocol.ask_upload_progress().then(progress => {
                self.setState({tracks: progress});
            });
        }, 1000);
    }

    componentDidUmount() {
        clearInterval(this.interval);
    }

    render({}, {tracks}) {
        var idx = 1;

        return (
            <div class={style.upload_list}>
                {tracks.length > 0 && tracks.map(x => (
                    <TrackItem idx={idx++} desc="Unbekannt" {...x} />
                ))}
                {tracks.length == 0 && (
                    <div class={style.upload_nothing}>Keine Uploads</div>
                )}
            </div>
        );
    }
}

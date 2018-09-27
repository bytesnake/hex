import {h, Component} from 'preact';
import {Icon, Button} from 'preact-mdl';
import {route} from 'preact-router';
import style from './style.less';
import TrackList from '../tracklist';
import Protocol from '../../lib/protocol.js';
import { guid } from '../../lib/uuid.js'

export default class Playlist extends Component {
    update = (props) => {
        if(this.state.pl_key != props.pl_key) {
            this.setState({pl_key: props.pl_key, updating: true});

            Protocol.get_playlist(props.pl_key).then(x => {
                this.setState({playlist: x, updating: false});
            });
        }
    }

    shouldComponentUpdate() {
		return true;
	}
    componentWillMount() {
        this.update(this.props);
    }

    componentWillReceiveProps(props) {
        this.update(props);
    }

    play = () => {
        window.player.play(this.state.playlist[1].map(x => x.key));
    }

    add_queue = () => {
        window.player.play(this.state.playlist[1].map(x => x.key));
    }

    download = () => {
        if(this.state.downloading != null)
            return;

        let uuid = guid();

        Protocol.download(uuid, 'wav', this.state.playlist[1].map(x => x.key));

        let self = this;
        let dwnd = this.download = setInterval(function() {
            Protocol.ask_download_progress()
                .then(x => {
                    let elm = x.filter(x => x.id == uuid);

                    if(elm[0]) {
                        self.setState({ downloading: Math.round(elm[0].progress * 100)});

                        if(elm[0].progress == 1.0) {
                            clearInterval(dwnd);

                            self.setState({ downloading: null });

                            var file_path = elm[0].download;
                            var a = document.createElement('A');
                            a.href = file_path;
                            a.download = file_path.substr(file_path.lastIndexOf('/') + 1);
                            document.body.appendChild(a);
                            a.click();
                            document.body.removeChild(a);
                            //window.open(elm[0].download);
                            //window.location.assign(elm[0].download);

                        }
                    }

                    console.log(x);
                });
        }, 200);
    }

    update_download = () => {
    }

    delete_pl = () => {
        Protocol.delete_playlist(this.state.playlist[0].key).then(x => {
            route("/");
        });
    }

    change_title = (e) => {
        Protocol.change_playlist_title(this.state.playlist[0].key, e.target.value);
    }

    change_desc = (e) => {
        Protocol.change_playlist_desc(this.state.playlist[0].key, e.target.value);
    }

    render({}, {pl_key, playlist, updating, downloading}) {
        if(!updating && playlist) {
            const header = playlist[0];
            const tracks = playlist[1];

            return (
                <div class={style.playlist}>
                    <div class={style.header}>
                        <Icon icon="queue music" />
                        <div class={style.header_text}>
                            <input type="text" onChange={this.change_title} value={header.title} />
                            <textarea onChange={this.change_desc} value={header.desc} />
                        </div>
                        <div class={style.header_actions}>
                            <Button onClick={this.play}><Icon icon="play circle outline" /></Button>
                            <Button onClick={this.add_queue}><Icon icon="add to queue" /></Button>
                            <Button onClick={this.download}>
                            { (downloading && downloading != 1.0) && (
                                <div style="align-text: right">{downloading}%</div>
                            )}
                            { (!downloading || downloading == 1.0) && (<Icon icon="file download" />)}
                            </Button>
                            <Button onClick={this.delete_pl}><Icon icon="delete forever" /></Button>
                        </div>
                            
                    </div>
                    <div class={style.tracks}>
                        {tracks && tracks.length > 0 && (
                            <TrackList tracks={tracks} />
                        )}
                    </div>
                </div>
            );
        }
        return (
            <div class={style.playlist}>
                Loading ...
            </div>
        );
    }
}

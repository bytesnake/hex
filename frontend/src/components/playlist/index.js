import {h, Component} from 'preact';
import {Icon, Button} from 'preact-mdl';
import {route} from 'preact-router';
import style from './style.less';
import TrackList from '../tracklist';
import Protocol from '../../lib/protocol.js';

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
        alert("DOWNLOAD");
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

    render({}, {pl_key, playlist, updating}) {
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
                            <Button onClick={this.download}><Icon icon="file download" /></Button>
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

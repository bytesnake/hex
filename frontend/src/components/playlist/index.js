import {h, Component} from 'preact';
import style from './style.less';
import TrackList from '../tracklist';
import Protocol from '../../lib/protocol.js';

export default class Playlist extends Component {
    state = {
        playlist: null,
        updating: false,
    };

    update(props) {
        console.log("Update: " + props);

        if(this.state.pl_key != props.pl_key) {
            this.setState({pl_key: props.pl_key, updating: true});

            Protocol.get_playlist(props.pl_key).then(x => {
                this.setState({playlist: x, updating: false});
            });
        }
    }

    componentDidMount() {
        this.update(this.props);
    }

    componentWillReceiveProps(props) {
        this.update(props);
    }

    render({}, {pl_key, playlist, updating}) {
        console.log(playlist);

        if(playlist) {
            const header = playlist[0];
            const tracks = playlist[1];

            return (
                <div class={style.playlist}>
                    <div class={style.header}></div>
                    <div class={style.tracks}>
                        {tracks && tracks.length > 0 && (
                            <TrackList tracks={tracks} />
                        )}
                    </div>
                </div>
            );
        } else {

            return (
                <div class={style.playlist}>
                    Loading ...
                </div>
            );
        }
    }
}

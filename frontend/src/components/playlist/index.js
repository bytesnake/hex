import {h, Component} from 'preact';
import style from './style.less';
import TrackList from '../tracklist';
import Protocol from '../../lib/protocol.js';

export default class Playlist extends Component {
    state = {
        playlist: null,
        updating: false,
        tracks: null
    };

    update(props) {
        console.log("Update: " + props);

        if(this.state.playlist != props.playlist) {
            this.setState({playlist: props.playlist, updating: true});

            Protocol.get_playlist_tracks(playlist.key).then(x => {
                this.setState({tracks: x, updating: false});
            });
        }
    }

    componentDidMount() {
        this.update(this.props);
    }

    componentWillReceiveProps(props) {
        this.update(props);
    }

    render({}, {playlist, updating, tracks}) {
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
    }
}

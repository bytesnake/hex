import {h, Component} from 'preact';
import {Icon} from 'preact-mdl';
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

    render({}, {pl_key, playlist, updating}) {
        if(!updating && playlist) {
            const header = playlist[0];
            const tracks = playlist[1];

            return (
                <div class={style.playlist}>
                    <div class={style.header}>
                        <Icon icon="queue music" />
                        <div class={style.header_text}>
                            <b>{header.title}</b>
                            <i>{header.desc}</i>
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

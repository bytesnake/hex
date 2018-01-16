import { h, Component } from 'preact';
import style from './style.less';
import Protocol from '../../lib/protocol.js';
import { Card } from 'preact-mdl';
import TrackList from '../tracklist';

// TODO advanced search method
// gets called when this route is navigated to
async function search(query) {
    var tracks = [];
    for await (const i of Protocol.search(query)) {
        tracks.push(JSON.parse(JSON.stringify(i)));
    }

    return tracks;
}

export default class Search extends Component {
    update = (props) => {
        if(this.state.query != props.query) {
            this.setState({ query: props.query, updating: true });
            search(props.query).then( tracks => {
                this.setState({ query: props.query, updating: false, tracks });
            });
        }
    }

    componentDidMount() {
        this.update(this.props);
    }

    componentWillReceiveProps(props) {
        this.update(props);
    }

	// Note: `user` comes from the URL, courtesy of our router
	render({}, { query, updating, tracks }) {
        if(tracks && tracks.length > 0) {
            return (
                <TrackList tracks={tracks} />
            );
        } else {
            return (
                <div class={style.search}>Nothing found</div>
            );
        }
	}
}

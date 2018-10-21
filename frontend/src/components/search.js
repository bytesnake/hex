import { h, Component } from 'preact';
import { Card } from 'preact-mdl';
import style from 'Style/search';
import Protocol from 'Lib/protocol';
import TrackList from 'Component/tracklist';

export default class Search extends Component {
    state = {
        updating: false,
        finished: false,
        stream: null,
        query: null
    };

    update = (props) => {
        if(this.state.query != props.query) {
            let stream = Protocol.start_search(props.query);
            this.setState({ stream: stream, query: props.query, tracks: [], updating: false, finished: false }, e => {this.more()})
        }
    }

    more() {
        if(this.state.finished || this.state.updating)
            return;

        this.setState({ updating: true });

        this.state.stream().then(res => {
            let tmp = this.state.tracks.slice();
            tmp.push.apply(tmp, res.answ);

            this.setState({ tracks: tmp, finished: !res.more, updating: false });
        });
    }

    componentDidMount() {
        this.update(this.props);

    }

    componentWillReceiveProps(props) {
        this.update(props);
    }

	// Note: `user` comes from the URL, courtesy of our router
	render({}, { query, updating, stream, tracks }) {
        if(stream && tracks.length > 0) {
            return (
                <div class={style.search}>
                    <TrackList loadMore={this.more.bind(this)} tracks={tracks} />
                </div>
            );
        } else {
            return (
                <div class={style.search}>Nothing found</div>
            );
        }
	}
}

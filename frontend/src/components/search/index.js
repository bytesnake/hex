import { h, Component } from 'preact';
import style from './style.less';
import Protocol from '../../lib/protocol.js';
import { Card } from 'preact-mdl';
import Track from '../track';

// gets called when this route is navigated to
async function search(query) {
    var tracks = [];
    for await (const i of Protocol.search(query)) {
        tracks.push(JSON.parse(JSON.stringify(i)));
    }

    return tracks;
}

export default class Profile extends Component {
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

	// gets called just before navigating away from the route
	componentWillUnmount() {
	}

	// Note: `user` comes from the URL, courtesy of our router
	render({}, { query, updating, tracks }) {
        if(tracks && tracks.length > 0) {
            return (
                <div class={style.search}>
                    <table>
                        <tr>
                            <th>Title</th>
                            <th>Album</th>
                            <th>Interpret</th>
                            <th>Conductor</th>
                            <th>Composer</th>
                        </tr>

                        { tracks.map( x => (
                            <Track minimal {...x} />
                        )) }
                    </table>
                </div>
            );
        } else {
            return (
                <div class={style.search}>Nothing found</div>
            );
        }
	}
}

import {Component, h} from 'preact';
import Track from './track.js';
import style from './style.less';

export default class TrackList extends Component {
    render({tracks},{}) {
        return (
            <div class={style.tracklist}>
            <table>
                <tr>
                    <th>Title</th>
                    <th>Album</th>
                    <th>Interpret</th>
                    <th>Conductor</th>
                    <th>Composer</th>
                </tr>

                { tracks.map( x => (
                    <Track minimal track_key={x.key} {...x} />
                )) }
            </table>
            </div>
        );
        
    }

}

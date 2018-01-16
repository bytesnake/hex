import {h, Component} from 'preact';
import style from './style.less';
import PlayButton from '../play_button';

export default class Track extends Component {
    state = {
        minimal: true
    };

    onClick = (e) => {
        this.setState({ minimal: !this.state.minimal });
    }

    render({track_key, title, album, interpret, conductor, composer}, {minimal}) {
        if(minimal)
            return (
                <tr onClick={this.onClick}>
                    <td>{title}</td>
                    <td>{album}</td>
                    <td>{interpret}</td>
                    <td>{conductor}</td>
                    <td>{composer}</td>
                </tr>
            );
        else
            return (
                <tr onClick={this.onClick}>
                    <td colspan="5">
                        <div class={style.desc}>
                            <table>
                                <tr>
                                    <th>Title</th>
                                    <td>{title}</td>
                                </tr>
                                <tr>
                                    <th>Interpret</th>
                                    <td>{interpret}</td>
                                </tr>
                                <tr>
                                    <th>Album</th>
                                    <td>{album}</td>
                                </tr>
                                <tr>
                                    <th>Conductor</th>
                                    <td>{conductor}</td>
                                </tr>
                                <tr>
                                    <th>Composer</th>
                                    <td>{composer}</td>
                                </tr>
                            </table>
                        </div>
                        <div class={style.playlists}>Playlists</div>
                        <PlayButton track_key={track_key} />
                    </td>
                </tr>
            );
    }
}


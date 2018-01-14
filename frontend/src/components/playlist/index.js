import {h, Component} from 'preact';
import style from './style.less';

export default class Playlist extends Component{
    render({matches}, {}) {
        return (
            <div class={style.playlist}>Key: {matches.key}</div>
        );
    }
}

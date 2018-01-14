import {h, Component} from 'preact';
import {Button, Icon} from 'preact-mdl';
import style from './style.less';

export default class PlayButton extends Component {
    play(e, key) {
        e.stopPropagation();

        window.player.play(key);
    }

    render({track_key, size},{}) {
        return (
            <div class={style.icon}>
                <Button onClick={e => this.play(e, track_key)}><Icon icon="play circle outline" /></Button>
            </div>
        );
    }
}

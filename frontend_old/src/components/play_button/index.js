import {h, Component} from 'preact';
import {Button, Icon} from 'preact-mdl';
import style from './style.less';
export class PlayButton extends Component {
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
export class AddToQueueButton extends Component {
    play(e, key) {
        e.stopPropagation();

        window.player.add_track(key);
    }

    render({track_key, size},{}) {
        return (
            <div class={style.icon}>
                <Button onClick={e => this.play(e, track_key)}><Icon icon="add to queue" /></Button>
            </div>
        );
    }
}


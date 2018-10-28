import {h, Component} from 'preact';
import {Icon} from 'preact-mdl';
import style from './music_queue_style.css';

class QueueItem extends Component {
    remove_track = (e) => {
        this.props.player.remove_track(this.props.idx);

        e.stopPropagation();
    }

    play_track = () => {
        this.props.player.set_queue_pos(this.props.idx);
    }

    render({title, interpret, current, idx}, {}) {
        return (
            <div class={style.item} onClick={this.play_track}>
                <div class={style.desc}>
                    <b>{title}</b> <br />
                    <span>{interpret}</span>
                </div>

                <div class={style.remove_track} onClick={this.remove_track} >
                    <Icon icon={(current?"play circle filled":"remove circle outline")} />
                </div>
            </div>
        );
    }

}

export default class MusicQueue extends Component {
    state = {
        visible: false,
    };
    
    click = () => {
        this.setState({ visible: !this.state.visible });
    }

    shuffle = () => {
        this.props.player.shuffle_below_current();
    }

    clear = () => {
        this.props.player.clear();
    }

    dur_to_string(duration) {
        let min = Math.floor(duration/60);
        let sec =  Math.round(duration) % 60;

        return min + "h " + sec + "m";
    }
    render({player, queue, queue_pos},{visible} ) {
        var idx = 0;
        var duration = 0.0;

        for(const elm in queue) {
            duration += queue[elm].duration;
        }

        return (
            <div class={style.music_queue}>
                <Icon icon="queue music" onClick={this.click}/>
                {visible && (
                    <div class={style.inner} tabindex="0">
                    <div class={style.inner_wrapper}>
                        <div class={style.inner_content}>
                        { queue.length == 0 && (
                            <b>Liste ist leer</b> 
                        )}

                        { queue.map(x => {
                            idx ++;
                            return ( <QueueItem player={player} track_key={x.key} title={x.title} interpret={x.interpret} current={idx-1 == queue_pos} idx={idx-1}/> );
                        })}
                        </div>
                   </div>
                        <div class={style.inner_actions}>
                            <div class={style.inner_actions_icons}>
                                <Icon icon="shuffle" onClick={this.shuffle} />
                                <Icon icon="delete sweep" onClick={this.clear} />
                            </div>
                            <b>{queue_pos+1}/{queue.length} Lieder</b>
                            <b>{this.dur_to_string(duration)}</b>
                        </div>
                        <div class={style.arrow_down} />
                   </div>
                )}
            </div>
        );
    }
}


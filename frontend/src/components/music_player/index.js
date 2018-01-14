import {h, Component} from 'preact';
import {Icon, Button} from 'preact-mdl';
import Player from '../../lib/player.js';
import sbottom from './style_bottom.less';

const BUFFER_SIZE = 8192*2;
const BUFFER_FILL = 4;

export default class MusicPlayer extends Component {
    state = {
        is_playing: false,
        track: null
    };
    
    componentWillMount() {
        this.player = new Player(2);

        setInterval(this.update_time.bind(this), 300);
    }

    play(key) {
        this.player.clear();
        this.player.add_track(key).then(x => {
            this.player.play();

            this.setState({is_playing: true, track: x});
        });
    }

    stop() {
        this.player.stop();
    }

    play_click(e) {
        if(this.state.is_playing) {
            this.player.stop();
            this.setState({is_playing: false });
        } else {
            this.player.play();
            this.setState({is_playing: true });
        }
    }

    update_time() {
        if(this.timer == null)
            return;

        let inner = this.timer.children[0];
        let knob = inner.children[0];

        const time = this.player.time_percentage();

        inner.style.width = time * this.timer.offsetWidth + "px";
        knob.style.left = time * this.timer.offsetWidth + "px";
    }

    render({}, {is_playing, track}) {
        let play_pause = null;
        if(!is_playing)
            play_pause = <Icon style="font-size: 4vw;" icon="play circle outline" onClick={this.play_click.bind(this)} onMouseOver={e => e.target.innerHTML = "play_circle_filled"} onMouseLeave={e => e.target.innerHTML = "play_circle_outline"} />;
        else
            play_pause = <Icon style="font-size: 4vw;" icon="pause circle outline" onClick={this.play_click.bind(this)} onMouseOver={e => e.target.innerHTML = "pause_circle_filled"} onMouseLeave={e => e.target.innerHTML = "pause_circle_outline"} />;

        let track_name = "Unbekannt";
        if(track && track.title)
            track_name = track.title;

        return (
            <div class={sbottom.outer}>
            <div class={sbottom.music_player}>
                <div class={sbottom.progress_bar} ref={x => this.timer = x}><div class={sbottom.progress_bar_inner}><div class={sbottom.round_button} /></div></div>
                <div class={sbottom.music_player_inner}>
                    <div class={sbottom.music_player_left}>{track_name}</div>
                    <div class={sbottom.music_player_center}>
                        <Icon style="font-size: 2.5vw;" icon="skip previous" onClick={this.player.next} />
                        {play_pause}
                        <Icon style="font-size: 2.5vw;" icon="skip next" onClick={this.player.prev}/>
                    </div>
                    <div class={sbottom.music_player_right}>
                        {track != undefined &&
                            <div>
                                {track.duration}
                            </div>
                        }
                        <Icon style="font-size: 30px; " icon="queue music" />
                    </div>
                </div>
            </div>
            </div>
        );
    }
}

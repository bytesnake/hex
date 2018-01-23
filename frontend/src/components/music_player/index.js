import {h, Component} from 'preact';
import {Icon, Button} from 'preact-mdl';
import Player from '../../lib/player.js';
import get_album_cover from '../../lib/get_cover.js';
import sbottom from './style_bottom.less';

const BUFFER_SIZE = 8192*2;
const BUFFER_FILL = 4;

export default class MusicPlayer extends Component {
    state = {
        is_playing: false,
        track: null
    };
    
    componentWillMount() {
        this.player = new Player(2, this.new_track, (x) => this.setState({ is_playing: x }));

        this.update = setInterval(this.update_time.bind(this), 300);
    }

    componentWillUmount() {
        clearInterval(this.update);

        this.player.stop();
        this.player.clear();

        console.log("UMOUNT");
    }

    new_track = (track) => {
        this.setState({ track });
        get_album_cover(track.interpret, track.album).then(cover => {
            this.setState({ cover });
        }, err => {
            console.error("Could not load cover: " + err);
            this.setState({ cover: null});
        });
    }

    play(key) {
        this.player.clear();
        this.player.add_track(key).then(x => {
            this.player.play();

            this.setState({is_playing: true});
        });
    }

    add_track(key) {
        this.player.add_track(key).then(x => {
        });
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

    dur_to_string(duration) {
        let min = Math.floor(duration/60);
        let sec =  Math.round(duration) % 60;

        return min + ":" + sec;
    }

    show_lyrics() {
        let artist = this.state.track.interpret.replace(/ /g,'').toLowerCase();
        let title = this.state.track.title.replace(/ /g,'').toLowerCase();

        window.open('https://www.azlyrics.com/lyrics/'+artist+'/'+title+'.html', '_blank');
    }

    render({}, {is_playing, track, cover}) {
        let play_pause = null;
        if(!is_playing)
            play_pause = <Icon style="font-size: 5em;" icon="play circle outline" onClick={this.play_click.bind(this)} onMouseOver={e => e.target.innerHTML = "play_circle_filled"} onMouseLeave={e => e.target.innerHTML = "play_circle_outline"} />;
        else
            play_pause = <Icon style="font-size: 5em;" icon="pause circle outline" onClick={this.play_click.bind(this)} onMouseOver={e => e.target.innerHTML = "pause_circle_filled"} onMouseLeave={e => e.target.innerHTML = "pause_circle_outline"} />;

        return (
            <div class={sbottom.outer}>
            <div class={sbottom.music_player}>
                <div class={sbottom.progress_bar} ref={x => this.timer = x}><div class={sbottom.progress_bar_inner}><div class={sbottom.round_button} /></div></div>
                <div class={sbottom.music_player_inner}>
                    <div class={sbottom.music_player_left}>
                        {cover && (
                            <img src={cover} />
                        )}
                        {!cover && (
                            <Icon style="font-size: 5em" icon="art track" />
                        )}
                        {track && (
                            <span>
                                <b>{track.title?track.title:"Unbekannt"}</b>
                                {track.interpret}
                            </span>
                        )}
                    </div>
                    <div class={sbottom.music_player_center}>
                        <Icon style="font-size: 3em;" icon="skip previous" onClick={this.player.prev} />
                        {play_pause}
                        <Icon style="font-size: 3em;" icon="skip next" onClick={this.player.next}/>
                    </div>
                    <div class={sbottom.music_player_right}>
                        {track &&
                            <div>
                                {this.dur_to_string(track.duration)}
                            </div>
                        }
                        <div>
                            {track && (
                                <Icon onClick={this.show_lyrics.bind(this)} style="font-size: 40px;" icon="textsms" />
                            )}

                            <Icon style="font-size: 40px;" icon="queue music" />
                        </div>
                    </div>
                </div>
            </div>
            </div>
        );
    }
}

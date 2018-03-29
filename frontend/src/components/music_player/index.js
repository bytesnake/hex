import {h, Component} from 'preact';
import {Icon, Button} from 'preact-mdl';
import Player from '../../lib/player.js';
import get_album_cover from '../../lib/get_cover.js';
import sbottom from './style_bottom.less';
import ProgressBar from './progress.js';
import MusicQueue from './music_queue.js';

export default class MusicPlayer extends Component {
    state = {
        is_playing: false,
        track: null,
        queue: [],
        queue_pos: 0
    };
    
    componentWillMount() {
        this.player = new Player(2, this.new_track, (x) => this.setState({ is_playing: x }), (x) => this.setState({ queue: x }), (x) => this.setState({queue_pos: x}));
    }

    componentWillUmount() {
        this.player.stop();
        this.player.clear();
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
        this.player.add_track(key);
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

    render({}, {is_playing, track, cover, queue, queue_pos}) {
        let play_pause = null;
        if(!is_playing)
            play_pause = <Icon style="font-size: 5em;" icon="play circle outline" onClick={this.play_click.bind(this)} onMouseOver={e => e.target.innerHTML = "play_circle_filled"} onMouseLeave={e => e.target.innerHTML = "play_circle_outline"} />;
        else
            play_pause = <Icon style="font-size: 5em;" icon="pause circle outline" onClick={this.play_click.bind(this)} onMouseOver={e => e.target.innerHTML = "pause_circle_filled"} onMouseLeave={e => e.target.innerHTML = "pause_circle_outline"} />;

        return (
            <div class={sbottom.outer}>
            <div class={sbottom.music_player}>
                <ProgressBar player={this.player} />
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
                        <div class={sbottom.music_player_actions} >
                            {track && (
                                <Icon onClick={this.show_lyrics.bind(this)} style="font-size: 40px;" icon="textsms" />
                            )}

                            <MusicQueue player={this.player} queue={queue} queue_pos={queue_pos} />
                        </div>
                    </div>
                </div>
            </div>
            </div>
        );
    }
}

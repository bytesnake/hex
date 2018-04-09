import {h, Component} from 'preact';

class TrackItem extends Component {
    state = {
        show: false
    };

    render({duration, progress, track_key}) {
        return (
            <div class={style.track_item} onClick={this.setState({show: false})}>
                <b>{progress}/100</b>
                <span>{duration}</b>
                <Icon icon="arrow" onClick={this.setState({show: true})} />
                {show && (
                    <div class={style.track_desc}>
                    </div>
                }
            </div>
        );

    }
}

export default class List extends Component {
    state = {
        tracks: []
    };

    componentDidMount() {
        this.interval = setInterval(function() {
            Protocol.ask_upload_progress(function(progress) {
                this.setState({tracks: progress});
            });
        }, 300);
    }

    componentDidUmount() {
        clearInterval(this.interval);
    }

    render({}, {tracks}) {
        return (
            <div class={style.upload_list}>
                {track.length > 0 && tracks.map(x => (
                    <TrackItem {...x} />
                ))}
            </div>
        );
    }
}

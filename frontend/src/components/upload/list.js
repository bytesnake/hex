import {h, Component} from 'preact';
import {Icon, Button} from 'preact-mdl';
import Autocomplete from 'accessible-autocomplete/preact'

import style from './style.css';
import Protocol from 'Lib/protocol';
import suggestion_flatten from 'Lib/suggestions';
import Spinner from 'Component/spinner';

export default class TrackMeta extends Component {
    constructor(props) {
        super();

        const track_key = props.track_key;

        let track = {
            key: track_key,
            title: null,
            album: null,
            interpret: null,
            people: null,
            composer: null
        };

        Protocol.get_suggestion(track_key)
        .then(suggestions => {
            console.log("Suggestion: " + suggestions);
            if(suggestions && suggestions[0] && suggestions[0].data)
                var suggestion = suggestion_flatten(JSON.parse(suggestions[0].data));

            this.setState({ track, suggestion });
        }).catch(e => {
            console.error("Could not get suggestions: " + e);

            this.setState({ track });
        });
    }

    suggest = (query, cb, kind) => {
        if(!this.state.suggestion)
            return;

        const filteredResults = this.state.suggestion[kind].filter(result => result.indexOf(query) !== -1);

        cb(filteredResults)
    }


    save = () => {
        const track = this.state.track;

        let self = this;
        Protocol.update_track(track.key, track.title, track.album, track.interpret, track.people, track.composer)
        .then(function() {
            console.log("Updated");
        });
        
    }

    update = (value, kind) => {
        const track = this.state.track;

        track[kind] = value;

        this.setState({ track });

    }
    
    render({},{track}) {
        if(!track) return (<div />);

        return (
            <div class={style.files_data} onClick={e => e.stopPropagation()}>
                <b>Title</b>
                <Autocomplete id='autocomplete_title' source={(a,b) => this.suggest(a, b, "title")} showAllValues={true} 
                    onConfirm={(x) => this.update(document.getElementById("autocomplete_title").value, "title")} displayMenu='overlay' />
                <b>Album</b>
                <Autocomplete id='autocomplete_album' source={(a,b) => this.suggest(a, b, "album")} showAllValues={true}
                    onConfirm={() => this.update(document.getElementById("autocomplete_album").value, "album")} displayMenu='overlay'/>
                <b>Interpret</b>
                <Autocomplete id='autocomplete_interpret' source={(a,b) => this.suggest(a, b, "artist")} showAllValues={true}
                    onConfirm={() => this.update(document.getElementById("autocomplete_interpret").value, "interpret")} displayMenu='overlay' />
                <b>Composer</b>
                <Autocomplete id='autocomplete_composer' source={(a,b) => this.suggest(a, b, "artist")} showAllValues={true}
                    onConfirm={() => this.update(document.getElementById("autocomplete_composer").value, "composer")} displayMenu='overlay' />

                <Button onclick={this.save}>Save</Button>
            </div>
        );
    }
}

class TrackItem extends Component {
    state = {
        show: false
    };

    format(kind, progress) {
        const p = Math.floor(progress*100);
        if(kind == "converting_opus")
            return p+"% (convert to opus)";
        if(kind == "converting_ffmpeg")
            return p +"% (convert to wav)";
        if(kind == "youtube_download")
            return p + "% (download from youtube)";
        if(kind == "finished")
            return "Finished";
    }

    render({idx, desc, kind, progress, track_key}, {show, track}) {
        return (
            <div class={style.track_item}>
                <div class={style.track_header}>
                    <span>{idx}.</span>
                    <b>{desc}</b>
                    <span class={style.upload_status}>{this.format(kind, progress)}</span>
                    {!track_key && (
                        <Spinner size="40px" style="margin: 5px;"/>
                    )}
                    {track_key && (
                        <Icon icon="arrow drop down" onClick={x => this.setState({show: !this.state.show})} />
                    )}
                </div>
                {show && (
                     <div class={style.track_meta}>
                        <TrackMeta track_key={track_key} />
                    </div>
                )}
            </div>
        );

    }
}

export class List extends Component {
    state = {
        tracks: []
    };

    componentDidMount() {
        let self = this;

        this.interval = setInterval(function() {
            Protocol.ask_upload_progress().then(progress => {
                let tracks = progress;
                for(const track of self.state.tracks) {
                    if(tracks.filter(x => JSON.stringify(x.id) == JSON.stringify(track.id)).length == 0)
                        tracks.push(track);
                }
                self.setState({ tracks });
            });
        }, 1000);
    }

    componentWillUnmount() {
        this.setState({tracks: []});
        clearInterval(this.interval);
    }

    render({}, {tracks}) {
        var idx = 1;

        return (
            <div class={style.upload_list}>
                {tracks.length > 0 && tracks.map(x => (
                    <TrackItem idx={idx++} track_key={x.key} {...x} />
                ))}
                {tracks.length == 0 && (
                    <div class={style.upload_nothing}>Keine Uploads</div>
                )}
            </div>
        );
    }
}

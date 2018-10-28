import {Component, h} from 'preact';
import Autocomplete from 'accessible-autocomplete/preact'
import {Button} from 'preact-mdl';

import style from './style.css';
import Protocol from 'Lib/protocol';
import suggestion_flatten from 'Lib/suggestions';

export default class TrackView extends Component {
    constructor(props) {
        super();

        // wrap in array if only one object
        let tracks = props.tracks;
        if(tracks.constructor != Array)
            tracks = [tracks];

        Protocol.get_suggestions(tracks.map(x => x.key))
        .then(suggestions => {
            console.log(suggestions);

            var suggestions = suggestions.map(suggestion => {
                var res;
                if(suggestion && suggestion.data)
                    res = suggestion_flatten(JSON.parse(suggestion.data));
                else
                    res = {};

                return res;
            });

            console.log("Tracks");
            console.log(suggestions);

            this.setState({ tracks: tracks, suggestions, selected: 1 });
        });
    }

    suggest = (query, cb, kind) => {
        if(!this.state.suggestions)
            return;

        const filteredResults = this.state.suggestions[this.state.selected-1][kind].filter(result => result.indexOf(query) !== -1);

        cb(filteredResults)

    }


    next = () => {
        const id = this.state.selected;
        const track = this.state.tracks[id-1];

        console.log(id);
        console.log(track);

        let self = this;
        Protocol.update_track(track)
        .then(function() {
            console.log("ID: " + id);
            console.log("Length: " + self.state.tracks.length);

            if(id == self.state.tracks.length) {
                if(self.props.finished)
                    self.props.finished();
            } else
                self.setState({ selected: id+1 });
        });
        
    }

    update = (value, kind) => {
        const id = this.state.selected;
        const tracks = this.state.tracks;

        tracks[id-1][kind] = value;

        this.setState({ tracks });

    }
    
    render({},{tracks, selected}) {
        if(!tracks) return (<div />);

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
                <b>People</b>
                <Autocomplete id='autocomplete_people' source={(a,b) => this.suggest(a, b, "artist")} showAllValues={true}
                    onConfirm={() => this.update(document.getElementById("autocomplete_people").value, "People")} displayMenu='overlay' />
                <b>Composer</b>
                <Autocomplete id='autocomplete_composer' source={(a,b) => this.suggest(a, b, "artist")} showAllValues={true}
                    onConfirm={() => this.update(document.getElementById("autocomplete_composer").value, "composer")} displayMenu='overlay' />

                <Button onclick={this.next}>Next</Button>
                <div class={style.selected}>{selected}/{tracks.length}</div>
            </div>
        );
    }
}


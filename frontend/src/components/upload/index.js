import { h, Component } from 'preact';
import style from './style.less';
import { Button, Icon } from 'preact-mdl';
import MozillaFolder from '../../lib/upload';
import Protocol from '../../lib/protocol';
import Autocomplete from 'accessible-autocomplete/preact'
import suggestion_flatten from '../../lib/suggestions';

export default class Upload extends Component {
    constructor() {
        super();

    }

    state = {
        show: false,
        drag: false,
        uploading: false,
        upload_done: true
    };

    handleFab = () => {
        this.setState({ show: true });
    }

    handleClick = (e) => {
        this.setState({ show: false, keys: null, suggestions: null });
    }

    dragHandler = () => {
        this.setState({ drag: true });
    }

    suggest = (query, cb, kind) => {
        const filteredResults = this.state.suggestions[this.state.selected-1][kind].filter(result => result.indexOf(query) !== -1);

        cb(filteredResults)

    }

    next = () => {
        let id = this.state.selected;
        let track = this.state.keys[id-1];
        let self = this;

        Protocol.update_track(track)
        .then(function() {
            if(id == self.state.keys.length)
                self.setState({ drag: false, show: false, keys: null, suggestions: null });
            else
                self.setState({ selected: id+1 });
        });
        
    }

    update = (value, kind) => {
        let id = this.state.selected;
        var tracks = this.state.keys;
        tracks[id-1][kind] = value;

        this.setState({ keys: tracks });

    }

    filesDropped = (e) => {
        e.stopPropagation();
        e.preventDefault();

        this.setState({ show: true, drag: false, uploading: true, upload_done: false });

        var files = [];
        for(const entry of e.target.files) {
            let file = "webkitGetAsEntry" in entry ? entry.webkitGetAsEntry() : entry;

            var name;
            switch(file.type) {
                case "audio/mpeg":
                    name = "mp3"; break;
                case "audio/x-wav":
                    name = "wav"; break;
                case "audio/mp4":
                    name = "mp4"; break;
                default:
                    continue;
            }
            files.push([name, file.slice()]);
        }

        Protocol.upload_files(files)
        .then(keys => {
            keys = keys.map(x => x.key);
            this.setState({ keys: keys.map(x => {
                return {
                    "update_key": x,
                    "title": null,
                    "interpret": null,
                    "conductor": null,
                    "composer": null
                };
            }), upload_done: true, uploading: true });

            return Protocol.get_suggestions(keys);
        })
        .then(x => new Promise((resolve) => setTimeout(resolve(x), 1500)))
        .then(suggestions => {
            var sugg_flattened = suggestions.map(suggestion => {
                var res;
                if(suggestion && suggestion.data && suggestion.data.Ok)
                    res = suggestion_flatten(JSON.parse(suggestion.data.Ok));
                else
                    res = {};

                return res;
            });

            this.setState({suggestions: sugg_flattened, selected: 1, uploading: false });
        });
    }

    render({},{ show, drag, keys, suggestions, selected, uploading, upload_done }) {
        if(show) {
            if(uploading && !upload_done)
                return (
                    <div class={style.upload}>
                        <div class={style.upload_data}><p><Icon icon="cloud upload" /> Uploading ...</p></div>
                    </div>
                );
            else if(uploading && upload_done) 
                return (
                    <div class={style.upload}>
                        <div class={style.upload_data}><p><Icon icon="cloud done" /> Upload done</p></div>
                    </div>
                );

            else if(keys && suggestions) {
                return (
                    <div class={style.upload}>
                        <div class={style.upload_data} onClick={e => e.stopPropagation()}>
                            <b>Title</b>
                            <Autocomplete id='autocomplete_title' source={(a,b) => this.suggest(a, b, "title")} showAllValues={true} 
                                onConfirm={(x) => this.update(document.getElementById("autocomplete_title").value, "title")} />
                            <b>Album</b>
                            <Autocomplete id='autocomplete_album' source={(a,b) => this.suggest(a, b, "album")} showAllValues={true}
                                onConfirm={() => this.update(document.getElementById("autocomplete_album").value, "album")} />
                            <b>Interpret</b>
                            <Autocomplete id='autocomplete_interpret' source={(a,b) => this.suggest(a, b, "artist")} showAllValues={true}
                                onConfirm={() => this.update(document.getElementById("autocomplete_interpret").value, "interpret")} />
                            <b>Conductor</b>
                            <Autocomplete id='autocomplete_conductor' source={(a,b) => this.suggest(a, b, "artist")} showAllValues={true}
                                onConfirm={() => this.update(document.getElementById("autocomplete_conductor").value, "conductor")} />
                            <b>Composer</b>
                            <Autocomplete id='autocomplete_composer' source={(a,b) => this.suggest(a, b, "artist")} showAllValues={true}
                                onConfirm={() => this.update(document.getElementById("autocomplete_composer").value, "composer")} />

                            <Button onclick={this.next}>Next</Button>
                            <div class={style.selected}>{selected}/{keys.length}</div>
                        </div>
                    </div>
                );
            } else
                return (
                <div class={style.upload} id="upload" onClick={this.handleClick}>
                    <div class={style.upload_inner}>
                    <label ondragover={this.dragHandler} onchange={this.filesDropped} ondrop={this.filesDropped}>
                        <input id="file" type="file" directory allowdirs webkitdirectory />
                    </label>
                    <div class={style.drop}>
                        <b>Drop your file/folder here!</b>
                    </div>
                    </div>
                </div>
            );
        } else
            return (
                    <Button id="fab" fab colored onClick={this.handleFab}>
                        <Icon icon="create" />
                    </Button>
            );

                    
    }
}

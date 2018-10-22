import {h, Component} from 'preact';
import {Button, Icon} from 'preact-mdl';
import {route} from 'preact-router';

import style from './style.less';
import {PlayButton, AddToQueueButton} from 'Component/play_button';
import Protocol from 'Lib/protocol';
import InputSuggest from 'Component/input_suggest';
import { guid } from 'Lib/uuid'

const Size = {
    FULL: 0,
    OMIT_COMP_COND: 1,
    ONLY_TITLE: 2
};

class Element extends Component {
    state = {
        edit: false,
        value: (this.props.value?this.props.value:"Unbekannt")
    };

    keypress = (e) => {
        if(e.keyCode === 13) {
            this.blur(e);
        }

        e.target.style.width = ((e.target.value.length)) + 'ch';
    }
    blur = (e) => {
        if(this.state.value != this.input.value) {
            let vals = {};
            vals[this.props.kind] = this.input.value;
            vals['key'] = this.props.track_key;

            Protocol.update_track(vals);
        }

        this.setState({edit: false, value: this.input.value});
    }

    click = (e) => {
        this.setState({edit: true});

        e.stopPropagation();
    }

    componentWillReceiveProps(newProps) {
        if(newProps.value != this.props.value)
            this.setState({ value: newProps.value });
    }

    render({track_key, kind, vertical},{edit, value}) {
        if(!vertical)
            if(edit) return (
            <td><input value={value} onClick={e => e.stopPropagation()} onKeyPress={this.keypress.bind(this)} ref={x => {this.input = x;}} onBlur={this.blur.bind(this)} autoFocus /></td>
            );
            else return (
                <td><span onClick={this.click}>{value}</span></td>
            );
        else
            if(edit) return(<div class={style.element_vertical}><b>{kind.toUpperCase()}</b><input value={value} onClick={e => e.stopPropagation()} onKeyPress={this.keypress} ref={x => {this.input = x;}} onBlub={this.blur} /></div>);
            else return (
                <div class={style.element_vertical} onClick={this.click}><b>{kind.toUpperCase()}</b>{value}</div>
            );
    }
}

export default class Track extends Component {
    state = {
        minimal: true,
        hide: false,
        playlists: null,
        suggestions: null,
        downloading: null
    };

    onClick = (e) => {
        if(this.state.minimal)
            this.update_playlists();

        this.setState({ minimal: !this.state.minimal });
    }

    update_playlists() {
        let pl_of_track = Protocol.get_playlists_of_track(this.props.track_key);
        let all_playlists = Protocol.get_playlists();
        
        Promise.all([pl_of_track, all_playlists]).then(values => {
            this.setState({ playlists: values[0], suggestions: values[1].map(x => x.title) });
        });
    }

    delete_forever = (e) => {
        Protocol.delete_track(this.props.track_key).then(x => {
            this.setState({ hide: true });
        });
    }

    upvote = (e) => {
        Protocol.upvote_track(this.props.track_key);

        e.stopPropagation();
    }

    download = (e) => {
        if(this.state.downloading != null)
            return;

        let self = this;
        Protocol.download('mp3', [this.props.track_key]).then(answ => {
            let dwnd = this.download = setInterval(function() {
                Protocol.ask_download_progress()
                    .then(x => {
                        let elm = x.filter(x => x.id == answ.packet_id);

                        if(elm[0]) {
                            self.setState({ downloading: Math.round(elm[0].progress * 100)});

                            if(elm[0].progress == 1.0) {
                                clearInterval(dwnd);

                                self.setState({ downloading: null });

                                window.open(elm[0].download);
                            }
                        }
                    });
            }, 1000);
        });

        e.stopPropagation();
    }

    open_web = (e) => {
        let query = this.props.title + " " + this.props.interpret;
        query = query.replace(/ /g, '+');

        window.open('https://www.discogs.com/search/?q=' + query + '&type=all', '_blank');
    }

    suggest = (query) => {
        if(!this.state.suggestions)
            return [];

        const suggestions = this.state.suggestions.filter(x => x.indexOf(query) === 0).filter(x => !this.state.playlists.some(y => y.title === x));

        return suggestions;
    }

    addToPlaylist = (playlist) => {
        if(playlist && !this.state.playlists.map(x => x.title).includes(playlist)) {
            if(this.state.suggestions.includes(playlist))
                Protocol.add_to_playlist(this.props.track_key, playlist).then(x => {
                    let playlists = this.state.playlists;
                    playlists.push(x);

                    this.setState({ playlists: playlists });
                });
            else {
                Protocol.add_playlist(playlist).then(x => Protocol.add_to_playlist(this.props.track_key, playlist)).then(x => {
                    let playlists = this.state.playlists;
                    playlists.push(x);

                    this.setState({ playlists: playlists });
                });
            }
        }
    }

    deleteFromPlaylist = (e) => {
        e.stopPropagation();

        const name = e.target.parentNode.firstChild.data;
        const key = this.state.playlists.filter(x => x.title == name).map(x => x.key);

        if(key.length == 0) {
            console.error("Could not find playlist in playlists!");
            return;
        }

        console.log("DON!");
        Protocol.delete_from_playlist(this.props.track_key, key[0])
        .then(x => {
            const playlists = this.state.playlists.filter(x => x.key != key[0]);

            this.setState({ playlists });
        });
    }

    render({size, track_key, title, album, interpret, people, composer}, {minimal, hide, playlists, suggestions, downloading}) {
        if(hide)
            return;

        // parse the people field to an array
        let people_arr = [];
        if(people) people_arr = people.split(',')
            .map(x => x.trim())
            .map(x => x.split(':'))
            .filter(x => x.length == 2)
            .map(x => { return {role: x[0], name: x[1]}; });

        if(minimal)
            return (
                <tr onClick={this.onClick}>
                    <Element track_key={track_key} kind="title" value={title} />
                    {size != Size.ONLY_TITLE && (<Element track_key={track_key} kind="album" value={album} />)}
                    {size != Size.ONLY_TITLE && (<Element track_key={track_key} kind="interpret" value={interpret} />)}
                    {size == Size.FULL && (<Element track_key={track_key} kind="people" value={people_arr.map(x => x.name).join(', ')} />)}
                    {size == Size.FULL && (<Element track_key={track_key} kind="composer" value={composer} />)}
                </tr>
            );
        else
            return (
                <tr onClick={this.onClick}>
                    <td colspan="5">
                        <div class={style.desc}>
                            <div class={style.desc_ctr}>
                                <PlayButton track_key={track_key} />
                                <AddToQueueButton track_key={track_key} />
                                <Button onClick={this.upvote}><Icon icon="insert emoticon" /></Button>
                                <Button onClick={this.download}>
                                { downloading && (
                                    <div style="align-text: right">{downloading}%</div>
                                )}
                                { !downloading && (<Icon icon="file download" />)}
                                </Button>
                                <Button onClick={this.open_web}><Icon icon="language" /></Button>
                                <Button onClick={this.delete_forever} style="flex-grow: 1"><Icon icon="delete forever" /></Button>
                            </div>
                            <div class={style.desc_content}>
                                <Element vertical track_key={track_key} kind="title" value={title} />
                                <Element vertical track_key={track_key} kind="album" value={album} />
                                <Element vertical track_key={track_key} kind="interpret" value={interpret} />
                                <Element vertical track_key={track_key} kind="people" value={people} />
                                <Element vertical track_key={track_key} kind="composer" value={composer} />
                            </div>
                            <div class={style.playlists}><b>Playlists</b><div class={style.playlist_inner}>
                                { playlists && playlists.length > 0 && playlists.map(x => (
                                    <span onClick={e => {e.stopPropagation(); route("/playlist/" + x.key);}} >{x.title}<Icon icon="close" onClick={this.deleteFromPlaylist} /></span>
                                ))}
                            </div>
                            <div class={style.playlist_add}>
                                <div onClick={e => e.stopPropagation()}><InputSuggest onEnter={this.addToPlaylist} suggest={this.suggest} /></div>
                                </div>
                            </div>
                        </div>
                    </td>
                </tr>
            );
    }
}


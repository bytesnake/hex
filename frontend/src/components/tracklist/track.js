import {h, Component} from 'preact';
import {Button, Icon} from 'preact-mdl';
import {route} from 'preact-router';
import style from './style.less';
import {PlayButton, AddToQueueButton} from '../play_button';
import Protocol from '../../lib/protocol.js';
import { InlineSuggest } from 'react-inline-suggest';

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
        suggestions: null
    };

    onClick = (e) => {
        let pl_of_track = Protocol.get_playlists_of_track(this.props.track_key);
        let all_playlists = Protocol.get_playlists();
        
        Promise.all([pl_of_track, all_playlists]).then(values => {
            this.setState({ playlists: values[0], suggestions: values[1].map(x => x.title) });
        });

        this.setState({ minimal: !this.state.minimal });
    }

    delete_forever = (e) => {
        Protocol.delete_track(this.props.track_key).then(x => {
            this.setState({ hide: true });
        });
    }

    suggest = (query) => {
        if(!this.state.suggestions)
            return;

        const suggestions = this.state.suggestions.filter(x => !this.state.playlists.some(y => y.title === x));

        if(this.state.playlists.some(y => y.title === query)) {
            return;
        } else
            return query;
    }

    addToPlaylist = (playlist) => {
        if(playlist && this.state.playlists.indexOf(playlist) === -1) {
            console.log(playlist);
            console.log(this.state.suggestions);

            if(playlist in this.state.suggestions)
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

    render({size, track_key, title, album, interpret, conductor, composer}, {minimal, hide, playlists, suggestions}) {
        if(hide)
            return;

        if(minimal)
            return (
                <tr onClick={this.onClick}>
                    <Element track_key={track_key} kind="title" value={title} />
                    {size != Size.ONLY_TITLE && (<Element track_key={track_key} kind="album" value={album} />)}
                    {size != Size.ONLY_TITLE && (<Element track_key={track_key} kind="interpret" value={interpret} />)}
                    {size == Size.FULL && (<Element track_key={track_key} kind="conductor" value={conductor} />)}
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
                                <Button onClick={this.delete_forever}><Icon icon="delete forever" /></Button>
                            </div>
                            <div class={style.desc_content}>
                                <Element vertical track_key={track_key} kind="title" value={title} />
                                <Element vertical track_key={track_key} kind="album" value={album} />
                                <Element vertical track_key={track_key} kind="interpret" value={interpret} />
                                <Element vertical track_key={track_key} kind="conductor" value={conductor} />
                                <Element vertical track_key={track_key} kind="composer" value={composer} />
                            </div>
                            <div class={style.playlists}><b>Playlists</b><div class={style.playlist_inner}>
                                { playlists && playlists.length > 0 && playlists.map(x => (
                                    <span onClick={e => {e.stopPropagation(); route("/playlist/" + x.key);}} >{x.title}</span>
                                ))}
                            </div>
                            <div class={style.playlist_add}>
                                <div onClick={e => e.stopPropagation()}><InlineSuggest haystack={playlists} getFn={this.suggest}/></div>
                                </div>
                            </div>
                        </div>
                    </td>
                </tr>
            );
    }
}


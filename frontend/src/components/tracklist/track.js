import {h, Component} from 'preact';
import style from './style.less';
import PlayButton from '../play_button';
import Protocol from '../../lib/protocol.js';

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

    keypress(e) {
        if(e.keyCode === 13) {
            this.blur(e);
        }
    }
    blur(e) {
        if(this.state.value != this.input.value) {
            let vals = {};
            vals[this.props.kind] = this.input.value;
            vals['key'] = this.props.track_key;

            Protocol.update_track(vals);
        }

        this.setState({edit: false, value: this.input.value});
    }

    click(e) {
        this.setState({edit: true});

        e.stopPropagation();
    }

    componentWillReceiveProps(newProps) {
        if(newProps.value != this.props.value)
            this.setState({ value: newProps.value });
    }

    render({track_key, kind},{edit, value}) {
        if(edit) return (
            <td style="border: #000 1px solid;"><input value={value} onClick={e => e.stopPropagation()} onKeyPress={this.keypress.bind(this)} ref={x => {this.input = x;}} onBlur={this.blur.bind(this)} autoFocus /></td>
        );
        else return (
            <td><span onClick={this.click.bind(this)}>{value}</span></td>
        );
    }
}

export default class Track extends Component {
    state = {
        minimal: true,
    };

    onClick = (e) => {
        Protocol.get_playlists_of_track(this.props.track_key).then(x => {
            console.log("Playlists: " + x);
        });
        this.setState({ minimal: !this.state.minimal });
    }

    render({size, track_key, title, album, interpret, conductor, composer}, {minimal}) {
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
                            <table>
                                <tr>
                                    <th>Title</th>
                                    <Element track_key={track_key} kind="title" value={title} />
                                </tr>
                                <tr>
                                    <th>Album</th>
                                    <Element track_key={track_key} kind="album" value={album} />
                                </tr>
                                <tr>
                                    <th>Interpret</th>
                                    <Element track_key={track_key} kind="interpret" value={interpret} />
                                </tr>
                                <tr>
                                    <th>Conductor</th>
                                    <Element track_key={track_key} kind="conductor" value={conductor} />
                                </tr>
                                <tr>
                                    <th>Composer</th>
                                    <Element track_key={track_key} kind="composer" value={composer} />
                                </tr>
                            </table>
                        </div>
                        <div class={style.playlists}>Playlists</div>
                        <PlayButton track_key={track_key} />
                    </td>
                </tr>
            );
    }
}


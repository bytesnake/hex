import {h, Component} from 'preact';
import {Icon} from 'preact-mdl';
import style from './style.less';
import {List} from './list.js';
import Protocol from '../../lib/protocol.js';

const Mode = {
    Closed: 0,
    Small: 1,
    Full: 2,
};

function matchYoutubeUrl(url) {
    var p = /^(?:https?:\/\/)?(?:m\.|www\.)?(?:youtu\.be\/|youtube\.com\/(?:embed\/|v\/|watch\?v=|watch\?.+&v=))((\w|-){11})(?:\S+)?$/;
    if(url.match(p)){
        return url.match(p)[1];
    }
    return false;
}

export default class Upload extends Component {
    state = {
        show: Mode.Closed,
        link_empty: true
    };

    open = () => {
        if(this.state.show == Mode.Closed)
            this.setState({show: Mode.Small});
        else
            this.setState({show: Mode.Closed});
    }

    close = () => {
        this.setState({show: Mode.Closed});
    }

    upload_link = () => {
        let val = this.input.value;

        Protocol.upload_youtube(val);
    }

    input_changed = (e) => {
        let elm = e.target;

        if(elm.value) {
            const url = matchYoutubeUrl(elm.value);
            if(url) {
                elm.classList.add(style.green_border);
                elm.classList.remove(style.red_border);

                if(e.keyCode == 13) {
                    console.log("Start downloading url: " + url);
                    
                    this.upload_link();
                }

            } else {
                elm.classList.add(style.red_border);
                elm.classList.remove(style.green_border);
            }
        } else {
            elm.classList.remove(style.green_border);
            elm.classList.remove(style.red_border);
        }

    }

    render(props, {show, link_empty}) {
        return (
            <div>
                <Icon icon="file upload" onClick={this.open} />

                {show != Mode.Closed && (
                <div class={style.upload}>
                    <List />
                    <div class={style.control}>
                        {!link_empty && (
                            <Icon icon="note add" class={style.link_button} onClick={this.upload_file} />
                        )}
                        {link_empty && (
                            <Icon icon="add circle outline" class={style.link_button} onClick={this.upload_link} />
                        )}
                        <input class={style.link_input} ref={x => this.input = x} onKeyUp={this.input_changed} />
                    </div>
                </div>
                )}
            </div>
        );
    }
}

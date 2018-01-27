import {Component, h} from 'preact';
import style from './style.less';

function matchYoutubeUrl(url) {
    var p = /^(?:https?:\/\/)?(?:m\.|www\.)?(?:youtu\.be\/|youtube\.com\/(?:embed\/|v\/|watch\?v=|watch\?.+&v=))((\w|-){11})(?:\S+)?$/;
    if(url.match(p)){
        return url.match(p)[1];
    }
    return false;
}

export default class Youtube extends Component {
    change = (e) => {
        let elm = e.target;



        if(elm.value) {
            const url = matchYoutubeUrl(elm.value);
            if(url) {
                elm.classList.add(style.green_border);
                elm.classList.remove(style.red_border);
                if(e.keyCode == 13)
                    Protocol.upload_youtube(url);

            } else {
                elm.classList.add(style.red_border);
                elm.classList.remove(style.green_border);
            }
        } else {
            elm.classList.remove(style.green_border);
            elm.classList.remove(style.red_border);
        }
    }

    render({onClose}) {
        return (
            <div class={style.youtube} onClick={onClose}>
                <div class={style.youtube_inner} onClick={e => e.stopPropagation()} >
                    <b>Youtube downloader</b>
                    <input type="text" ref={x => this.input = x} onKeyUp={this.change} />
                </div>
            </div>
        );
    }
}

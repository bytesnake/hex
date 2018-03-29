import {h, Component} from 'preact';
import style from './progress.less';

export default class ProgressBar extends Component {
    componentDidMount() {
        this.update = setInterval(this.update_time, this.props.interval?this.props.interval:300 );
    }

    componentDidUmount() {
        clearInterval(this.update);
    }

    seek = (e) => {
        let rect = this.elm.getBoundingClientRect();

        let slider_pos;
        if(e.screenX < rect.x)
            slider_pos = 0;
        else if(e.screenX > rect.x && e.screenX < rect.x+rect.width)
            slider_pos = (e.screenX - rect.x) / rect.width;
        else
            slider_pos = 1.0;

        let inner = this.elm.children[1];
        let knob = inner.children[0];

        inner.style.width = slider_pos*rect.width + "px";
        knob.style.left = slider_pos*rect.width + "px";

        if(this.seek_timer)
            clearTimeout(this.seek_timer)

        this.seek_timer = setTimeout(() => {
            this.props.player.seek(slider_pos * this.props.player.duration);
            this.update = setInterval(this.update_time.bind(this), 300);
            console.log("Set to " + slider_pos);
        }, 500);
    }

    start_seek = (e) => {
        if(this.props.player.queue.length == 0)
            return;

        clearInterval(this.update);

        window.addEventListener("mousemove", this.seek);
        window.addEventListener("mouseup", _ => {
            window.removeEventListener("mousemove", this.seek);
        });

    }

    update_time = () => {
        if(this.elm == null)
            return;

        let inner_loaded = this.elm.children[0];
        let inner = this.elm.children[1];
        let knob = inner.children[0];

        const time = this.props.player.time_percentage();
        const loaded = this.props.player.loaded_percentage();

        inner.style.width = time * this.elm.offsetWidth + "px";
        inner_loaded.style.width = loaded * this.elm.offsetWidth + "px";
        knob.style.left = time * this.elm.offsetWidth + "px";
    }

    render({player}, {}) {
        return (
            <div class={style.progress_bar} ref={x => this.elm = x}>
                <div class={style.progress_bar_loaded} />
                <div class={style.progress_bar_timer} >
                    <div class={style.round_button} onMouseDown={this.start_seek} ref={x => this.elm_button = x} />
                </div>
            </div>
        );
    }
}

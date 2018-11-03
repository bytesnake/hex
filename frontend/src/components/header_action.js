import {h, Component} from 'preact';
import {Icon} from 'preact-mdl';
import style from 'Style/header_action';
import Portal from 'preact-portal';

export default class HeaderAction extends Component {
    state = {
        is_open: false,
        open_id: 0,
        height: 60,
        start_left: 0
    };

    open(icon) {
        const name = icon.target.innerHTML;
        const id = this.props.icons.indexOf(name);


        if(this.state.is_open && this.state.open_id == id)
            this.setState({is_open: false});
        else {
            this.resize();
            this.setState({is_open: true, open_id: id});
        }
    }

    resize = () => {
        const header = document.querySelector("header");
        const icons = document.querySelector("#action_self");
        var start_left = header.offsetWidth - icons.getBoundingClientRect().left;

        this.setState({ height: header.offsetHeight, start_left });
    }

    componentDidMount = () => {
        window.addEventListener('resize', this.resize);
    }

    componentWillUmount = () => {
        window.removeEventListener('resize', this.resize);
    }

    render({icons},{is_open, open_id, height, start_left}) {
        const tmp = "top: " + height + "px; right: " + (start_left - 40 - (open_id * 45)) + "px;";

        return (
            <div class={style.action} id="action_self">
            { icons.map(x => ( <Icon icon={x} onClick={this.open.bind(this)} /> )) }

            {is_open && (
                    <div class={style.inner} style={tmp}>
                        <div class={style.inner_arrow} />
                        <div class={style.inner_elm}>
                            {this.props.children[open_id]}
                        </div>
                    </div>
            )}
            </div>
        );
    }
}

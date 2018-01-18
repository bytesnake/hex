import { h, Component } from 'preact';
import style from './style.less';
import Spinner from '../spinner';

export default class Home extends Component {
    state = {
        clicked: false
    };

    click() {
        this.setState({ clicked: true });
    }

    render({}, { clicked }) {
        if(clicked) return (<div class={style.home}>Hey</div>);
        else return (<div class={style.home}><span onClick={this.click.bind(this)}>Click me</span></div>);
    }

}


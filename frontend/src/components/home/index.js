import { h, Component } from 'preact';
import style from './style.less';
import Spinner from '../spinner';
import InputSuggest from '../suggest_input';

export default class Home extends Component {
    suggest(val) {
        return ["Baumhaus", "Raumwand"].filter(x => x.indexOf(val) !== -1);
    }

    render({}, { clicked }) {
        return (
            <div class={style.home}>
                <InputSuggest onEnter={x => alert(x)} suggest={this.suggest} />
            </div>
        );
    }

}


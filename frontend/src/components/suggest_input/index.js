import {h, Component} from 'preact';
import style from './style.less';

export default class InputSuggest extends Component {
    state = {
        pos: 0,
        suggestions: []
    };

    change = (e) => {
        if(e.target.value == "")
            this.setState({ suggestions: [] });
        else if(this.props.suggest) {
            const selected = this.state.suggestions[this.state.pos];
            const suggestions = Array.from(new Set(this.props.suggest(e.target.value)));

            let new_pos = 0;
            if(selected in suggestions)
                new_pos = suggestions.indexOf(selected);

            this.setState({ suggestions, pos: new_pos });
        }
    }

    key_input = (e) => {
        if(e.which == 9) {
            if(this.state.suggestions.length == 1)
                e.target.value = this.state.suggestions[this.state.pos];
            else {
                let pos = (this.state.pos + 1) % this.state.suggestions.length;
                this.setState({ pos });
            }
        } else if(e.which == 13)
            if(this.props.onEnter) {
                this.props.onEnter(e.target.value);
                e.target.value = "";
                this.setState({ suggestions: [] });
            }
    }

    prevent_tab = (e) => {
        if(e.which == 9)
            e.preventDefault();
    }

    render(props, {pos, suggestions}) {
        let suggestion = "";
        if(suggestions && suggestions[pos])
            suggestion = suggestions[pos];

        return (
            <div class={style.field}>
                <input class={style.input} onInput={this.change} onKeyDown={this.prevent_tab} onKeyUp={this.key_input}/>
                <input class={style.hint} disabled value={suggestion} />
                <b>{suggestions.length}</b>
            </div>
        );
    }
}

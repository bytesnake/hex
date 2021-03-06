import {h, Component} from 'preact';
import {Icon} from 'preact-mdl';
import { route } from 'preact-router';

import style from 'Style/zyklop';
import Protocol from 'Lib/protocol';
import InputSuggest from 'Component/input_suggest';

export default class Upload extends Component {
    state = {
        token: null,
        playlists: null
    };

    componentWillMount() {
        console.log("mount");
        let token = Protocol.last_token()
            .then(token_id => {
                if(token_id == null) this.setState({token: null})
                else return Protocol.get_token(token_id);
            });

        let playlists = Protocol.get_playlists();

        Promise.all([token, playlists]).then(x => this.setState({token: x[0], playlists: x[1]}));
    }

    suggest = (query) => {
        if(!this.state.playlists)
            return [];

        const suggestions = this.state.playlists.map(x => x.title).filter(x => x.indexOf(query) === 0);

        return suggestions;
    }

    enter = (value) => {
        let val = this.state.playlists.filter(x => x.title == value);

        if(val.length > 0) {
            Protocol.update_token(this.state.token[0].token, val[0].key).then(_ => {
                let token = this.state.token;
                token[0].key = val[0].key;
                token[1][0] = val[0];
                token[1][1] = val[1];

                console.log("UPDATED : " + token[0].key);
                this.setState({ token });
            });
        }
    }

    render(props, {show, token}) {
        return (
            <div class={style.upload}>
                {token == null && (<span>No token found!</span>)}
                {token != null && (
                        <InputSuggest onEnter={this.enter} suggest={this.suggest} value={token[1] ? token[1][0].title:""} />
                )}
            </div>
        );
    }
}

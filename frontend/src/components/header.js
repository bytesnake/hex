import { h, Component } from 'preact';
import { Link, route } from 'preact-router';
import {Layout, TextField, Icon} from 'preact-mdl';
import TokenInput from 'preact-token-input';

import style from 'Style/header';
import Protocol from 'Lib/protocol';
import Upload from 'Component/upload';
import Zyklop from 'Component/zyklop';

export default class Header extends Component {
	render(props, {tags}) {
        return (
            <Layout.Header class={style.header}>
            <Layout.HeaderRow class={style.header_row}>
			<Layout.Title>
                <Icon icon="hearing" />
				<a href="/">Musik</a>
			</Layout.Title>
            <div class={style.search}>
                <TokenInput 
                    class={style.search_input}
                    placeholder="Suchen"
                    onClick={(e) => {route('/search/' + encodeURIComponent(e.target.value))}}
                    onChange={(vals) => {route('/search/' + encodeURIComponent(vals.value.join(",")))}}
                    style="background-color:#FFF; color:#000; padding:10px; width: 100%;"
                />

                <div class={style.search_button}>
                    <Icon icon="search" />
                </div>
            </div>
            <div class={style.actions}>
                <Zyklop />
                <Upload />
                <Icon icon="info outline" />
            </div>
            </Layout.HeaderRow>
            </Layout.Header>
        );
	}
}

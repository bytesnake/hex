import { h, Component } from 'preact';
import { Link, route } from 'preact-router';
import style from './style.less';
import {Layout, TextField, Icon} from 'preact-mdl';
import Protocol from '../../lib/protocol.js';
import Upload from '../upload';
import Zyklop from '../zyklop';
import TokenInput from 'preact-token-input';

export default class Header extends Component {
	render(props, {tags}) {
        return (
            <Layout.Header class={style.header}>
            <Layout.HeaderRow>
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
            <div class={style.upload}>
                <Zyklop />
                <Upload />
                <Icon icon="info outline" />
            </div>
            </Layout.HeaderRow>
            </Layout.Header>
        );
	}
}
                /*<TextField
                    class={style.search_input}
                    placeholder="Suchen"
                    type="search"
                    onClick={(e) => {route('/search/' + encodeURIComponent(e.target.value))}}
                    onInput={(e) => {route('/search/' + encodeURIComponent(e.target.value))}}
                    style="background-color:#FFF; color:#000; padding:10px; width: 100%;"
                />*/

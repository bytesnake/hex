import { h, Component } from 'preact';
import { Link, route } from 'preact-router';
import style from './style.less';
import {Layout, TextField} from 'preact-mdl';
import Protocol from '../../lib/protocol.js';

export default class Header extends Component {
	render(props) {
        return (
            <Layout.Header class={style.header}>
            <Layout.HeaderRow>
			<Layout.Title>
				<a href="/">Hex music</a>
			</Layout.Title>
			<Layout.Spacer />
			<TextField
				placeholder="Search"
				type="search"
                onInput={(e) => {route('/search/' + encodeURIComponent(e.target.value))}}
				style="background-color:#FFF; color:#000; padding:10px;"
			/>
            </Layout.HeaderRow>
            </Layout.Header>
        );
	}
}

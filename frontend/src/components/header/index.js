import { h, Component } from 'preact';
import { Link } from 'preact-router';
import style from './style.less';
import {Layout, TextField} from 'preact-mdl';

export default class Header extends Component {
    onSearch = () => {
    }

	render() {
		/*return (
			<header class={style.header}>
				<h1>Preact App</h1>
				<nav>
					<Link href="/">Home</Link>
					<Link href="/profile">Me</Link>
					<Link href="/profile/john">John</Link>
				</nav>
			</header>
		);*/

        /*return (
            <Layout.Header class={style.header}>
            <Layout.HeaderRow>
			<Layout.Title>
				<a href="/">Hex music</a>
			</Layout.Title>
			<Layout.Spacer />
			<TextField
				placeholder="Search"
				type="search"
				onSearch={this.onSearch}
				style="background-color:#FFF; color:#000; padding:10px;"
			/>
            </Layout.HeaderRow>
            </Layout.Header>
        );*/

        return;
	}
}

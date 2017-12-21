import { h, Component } from 'preact';
import { Layout, Navigation } from 'preact-mdl';

class Sidebar extends Component {
	shouldComponentUpdate() {
		return false;
	}

	hide = () => {
		this.base.classList.remove('is-visible');
	};

	render() {
		/*return (
			<Layout.Drawer onClick={this.hide}>
				<Layout.Title>Example App</Layout.Title>
				<Navigation>
					<Navigation.Link href="/">Home</Navigation.Link>
					<Navigation.Link href="/profile">Profile</Navigation.Link>
					<Navigation.Link href="/profile/john">John</Navigation.Link>
				</Navigation>
			</Layout.Drawer>
		);*/
        return;
	}
}

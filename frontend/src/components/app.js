import { h, Component } from 'preact';
import { Router } from 'preact-router';
import { Button, Icon, Layout } from 'preact-mdl';

import Header from './header';
import Sidebar from './sidebar';
import Home from './home';
import Profile from './profile';

export default class App extends Component {
	/** Gets fired when the route changes.
	 *	@param {Object} event		"change" event from [preact-router](http://git.io/preact-router)
	 *	@param {string} event.url	The newly routed URL
	 */
	handleRoute = e => {
		this.currentUrl = e.url;
	};

    handleFab = () => {
		alert('Add a new song!');
	};

	render() {
		return (
			<div id="app">
                <Layout fixed-header fixed-drawer>
                    <Header />
                    <Sidebar />

                    <Button id="fab" fab colored onClick={this.handleFab}>
                        <Icon icon="create" />
                    </Button>

                    <Layout.Content>
                        <Router onChange={this.handleRoute}>
                            <Home path="/" />
                            <Profile path="/profile/" user="me" />
                            <Profile path="/profile/:user" />
                        </Router>
                    </Layout.Content>
                </Layout>
			</div>
		);
	}
}

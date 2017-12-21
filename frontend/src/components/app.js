import { h, Component } from 'preact';
import { Router } from 'preact-router';
import { Button, Icon } from 'preact-mdl';

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
		alert('You clicked New!');
	};

	render() {
		return (
			<div id="app">
				<Header />
                /*<Sidebar />

                <Button id="fab" fab colored onClick={this.handleFab}>
                    <Icon icon="create" />
                </Button>
*/
				<Router onChange={this.handleRoute}>
					<Home path="/" />
					<Profile path="/profile/" user="me" />
					<Profile path="/profile/:user" />
				</Router>
			</div>
		);
	}
}

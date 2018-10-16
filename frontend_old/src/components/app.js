import { h, Component } from 'preact';
import { Router } from 'preact-router';
import { Button, Icon, Layout } from 'preact-mdl';

import Header from './header';
import Sidebar from './sidebar';
import Home from './home';
import Search from './search';
import Upload from './upload';
import MusicPlayer from './music_player';
import Playlist from './playlist';

export default class App extends Component {
	/** Gets fired when the route changes.
	 *	@param {Object} event		"change" event from [preact-router](http://git.io/preact-router)
	 *	@param {string} event.url	The newly routed URL
	 */
	handleRoute = e => {
		this.currentUrl = e.url;
	};

    componentDidMount() {
        //setTimeout(function() {
        //    window.player.play("5fc1d01ec41c440e90fc9dc880e38402");
        //}, 500);

    }

	render() {
		return (
			<div id="app">
                <Layout fixed-header>
                    <Header ref={ x => this.header = x} />
                    <Sidebar />

                    <Layout.Content>
                        <Router onChange={this.handleRoute}>
                            <Home path="/" />
                            <Home path="/index.html" />
                            <Playlist path="/playlist/:pl_key" />
                            <Search path="/search/:query?" />
                        </Router>
                        <MusicPlayer ref={ x => window.player = x } />
                    </Layout.Content>

                </Layout>
			</div>
		);
	}
}

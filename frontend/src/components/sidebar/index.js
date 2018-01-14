import { h, Component } from 'preact';
import { Icon, Layout, Navigation } from 'preact-mdl';
import style from './style.less';
import Protocol from '../../lib/protocol.js';

export default class Sidebar extends Component {
	shouldComponentUpdate() {
		return true;
	}

	hide = () => {
		this.base.classList.remove('is-visible');
	};

    componentDidMount() {
        let self = this;
        Protocol.get_playlists().then(x => {
            self.setState({playlists: x});
        });
    }

	render({},{playlists}) {
		return (
			<Layout.Drawer onClick={this.hide}>
				<Layout.Title>Example App</Layout.Title>
				<Navigation>
					<Navigation.Link href="/" class={style.link}><Icon icon="home" /><b>Übersicht</b></Navigation.Link>
					<Navigation.Link href="/Verlauf" class={style.link}><Icon icon="history" /><b>Verlauf</b></Navigation.Link>
                    <div class={style.line} />
                    <div class={style.header}>Playlists</div>
                    { playlists && playlists.map( x => (
                        <Navigation.Link href={"/playlist/" + x.key} class={style.link}><Icon icon="queue music" /><b>{x.title}</b><span>{x.count}</span></Navigation.Link>
                    ))}
				</Navigation>
			</Layout.Drawer>
		);
	}
}

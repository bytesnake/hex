import { h, Component } from 'preact';
import { Icon, Layout, Navigation } from 'preact-mdl';
import style from './style.less';
import Protocol from '../../lib/protocol.js';

export default class Sidebar extends Component {
	state = {
        playlists: [],
        create: false
    };
    
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

    click(e) {
        this.setState({create: true});
        e.stopPropagation();
    }

    add_playlist(e) {
        const name = this.elm_name.value;

        let self = this;
        Protocol.add_playlist(name).then(new_pl => {
            let playlists = self.state.playlists;
            playlists.push(new_pl);

            this.setState({playlists: playlists, create: false});
        });

        e.stopPropagation();
    }

	render({},{playlists, create}) {
		return (
			<Layout.Drawer onClick={this.hide}>
				<Layout.Title>Example App</Layout.Title>
				<Navigation>
					<Navigation.Link href="/" class={style.link}><Icon icon="home" /><b>Übersicht</b></Navigation.Link>
					<Navigation.Link href="/Verlauf" class={style.link}><Icon icon="history" /><b>Verlauf</b></Navigation.Link>
                    <div class={style.line} />
                    <div class={style.header}>Playlists
                        { !create && (
                            <Icon icon="add" onClick={this.click.bind(this)} />
                        )}
                    </div>
                    { create && (
                        <div class={style.link}><input placeholder="Name" onClick={e => e.stopPropagation()} ref={x => this.elm_name = x} /><Icon icon="add" onClick={this.add_playlist.bind(this)} /> </div>
                    )}
                    { playlists.map( x => (
                        <Navigation.Link href={"/playlist/" + x.key} class={style.link}><Icon icon="queue music" /><b>{x.title}</b><span>{x.count}</span></Navigation.Link>
                    ))}
				</Navigation>
			</Layout.Drawer>
		);
	}
}

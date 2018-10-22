import {h, Component} from 'preact';

export default class PlaylistForTrack {
    state = {
        update: false,
        playlists: null
    };

    constructor(props) {
        super(props);

        this.update_playlists();
    }

    componentWillReceiveProps(newProps) {
        if(newProps.track_key != this.props.track_key)
            this.update_playlists();
    }

    update_playlists() {
        const key = this.props.track_key;

        this.setState({ update: true });
        Protocol.get_playlists_of_track(key).then(x => {
            this.setState({ update: false, playlists: x });
        });
    }

   render({track_key},{update, playlists}) {
       let elm;
       if(update) elm = (<span>Updating</span>); 
       else elm = (
           <div class={style.inner}>
                <div class={style.playlists}>
                    { playlists && playlists.length > 0 && playlists.map(x => (
                        <span onClick={this.loadPlaylist({x.pl_key})}>{x.title}</span>
                    ))}
                </div>
           </div>
           <div class={style.add_to}>
                <input placeholder="Add to playlist" ref={x => this.add_input} />
                <Icon icon="add" />
           </div>
        );

        return (<div class={style.main}>{elm}</div>);
   }

}

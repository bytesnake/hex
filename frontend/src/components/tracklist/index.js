import {Component, h} from 'preact';
import Track from './track.js';
import style from './style.less';

const Size = {
    FULL: 0,
    OMIT_COMP_COND: 1,
    ONLY_TITLE: 2
};

export default class TrackList extends Component {
    state = {
        size: Size.FULL // Omit Conductor, Composer if screen too small
    };
    
    componentDidMount() {
        window.addEventListener('resize', this.handleResize);
        this.handleResize();
    }

    componentDidUmount() {
        window.removeEventListener('resize', this.handleResize);
    }

    handleResize = () => {
        const width = this.list.clientWidth;
        if(width < 500)
            this.setState({ size: Size.ONLY_TITLE });
        else if(width < 1000)
            this.setState({size: Size.OMIT_COMP_COND })
        else
            this.setState({size: Size.FULL });
    }

    render({tracks},{size}) {
        return (
            <div class={style.tracklist} ref={x => this.list = x}>
            <table>
                <tr class={style.header}>
                    <th>Title</th>
                    {size != Size.ONLY_TITLE && (<th>Album</th>)}
                    {size != Size.ONLY_TITLE && (<th>Interpret</th>)}
                    {size == Size.FULL && (<th>Conductor</th>)}
                    {size == Size.FULL && (<th>Composer</th>)}
                </tr>

                { tracks.map( x => (
                    <Track minimal size={size} track_key={x.key} {...x} />
                )) }
            </table>
            </div>
        );
        
    }

}

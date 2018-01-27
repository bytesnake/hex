import {h, Component} from 'preact';
import {Icon} from 'preact-mdl';
import style from './style.less';
import Portal from 'preact-portal';
import Youtube from './youtube';
import Files from './file';

const Showing = {
    Nothing: 0,
    Menu: 1,
    Youtube: 2,
    Files: 3
};

export default class Upload extends Component {
    state = {
        show: Showing.Nothing
    };

    open = () => {
        if(this.state.show == Showing.Nothing)
            this.setState({show: Showing.Menu});
        else
            this.setState({show: Showing.Nothing});
    }

    youtube = () => {
        this.setState({show: Showing.Youtube});
    }

    files = () => {
        this.setState({show: Showing.Files});
    }

    close = () => {
        this.setState({show: Showing.Nothing});
    }

    render(props, {show}) {
        let inner = (<div></div>);
        console.log(show);

        switch(show) {
            case Showing.Menu:
                inner = (
                    <div class={style.menu}>
                        <div class={style.arrow_up} />
                        <span onClick={this.youtube} >Youtube</span>
                        <span onClick={this.files} >Dateien</span>
                    </div>
                );
                break;
            case Showing.Youtube:
                inner = (
                    <Portal into="body">
                        <Youtube onClose={this.close} />
                    </Portal>
                );
                break;
            case Showing.Files:
                inner = (
                    <Portal into="body">
                        <Files onClose={this.close} />
                    </Portal>
                );
                break;
        }

        if(show != Showing.Nothing) return (
            <div>
                <Icon icon="file upload" onClick={this.open} />
                
                {inner}
            </div>
        );

        else return (
            <Icon icon="file upload" onClick={this.open} />
        );
    }
}

import { h, Component } from 'preact';
import style from './style.less';
import { Button, Icon } from 'preact-mdl';
import MozillaFolder from '../../lib/upload';
import Protocol from '../../lib/protocol';
import TrackView from './track';

const State = {
    Open: 0,
    Uploading: 1,
    UploadDone: 2,
    Tracks: 3
};

export default class Upload extends Component {
    state = {
        intern: State.Open
    };

    handleClick = (e) => {
        console.log("BLUB");
        this.setState({ intern: State.Open });
    }

    dragHandler = () => {
        console.log("BLUB");
        this.setState({ drag: true });
    }

    filesDropped = (e) => {
        e.stopPropagation();
        e.preventDefault();

        this.setState({ intern: State.Uploading });

        var files = [];
        for(const entry of e.target.files) {
            let file = "webkitGetAsEntry" in entry ? entry.webkitGetAsEntry() : entry;

            var name;
            switch(file.type) {
                case "audio/mpeg":
                    name = "mp3"; break;
                case "audio/x-wav":
                    name = "wav"; break;
                case "audio/mp4":
                    name = "mp4"; break;
                default:
                    continue;
            }
            files.push([name, file.slice()]);
        }

        console.log(files);

        Protocol.upload_files(files)
        .then(keys => {
            this.setState({ intern: State.UploadDone, tracks: keys.map(x => {
                return {
                    "key": x,
                    "album": null,
                    "title": null,
                    "interpret": null,
                    "conductor": null,
                    "composer": null
                };
            })});

            return new Promise((resolve) => setTimeout(resolve(keys), 1500));
        })
        .then(keys => {
            console.log(keys);

            this.setState({ intern: State.Tracks });
        });
    }

    render({onClose},{ intern, tracks }) {
        console.log("State: " + intern);

        if(intern == State.Open) return (
            <div class={style.files_inner} onClick={e => e.stopPropagation()}>
                <input type="file" webkitdirectory allowdir mozdirectory onChange={this.filesDropped} />
                <div class={style.drop}>
                    <b>Drop your file/folder here!</b>
                </div>
            </div>);
        else if(intern == State.Uploading) return (<div class={style.files_data}><p><Icon icon="cloud upload" /> Uploading ...</p></div>);
        else if(intern == State.UploadDone) return (<div class={style.files_data}><p><Icon icon="cloud done" /> Upload done</p></div>);
        else if(intern == State.Tracks) return (<TrackView tracks={tracks} finished={onClose} />);
    }
}

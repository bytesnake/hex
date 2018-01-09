import {h, Component} from 'preact';
import Decoder from '../../lib/opus/decoder.js';
import Protocol from '../../lib/protocol.js';
import {guid} from '../../lib/uuid.js';

export default class MusicPlayer {
    state = {
        key: null,
        proto_id: null
    };
    
    async componentDidMount() {
        this.decoder = new Decoder(2);

        const uuid = guid();

        for await (const pack of Protocol.stream(uuid, "178f6fd3faf44c49a92e0c1e1b069bb6")) {
            console.log(pack.length);
        }
    }

    play(key) {
        this.setState({ key: key });
    }


    render({}, {}) {
        return (
            <div></div>
        );
    }
}

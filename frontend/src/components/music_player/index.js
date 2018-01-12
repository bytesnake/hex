import {h, Component} from 'preact';
import Decoder from '../../lib/opus/decoder.js';
import Protocol from '../../lib/protocol.js';
import {guid} from '../../lib/uuid.js';

const BUFFER_SIZE = 8192*2;
const BUFFER_FILL = 4;

export default class MusicPlayer extends Component {
    state = {
        uuid: null,
        key: null,
        proto_id: null
    };
    
    process(e) {
        let ouBuf = e.outputBuffer;
        
        const buf = this.buffer.shift();

        if(buf == undefined)
            return;

        for(let channel = 0; channel < ouBuf.numberOfChannels; channel++) {
            const buf_channel = buf[channel];
            let out = ouBuf.getChannelData(channel);

            for(let sample = 0; sample < BUFFER_SIZE; sample++)
                out[sample] = buf_channel[sample];
        }

        this.fill_buffer();
    }

    async fill_buffer() {
        if(this.buffer.length >= BUFFER_FILL)
            return;

        let rem = BUFFER_FILL - this.buffer.length;

        // create a new array in the case there was no yet
        if(this.tArr == null) {
            this.nbytes = 0;
            this.tArr = [];
            for(let i = 0; i < this.numChannel; i++)
                this.tArr.push(new Float32Array(BUFFER_SIZE));
        }


        for await (const buf_raw of Protocol.stream(this.state.uuid, this.state.key)) {
            let buf;
            try {
                buf = this.decoder.decode(buf_raw);
            } catch(e) {
                console.error("Couldn't parse opus packet: " + e);
            }

            // TODO: more than two channels
            // number of bytes written in a temporary buffer
            const nbytes = this.nbytes;
            const nbuf = buf.length / 2;

            const length = Math.min(BUFFER_SIZE, nbytes + nbuf);

            // the values are interleaved, therefore copy in each step both two the channels
            for(var i=0; i < length - nbytes; i++) {
                this.tArr[0][nbytes+i] = buf[i*2];
                this.tArr[1][nbytes+i] = buf[i*2+1];
            }


            // push a finished buffer and prepare for the next session
            if(nbytes + nbuf >= BUFFER_SIZE) {
                rem --;
                this.buffer.push([this.tArr[0], this.tArr[1]]);

                this.tArr.length = 0;

                for(let i = 0; i < this.numChannel; i++)
                    this.tArr[i] = new Float32Array(BUFFER_SIZE);

                // remaining samples for a single channel
                const more = nbytes + nbuf - BUFFER_SIZE;

                // initialize the buffer with the remaining data
                for(let i=0; i < more; i++) {
                    this.tArr[0][i] = buf[2*(nbuf - more + i)];
                    this.tArr[1][i] = buf[2*(nbuf - more + i) + 1];
                }

                this.nbytes = more;
            } else
                this.nbytes = length;

            if(rem == 0)
                break;
        }

    }

    async componentDidMount() {
        try {
            this.audioContext = new AudioContext();
            this.processor = this.audioContext.createScriptProcessor(BUFFER_SIZE, 2, 2);
            this.source = this.audioContext.createBufferSource(2, 2*48000, 48000);

            this.processor.onaudioprocess = this.process.bind(this);
        } catch(e) {
            throw new Error("Web Audio API is not supported: " + e);
        }

        this.buffer = [];

        // TODO: wait until module loaded, not an arbitrary timespan
        let self = this;
        try {
            setTimeout(function() {
                self.decoder = new Decoder(2);
            }, 500);

        } catch(e) {
            console.error("Couldn't create the decoder: " + e);
        }
    }

    async play(key) {
        if(this.state.key != null) {
            Protocol.stream_end(this.state.uuid);
            this.buffer.length = 0;
        }

        const uuid = guid();
        this.stream = Protocol.stream(uuid, key);
        this.numChannel = 2;

        this.setState({ uuid: uuid, key: key, stream: this.stream });

        await this.fill_buffer();

        this.source.connect(this.processor);
        this.processor.connect(this.audioContext.destination);
        this.source.start();

    }

    render({}, {}) {
        return (
            <div></div>
        );
    }
}

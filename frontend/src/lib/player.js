import Decoder from 'opus/decoder.js';
import Protocol from 'protocol.js';
import {guid} from 'uuid.js';

const BUFFER_SIZE = 8192*2;
const BUFFER_FILL = 4;

export default class Player {
    constructor(numChannel) {
        try {
            this.audioContext = new AudioContext();
            this.processor = this.audioContext.createScriptProcessor(BUFFER_SIZE, numChannel, numChannel);

            this.processor.onaudioprocess = this.process.bind(this);
        } catch(e) {
            throw new Error("Web Audio API is not supported: " + e);
        }

        this.time = 0.0;
        this.playing = false;
        this.numChannel = numChannel;
        this.playlist = [];
        this.playlist_pos = 0;
        this.buffer = [];
        this.nbytes = 0;
        this.tArr = [];
        for(let i = 0; i < numChannel; i++)
            this.tArr.push(new Float32Array(BUFFER_SIZE));

        // TODO: wait until module loaded, not an arbitrary timespan
        let self = this;
        try {
            setTimeout(function() {
                self.decoder = new Decoder(numChannel);
            }, 500);

        } catch(e) {
            console.error("Couldn't create the decoder: " + e);
        }
    }

    // forward to audio output
    process(e) {
        let ouBuf = e.outputBuffer;
        
        // get the oldest element in the buffer
        const buf = this.buffer.shift();

        // if there is no buffer, then we can just write an empty result
        if(buf == undefined || !this.playing) {
            for(let channel = 0; channel < ouBuf.numberOfChannels; channel++) {
                let out = ouBuf.getChannelData(channel);
                for(let sample = 0; sample < BUFFER_SIZE; sample++)
                    out[sample] = 0.0;
            }
        } else {
            this.time += BUFFER_SIZE / 44100;

            for(let channel = 0; channel < ouBuf.numberOfChannels; channel++) {
                const buf_channel = buf[channel];
                let out = ouBuf.getChannelData(channel);

                for(let sample = 0; sample < BUFFER_SIZE; sample++)
                    out[sample] = buf_channel[sample];
            }
        }

        // we have used a buffer or it was empty, refill it
        if(this.playing)
            this.fill_buffer();
    }

    async fill_buffer() {
        if(this.buffer.length >= BUFFER_FILL || !this.playing || this.playlist.length == 0)
            return;

        let rem = BUFFER_FILL - this.buffer.length;
        const key = this.playlist[this.playlist_pos].key;

        if(key == undefined)
            return;

        for await (const buf_raw of Protocol.stream(this.uuid, key)) {
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

    // clear the playlist
    clear() {
        this.stop();

        this.playlist = [];
        this.playlist_pos = 0;
        this.uuid = null;
        this.time = 0.0;
        this.buffer.length = 0;
        this.nbytes = 0;
    }

    // add a new track to play
    add_track(key) {
        let playlist = this.playlist;
        let tmp = Protocol.get_track(key);
        tmp.then(x => {
            playlist.push(x);
        });

        return tmp;
    }

    play() {
        if(this.playing)
            return;

        if(this.uuid == null)
            this.uuid = guid();

        console.log("Play with uuid: " + this.uuid);

        this.playing = true;
        this.processor.connect(this.audioContext.destination);
    }

    stop() {
        if(!this.playing)
            return;

        this.playing = false;
        this.processor.disconnect(this.audioContext.destination);
    }

    next() {
        Protocol.stream_end(this.uuid);

        this.time = 0.0;
        this.buffer.length = 0;
        this.playlist_pos ++;
        this.uuid = guid();
    }

    prev() {
        Protocol.stream_end(this.uuid);

        this.time = 0.0;
        this.buffer.lenght = 0;
        this.playlist_pos --;
        this.uuid = guid();
    }

    time() {
        return this.time;
    }

    time_percentage() {
        if(this.playlist.length == 0)
            return 0.0;

        return this.time / this.playlist[this.playlist_pos].duration;
    }
}

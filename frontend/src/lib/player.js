import Worker from 'worker-loader!./worker.js';
import Protocol from 'protocol.js';

class AudioBuffer {
    constructor(sample_rate, channel, samples, finished)  {
        this.channel = channel;
        this.sample_rate = sample_rate;

        this.worker = new Worker();

        this.pos = 0;

        //this.worker.postMessage({kind: 0, channel: channel, samples: samples});
        this.worker.onmessage = this.on_packet.bind(this);
        this.finished = finished;
    }

    next(length) {
        if(this.pos+length > this.buffer[0].length) {
            length = this.buffer[0].length - this.pos;
            this.finished();
        }

        const slice1 = this.buffer[0].slice(this.pos, this.pos+length);
        const slice2 = this.buffer[1].slice(this.pos, this.pos+length);
        this.pos += length;

        //console.log(this.pos);

        return [slice1,slice2];
    }

    next_track(track) {
        const samples = track.duration * this.sample_rate;

        this.buffer = [new Float32Array(samples), new Float32Array(samples)];
        this.pos = 0;

        this.worker.postMessage({kind: 0, channel: this.channel, samples: samples, track: track, sample_rate: this.sample_rate});
    }

    set pos(new_pos) {
        this.worker.postMessage({kind: 1, pos: new_pos});
        this._pos = new_pos;
    }

    get pos() {
        return this._pos;
    }

    bitmap() {
        let self = this;
        return new Promise(function(resolve, reject) {
            self.worker.addEventListener('message', function fnc(e){
                if(e.data && e.data.kind == 1) {
                    self.worker.removeEventListener('message', fnc);
                    resolve(e.data.bitmap);
                }
            });
        });
    }

    on_packet(e) {
        if(e.data.kind == 0) {
            //console.log("Set new one");
            //console.log(e.data.data);

            if(this.buffer[0].length - e.data.offset < e.data.data[0].length) {

                this.buffer[0].set(e.data.data[0].slice(0, this.buffer[0].length - e.data.offset), e.data.offset);
                this.buffer[1].set(e.data.data[1].slice(0, this.buffer[0].length - e.data.offset), e.data.offset);
            } else {
                this.buffer[0].set(e.data.data[0], e.data.offset);
                this.buffer[1].set(e.data.data[1], e.data.offset);
            }
        }
    }
}

const PLAY_BUFFER_SIZE = 2*8192;

export default class Player {
    constructor(numChannel) {
        try {
            this.audioContext = new AudioContext();
            this.processor = this.audioContext.createScriptProcessor(PLAY_BUFFER_SIZE, 0, numChannel);

            this.processor.onaudioprocess = this.process.bind(this);
        } catch(e) {
            throw new Error("Web Audio API is not supported: " + e);
        }

        this.buffer = new AudioBuffer(this.audioContext.sampleRate, 0, numChannel, this.next.bind(this));
        this.playing = false;
        this.numChannel = numChannel;
        this.playlist = [];
        this.playlist_pos = 0;
    }

    // forward to audio output
    process(e) {
        let ouBuf = e.outputBuffer;
        
        // if there is no buffer, then we can just write an empty result
        if(this.playing) {
            // get the oldest element in the buffer
            const buf = this.buffer.next(PLAY_BUFFER_SIZE);

            for(let channel = 0; channel < ouBuf.numberOfChannels; channel++) {
                ouBuf.copyToChannel(buf[channel], channel);
            }
        }
    }


    // clear the playlist
    clear() {
        this.stop();

        this.playlist = [];
        this.playlist_pos = 0;
    }

    // add a new track to play
    add_track(key) {
        let playlist = this.playlist;
        let buffer = this.buffer;

        let tmp = Protocol.get_track(key);
        tmp.then(x => {
            playlist.push(x);

            if(playlist.length == 1)
                buffer.next_track(x);
        });

        return tmp;
    }

    play() {
        if(this.playing)
            return;

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
        if(this.playlist_pos == this.playlist.length - 1) {
            this.stop();
            return;
        }

        this.playlist_pos ++;
        this.buffer.next_track(this.playlist[this.playlist_pos]);
    }

    prev() {
        if(this.playlist_pos - 1 < 0)
            return;

        this.playlist_pos --;
        this.buffer.next_track(this.playlist[this.playlist_pos]);
    }


    get time() {
        return this.buffer.pos / this.audioContext.sampleRate;
    }

    time_percentage() {
        if(this.playlist.length == 0)
            return 0.0;

        return this.time / this.playlist[this.playlist_pos].duration;
    }
}

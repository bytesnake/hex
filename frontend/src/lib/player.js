//import Worker from 'worker-loader!./worker.js';
import Protocol from 'Lib/protocol';

var createRingBuffer = function(length){

  var pointer = 0, buffer = [];

  return {
    get  : function(key){return buffer[key];},
    push : function(item){
      buffer[pointer] = item;
      pointer = (length + pointer +1) % length;
    }
  };
};

class AudioBuffer {
    constructor(sample_rate, channel, samples, finished)  {
        this.channel = channel;
        this.sample_rate = sample_rate;

        this.worker = null;
        //this.worker = new Worker();

        this._pos = 0;
        this.pos_loaded = 0;

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
        this._pos += length;

        return [slice1,slice2];
    }

    next_track(track) {
        const samples = track.duration * this.sample_rate;

        this.buffer = [new Float32Array(samples), new Float32Array(samples)];
        this._pos = 0;
        this.pos_loaded = 0;

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

            //console.log(e.data.offset + e.data.data[0].length);
            //console.log(this.buffer[0].length);
            if(this.buffer[0].length < e.data.offset + e.data.data[0].length) {
                this.pos_loaded = this.buffer[0].length;

                this.buffer[0].set(e.data.data[0].slice(0, this.buffer[0].length - e.data.offset), e.data.offset);
                this.buffer[1].set(e.data.data[1].slice(0, this.buffer[0].length - e.data.offset), e.data.offset);
            } else {
                this.pos_loaded += e.data.data[0].length;

                this.buffer[0].set(e.data.data[0], e.data.offset);
                this.buffer[1].set(e.data.data[1], e.data.offset);
            }
        }
    }
}

const PLAY_BUFFER_SIZE = 2*8192;

export default class Player {
    constructor(numChannel, new_track_cb, set_playing_cb, set_queue_cb, set_queue_pos_cb) {
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
        this.queue = [];
        this.queue_pos = 0;
        this.new_track_cb = new_track_cb;
        this.set_playing_cb = set_playing_cb;
        this.set_queue_cb = set_queue_cb;
        this.set_queue_pos_cb = set_queue_pos_cb;
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


    // clear the queue
    clear() {
        console.log("CLEAR");

        this.stop();

        this.queue = [];
        this.queue_pos = 0;

        this.set_queue_cb(this.queue);
        this.set_queue_pos_cb(this.queue_pos);
    }

    // add a new track to play
    add_track(key) {
        let queue = this.queue;
        let buffer = this.buffer;

        let tmp;
        if(typeof key == "string") {
            tmp = Protocol.get_track(key);
            tmp.then(x => {
                queue.push(x);

                this.set_queue_cb(queue);

                if(queue.length == 1) {
                    this.new_track_cb(x);
                    buffer.next_track(x);
                }
            });
        } else {
            let vecs = [];
            for(var elm of key) {
                vecs.push(Protocol.get_track(elm));
            }
            
            tmp = Promise.all(vecs);

            tmp.then(x => {
                console.log(x);
                queue.push.apply(queue, x);

                this.set_queue_cb(queue);

                if(queue.length == x.length) {
                    this.new_track_cb(x[0]);
                    buffer.next_track(x[0]);
                }
            });
        }

        return tmp;
    }

    is_empty() {
        return this.queue.length == 0;
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

    seek(pos) {
        pos = Math.round(pos);
        if(pos < 0 || pos > this.queue[this.queue_pos].duration)
            return;

        this.buffer.pos = pos * this.audioContext.sampleRate;
    }

    next = () => {
        if(this.queue_pos == this.queue.length - 1) {
            //this.set_playing_cb(false);
            return false;
        }

        this.queue_pos ++;

        this.set_queue_pos_cb(this.queue_pos);

        // if we are playing the same track again, just reset the position
        if(this.queue[this.queue_pos].key == this.queue[this.queue_pos-1].key)
            this.buffer.pos = 0;
        else {
            this.new_track_cb(this.queue[this.queue_pos]);
            this.buffer.next_track(this.queue[this.queue_pos]);
        }

        return true;
    }

    prev = () => {
        if(this.time < 4) {
            if(this.queue_pos - 1 < 0)
                return false;

            this.queue_pos --;

            this.set_queue_pos_cb(this.queue_pos);

            this.new_track_cb(this.queue[this.queue_pos]);
            this.buffer.next_track(this.queue[this.queue_pos]);
        } else
            this.buffer.pos = 0;

        return true;
    }

    shuffle_below_current = () => {
        var j, x, i;
        for (i = this.queue.length - 1; i > this.queue_pos+1; i--) {
            j = this.queue_pos + 1 + Math.floor(Math.random() * (i - this.queue_pos));
            x = this.queue[i];
            this.queue[i] = this.queue[j];
            this.queue[j] = x;
        }

        this.set_queue_cb(this.queue);
    }

    set_queue_pos = (new_pos) => {
        this.queue_pos = new_pos;

        this.set_queue_pos_cb(this.queue_pos);

        this.new_track_cb(this.queue[this.queue_pos]);
        this.buffer.next_track(this.queue[this.queue_pos]);
    }

    remove_track = (pos) => {
        this.queue.splice(pos, 1);

        this.set_queue_cb(this.queue);
    }

    get time() {
        return this.buffer.pos / this.audioContext.sampleRate;
    }

    get duration() {
        return this.queue[this.queue_pos].duration;
    }

    time_percentage() {
        if(this.queue.length == 0)
            return 0.0;

        const val = this.time / this.queue[this.queue_pos].duration;
        if(val == 100)
            this.next();

        return val;
    }

    loaded_percentage() {
        if(this.queue.length == 0)
            return 0.0;

        const tmp = this.buffer.pos_loaded / this.audioContext.sampleRate / this.queue[this.queue_pos].duration;
        //console.log(this.buffer.pos_loaded);

        return tmp;

    }
}

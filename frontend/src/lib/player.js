import Protocol from 'Lib/protocol';

class RingBuffer {
    constructor(channels, duration) {
        this.buf = Array(channels).fill(Array(48000 * duration).fill(0));
        this.ptr_end = 0;
        this.ptr_start = 0;
        this.channels = channels;
        this.samples = 48000 * duration;
    }

    push(buf) {
        let channel = 0;
        for(sample of buf) {
            this.buf[channel][pointer] = sample;

            if(channel == this.channels - 1) {
                channel = 0;
                this.ptr_end = (this.ptr_end % this.samples);
            } else
                channel ++;
        }

        return this.length() >= buf.length / this.channels;
    }

    slice(length) {
        const avail = this.length();
        if(avail < length) return null;

        if(this.ptr_start < this.ptr_end) {
            this.ptr_start = (this.ptr_start + 1) % this.samples;

            return this.buf.map(x => x.slice(this.ptr_start, this.ptr_start + length));
        } else {
            this.ptr_start = (this.ptr_start + 1) % this.samples;
            return this.buf.map(x => {
                let tmp = x.slice(this.ptr_start);
                tmp.push.apply(tmp, x.slice(0, length - (this.ptr_end-this.ptr_start)));

                return tmp;
            });
        }
    }

    clear() {
        this.ptr_end = 0;
        this.ptr_start = 0;
    }

    length() {
        if(this.ptr_start < this.ptr_end)
            return this.ptr_end - this.ptr_start;
        else
            return (this.samples - this.ptr_start) + this.ptr_end;
    }

    should_fill() {
        return this.length() < 48000 * 2;
    }
}

class AudioBuffer {
    constructor(sample_rate, channels, samples, finished)  {
        this.channels = channels;
        this.sample_rate = sample_rate;
        this.samples = samples;

        this.buffer = new RingBuffer(channels, samples);

        this._pos = 0;
        this.pos_loaded = 0;

        this.finished = finished;
    }

    next(length) {
        if(this.pos+length > this.samples) {
            length = this.samples - this.pos;
            this.finished();
        }

        this.buffer.slice(length);

        if(this.buffer.should_fill())
            this.fill_buf();
    }

    load_track(track) {
        this._pos = 0;
        this.pos_loaded = 0;
        this.buffer.clear();

        let [stream_next, stream_seek, stream_end] = Protocol.stream_start(track);
        this.stream_next = stream_next;
        this.stream_seek = stream_seek;
        this.stream_end = stream_end;
    }

    fill_buf() {
        if(this.track == null)
            return;

        this.stream_next().then(x => {
            if(this.buffer.push(x)) this.fill_buf();
        });
    }

    set pos(new_pos) {
        this.stream_seek(new_pos);
        this._pos = new_pos;
    }

    get pos() {
        return this._pos;
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

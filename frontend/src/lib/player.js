import Protocol from 'Lib/protocol';
import Resampler from './resampler.js';

class RingBuffer {
    constructor(channels, duration, sampling_rate) {
        this.buf = Array(channels).fill(new Int16Array(sampling_rate * duration));

        this.ptr_end = 0;
        this.ptr_start = 0;
        this.length = 0;
        this.max_length = sampling_rate * duration;

        this.channels = channels;
        this.sampling_rate = sampling_rate;
    }

    push(buf_arr) {
        let buf_u8 = new Uint8Array(buf_arr);
        let buf = new Int16Array(buf_u8.buffer);

        const num_samples = buf.length / this.channels;

        if(this.resampler == null) {
            this.resampler = new Resampler(48000, this.sampling_rate, this.channels, buf.length);
        }

        //if(this.sampling_rate != 48000)
        //    buf = resampler.resampler(buf);

        if(this.length + num_samples > this.max_length) {
            console.error("Ringbuffer overflow!");
            return false;
        }
        
        for(var i = 0; i < buf.length; i++) {
            const val = buf.slice(i, i+1);
            this.buf[i % 2][this.ptr_end + Math.floor(i/2)] = val[0];
        }

        buf = null;
        buf_u8 = null;
        buf_arr = null;

        this.ptr_end = (this.ptr_end + num_samples) % this.max_length;
        this.length += num_samples;

        return this.length <= this.max_length - num_samples;
    }

    slice(num_samples) {
        if(this.length < num_samples * 10) return null;

        this.length -= num_samples;

        if(this.ptr_start < this.ptr_end) {
            let buf = this.buf.map(x => {
                let tmp = new Float32Array(num_samples);

                for(var i = 0; i < num_samples; i ++)
                    tmp[i] = x[this.ptr_start + i] / (x[this.ptr_start+i] >= 0 ? 32767 :32768);
            
                return tmp;
            })


            this.ptr_start = (this.ptr_start + num_samples) % this.max_length;

            return buf;
        } else {
            const first = Math.min(num_samples, this.max_length - this.ptr_start);
            const last = num_samples - first;

            const buf = this.buf.map(x => {
                let tmp = new Float32Array(num_samples);

                for(var i = 0; i < first; i ++)
                    tmp[i] = x[this.ptr_start+i] / (x[this.ptr_start+i] >= 0 ? 32767 : 32768);

                for(var i = 0; i < last; i ++)
                    tmp[first + i] = x[i] / (x[i] >= 0 ? 32767 :32768);

                return tmp;
            });

            this.ptr_start = (this.ptr_start + num_samples) % this.max_length;

            return buf;
        }
    }

    clear() {
        this.ptr_end = 0;
        this.ptr_start = 0;
        this.length = 0;
    }

    should_fill() {
        return this.length < this.sampling_rate * 10;
    }
}

class AudioBuffer {
    constructor(sample_rate, channels, samples, finished)  {
        this.channels = channels;
        this.sample_rate = sample_rate;
        this.samples = samples;

        this.buffer = new RingBuffer(channels, 40, sample_rate);

        this._pos = 0;
        this.pos_loaded = 0;

        this.finished = finished;
        this.filling = false;
    }

    next(length) {
        if(this.pos+length > this.samples) {
            length = this.samples - this.pos;
            this.finished();
        }

        if(this.buffer.should_fill() && !this.filling) {
            console.log("FILLING UP!");
            this.fill_buf();
        }

        const buf = this.buffer.slice(length);
        if(buf)
            this._pos += length;

        return buf;
    }

    load_track(track) {
        this.samples = Math.round(track.duration * this.sample_rate);
        this._pos = 0;
        this.pos_loaded = 0;
        this.buffer.clear();
        this.track = track;

        let [stream_next, stream_seek, stream_end] = Protocol.start_stream(track.key);
        this.stream_next = stream_next;
        this.stream_seek = stream_seek;
        this.stream_end = stream_end;

        this.fill_buf();
    }

    fill_buf() {
        if(this.track == null)
            return;

        this.filling = true;

        this.stream_next().then(x => {
            this.pos_loaded += x.length / this.channels / 2;

            if(this.buffer.push(x)) {
                setImmediate(this.fill_buf.bind(this));
            } else {
                this.filling = false;
            }

            x.length = 0;
            x = null;
        });
    }

    set pos(new_pos) {
        this.stream_seek(new_pos).then(x => {
            console.log("SEEK TO ");
            console.log(x);
            this._pos = new_pos;
            this.pos_loaded = new_pos;
            this.buffer.clear();

            console.log("Loaded: " + this.pos_loaded);
            this.fill_buf();
        });
    }

    get pos() {
        return this._pos;
    }
}

const PLAY_BUFFER_SIZE = 8192 * 2;

export default class Player {
    constructor(numChannel, new_track_cb, set_playing_cb, set_queue_cb, set_queue_pos_cb) {
        try {
            this.audioContext = new AudioContext();
            this.processor = this.audioContext.createScriptProcessor(PLAY_BUFFER_SIZE, 0, numChannel);

            this.processor.onaudioprocess = this.process.bind(this);
        } catch(e) {
            throw new Error("Web Audio API is not supported: " + e);
        }

        this.buffer = new AudioBuffer(this.audioContext.sampleRate, numChannel, 0, this.next.bind(this));
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
            let buf = this.buffer.next(PLAY_BUFFER_SIZE);
            for(let channel = 0; channel < ouBuf.numberOfChannels; channel++) {
                if(buf)
                    ouBuf.copyToChannel(buf[channel], channel);
                else
                    ouBuf.copyToChannel(new Float32Array(new Array(PLAY_BUFFER_SIZE).fill(0)), channel);
            }

            //buf[0] = null;
            //buf[1] = null;
            buf = null;
        }
    }


    // clear the queue
    clear() {
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
                    buffer.load_track(x);
                }
            });
        } else {
            let vecs = [];
            for(var elm of key) {
                vecs.push(Protocol.get_track(elm));
            }
            
            tmp = Promise.all(vecs);

            tmp.then(x => {
                queue.push.apply(queue, x);

                this.set_queue_cb(queue);

                if(queue.length == x.length) {
                    this.new_track_cb(x[0]);
                    buffer.load_track(x[0]);
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
            this.buffer.load_track(this.queue[this.queue_pos]);
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
            this.buffer.load_track(this.queue[this.queue_pos]);
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
        this.buffer.load_track(this.queue[this.queue_pos]);
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

        return tmp;

    }
}

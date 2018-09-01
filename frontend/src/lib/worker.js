import 'babel-polyfill';
import Decoder from 'opus/decoder.js';
import Protocol from 'protocol.js';
import {guid} from 'uuid.js';
import Resampler from './resampler.js';
import Bitmap from 'bitmap.js';

const BUF_SIZE = 2**15;

let track = null;
let buffer = null;
let buf_size = 0;
let stream = null;
let bitmap = null;
let bitmap_pos = 0;
let uuid = null;
let channel = 2;
let resampler = null;
let resampler_length = 0;
let sample_rate = null;

let abort = null;
let finished = true;

function reg_abort(fnc) {
    if(finished)
        fnc();
    else
        abort = fnc;
}

function clear_abort() {
    abort = null;
}

// TODO: wait until module loaded, not an arbitrary timespan
let decoder;
try {
    setTimeout(function() {
        decoder = new Decoder(2);
    }, 500);
} catch(e) {
    console.error("Couldn't create the decoder: " + e);
}

function fill_buf() {
    finished = false;
    // abort the buffering if the track has changed
    if(abort && typeof abort == "function") {
        abort();
        return;
    }

    Protocol.stream(uuid, track.key).next().then(buf_raw => {
        // if we got all packets, then send the last block
        if(buf_raw.done) {
            // fill the remaining space with zero
            buffer.fill(0, buf_size);

            // and send it to the main thread
            self.postMessage({kind: 0, offset: bitmap_pos*BUF_SIZE, data: buffer});

            // set the bit
            bitmap.set(bitmap_pos, true);

            // we are finished here
            finished = true;

            // stop the stream
            return;
        }



        /*let buf;
        try {
            buf = decoder.decode(buf_raw.value);
        } catch(e) {
            console.error("Couldn't parse opus packet: " + e);
        }*/

        let buf = new Int16Array(buf_raw.value.buffer);

        //console.log(buf.length);

        if(resampler == null || resampler_length != buf.length) {
            resampler = new Resampler(48000, sample_rate, 2, buf.length);
            resampler_length = buf.length;
        }

        //console.log("Before: " + buf.length);
        buf = resampler.resampler(buf);
        //console.log(buf);

        //console.log("Start size: " + buf_size);
        const new_size = buf_size+buf.length / 2;
        let j = 0;
        for(let i = buf_size; i < Math.min(BUF_SIZE, new_size); i++) {
            buffer[0][i] = buf[j] / (buf[j] >= 0 ? 32767 : 32768);
            buffer[1][i] = buf[j+1] / (buf[j+1] >= 0 ? 32767 : 32768);

            j += 2;
        }
        //console.log("Copied up to: " + j);

        if(new_size >= BUF_SIZE) {
            //console.log(buffer);

            self.postMessage({kind: 0, offset: bitmap_pos*BUF_SIZE, data: buffer});
            // set the bit and increase the position
            bitmap.set(bitmap_pos, true);
            
            // skip tiles until an empty one is encountered
            while(bitmap.is_set(bitmap_pos)) {
                // if we reached the end
                if(bitmap_pos == bitmap.size-1) {
                    console.log(buffer);
                    return;
                }

                bitmap_pos += 1;
            }

            //console.log(bitmap.as_arr());
            // fill the remaining bytes to the next buffer
            if(new_size > BUF_SIZE) {
                for(let i = 0; i < new_size-BUF_SIZE; i++) {
                    buffer[0][i] = buf[j] / (buf[j] >= 0 ? 32767 : 32768);
                    buffer[1][i] = buf[j+1] / (buf[j+1] >= 0 ? 32767 : 32768);

                    j += 2;
                }

                buf_size = new_size-BUF_SIZE;
            } else
                buf_size = 0;
        } else 
            buf_size = new_size;

        setImmediate(fill_buf);
    });
}


onmessage = function(e) {
    const kind = e.data.kind;
    if(kind == 0) {
        reg_abort(() => {
            // create a new buffer for a new track
            sample_rate = e.data.sample_rate;
            track = e.data.track;
            buffer = [new Float32Array(BUF_SIZE), new Float32Array(BUF_SIZE)];
            uuid = guid();
            stream = Protocol.stream(uuid, track.key);
            bitmap = new Bitmap(Math.trunc(track.duration * sample_rate / BUF_SIZE)+1);
            buf_size = 0;
            bitmap_pos = 0;

            clear_abort();

            fill_buf();
        });
    } else if(kind == 1) {
        console.log("change pos to " + e.data.pos);
        reg_abort(() => {
            buf_size = 0;
            bitmap_pos = 0;

            clear_abort();

            fill_buf();
        });

    }

}


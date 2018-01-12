import opus from './libopus.js';
//import makeOpus from '../opus.wasm';
//import opus from './test.js';

function toHexString(byteArray) {
  return Array.from(byteArray, function(byte) {
    return ('0' + (byte & 0xFF).toString(16)).slice(-2);
  }).join(' ')
}

function throw_error(error_code) {
    switch(error_code) {
        case -1:
            throw new Error("Opus: bad argument");
        case -2:
            throw new Error("Opus: buffer too small");
        case -3:
            throw new Error("Opus: internal error");
        case -4:
            throw new Error("Opus: invalid packet");
        case -5:
            throw new Error("Opus: invalid state");
        case -6:
            throw new Error("Opus: allocation failed");
    }
}

export default class Decoder {
    constructor(channels) {

//        opus.run();
        //opus.opus._opus_decoder_create(48000, 2);
            let retPtr = opus._malloc(4);
            this.opus = opus._opus_decoder_create(48000, channels, retPtr);
            throw_error(new Int32Array(opus.HEAP32.buffer, retPtr, 1)[0]);

            console.log("Created decoder");

            // prealloc in buffer
            this.buflen = 2400;
            this.buf = opus._malloc(2400);
            // prealloc out buffer
            this.outbuflen = Float32Array.BYTES_PER_ELEMENT * channels * 2000;
            this.outbuf = opus._malloc(this.outbuflen);

            this.channels = channels;
    }

    decode(data) {
        let res = [];
        let pos = 0;
        while(true) {
            //let length = data.slice(pos, (pos+4)).map(Number);
            //let length = new Uint32Array(data.slice(pos, pos+4))[1];
            let length = data[pos] << 24 | 
                         data[pos+1] << 16 |
                         data[pos+2] << 8 |
                         data[pos+3];
            pos += 4;
            //console.log(length);

            // convert the js array to malloc array
            if(this.buflen < length) {
                this.buf = opus._realloc(this.buf, length);
                this.buflen = length;
            }

            //console.log(toHexString(data));

            opus.HEAPU8.set(data.slice(pos, pos+length), this.buf);
            let len = opus._opus_decode_float(this.opus, this.buf, length, this.outbuf, this.outbuflen);

            //console.log("Decode packet with size " + length + ": buflen: " + this.buflen+ "; outbuflen: " + this.outbuflen + "; got: " + len);

            // check for errors
            throw_error(len);

            // truncate the resulting array
            // WTF
            res.push(new Float32Array(opus.HEAPF32.subarray((this.outbuf >> 2), (this.outbuf >> 2) + len * this.channels)));

            pos += length;

            if(pos >= data.length)
                break;
        }

        let ret_data = new Float32Array(res.map(x => x.length).reduce((a, b) => a+b, 0));
        let i = 0;
        for(const elm of res) {
            ret_data.set(elm, i);
            i += elm.length;
        }

        //console.log(ret_data);

        return ret_data;
    }

}

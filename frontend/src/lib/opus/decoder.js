import opus from './libopus.js';

export default class Decoder {
    constructor(channels) {
        this.opus = opus._opus_decoder_create(48000, channels);
        this.channels = channels;
    }

    decode(data) {
        const buf_len = Float32Array.BYTES_PER_ELEMENT * 5760 * this.channels;
        let buf = opus._malloc(buf_len);
        let len = opus._opus_decode_float(this.opus, data, data.length, buf_len, 0);

        if(len < 0)
            throw new Error("Opus decoding error: " + len);

        const samples = opus.HEAPF32.subarray(buf, buf + len * Float32Array.BYTES_PER_ELEMENT * this.channels);

        return new Float32Array(samples);
    }
}

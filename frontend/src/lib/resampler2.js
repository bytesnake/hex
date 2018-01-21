export default class Resampler {
    constructor(numChannel, fromSampleRate, toSampleRate) {
        this.numChannel = numChannel;
        this.from = fromSampleRate;
        this.to = toSampleRate;
        this.context = null;
    }

    resample(inbuf) {
        // asume interleaved values
        const length = inbuf.length / this.numChannel;

        if(!this.context || this.buffer.length != length) {
            this.context = new OfflineAudioContext(this.numChannel, length*this.to/this.from, this.to);
            this.buffer = this.context.createBuffer(this.numChannel, length, this.from)
            this.source = this.context.createBufferSource();
            this.source.buffer = this.buffer;
            this.source.connect(this.context.destination);
            this.source.start();
        }

        let ch1 = this.buffer.getChannelData(0);
        let ch2 = this.buffer.getChannelData(1);
        for(let i = 0; i < inbuf.length; i++) {
            if(i%2==0) ch1[i/2] = inbuf[i];
            else ch2[i/2] = inbuf[i];
        }


        let self = this;
        /*return this.context.startRendering().then(x => {
            self.source.stop();
            return Promise.resolve(x);
        });*/
        return this.context.startRendering();
        //return Promise.resolve(5);
    }
}

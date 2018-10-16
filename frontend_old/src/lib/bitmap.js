export default class Bitmap {
    constructor(size) {
        this.bins = new Uint8Array(Math.floor(size / 8) + 1);
        this.size = size;
    }

    set(pos, val) {
        if(pos >= this.size || !(typeof val === "boolean"))
            throw new Error("Out of range: " + pos + " >= " + this.size);

        const bin = Math.trunc(pos/8);
        const idx = pos % 8;

        if(val)
            this.bins[bin] |= 1 << idx;
        else
            this.bins[bin] &= ~(1 << idx);
    }

    is_set(pos) {
        if(pos >= this.size)
            throw new Error("Out of range: " + pos + " >= " + this.size);

        const bin = Math.trunc(pos/8);
        const idx = pos % 8;
        
        return ((this.bins[bin] >> idx) & 1) == 1;
    }

    as_arr() {
        let arr = new Array(this.size);

        for(let i = 0; i < this.size; i++)
            arr[i] = this.is_set(i);

        return arr;
    }
}



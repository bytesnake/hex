/* tslint:disable */
import * as wasm from './hex_server_protocol_bg.wasm';

let cachegetUint32Memory = null;
function getUint32Memory() {
    if (cachegetUint32Memory === null || cachegetUint32Memory.buffer !== wasm.memory.buffer) {
        cachegetUint32Memory = new Uint32Array(wasm.memory.buffer);
    }
    return cachegetUint32Memory;
}

function passArray32ToWasm(arg) {
    const ptr = wasm.__wbindgen_malloc(arg.length * 4);
    getUint32Memory().set(arg, ptr / 4);
    return [ptr, arg.length];
}

const slab = [{ obj: undefined }, { obj: null }, { obj: true }, { obj: false }];

let slab_next = slab.length;

function addHeapObject(obj) {
    if (slab_next === slab.length) slab.push(slab.length + 1);
    const idx = slab_next;
    const next = slab[idx];

    slab_next = next;

    slab[idx] = { obj, cnt: 1 };
    return idx << 1;
}

let cachegetUint8Memory = null;
function getUint8Memory() {
    if (cachegetUint8Memory === null || cachegetUint8Memory.buffer !== wasm.memory.buffer) {
        cachegetUint8Memory = new Uint8Array(wasm.memory.buffer);
    }
    return cachegetUint8Memory;
}

function getArrayU8FromWasm(ptr, len) {
    return getUint8Memory().subarray(ptr / 1, ptr / 1 + len);
}

let cachedGlobalArgumentPtr = null;
function globalArgumentPtr() {
    if (cachedGlobalArgumentPtr === null) {
        cachedGlobalArgumentPtr = wasm.__wbindgen_global_argument_ptr();
    }
    return cachedGlobalArgumentPtr;
}
/**
* @param {Uint32Array} arg0
* @param {any} arg1
* @returns {Uint8Array}
*/
export function request_to_buf(arg0, arg1) {
    const [ptr0, len0] = passArray32ToWasm(arg0);
    const retptr = globalArgumentPtr();
    wasm.request_to_buf(retptr, ptr0, len0, addHeapObject(arg1));
    const mem = getUint32Memory();
    const rustptr = mem[retptr / 4];
    const rustlen = mem[retptr / 4 + 1];
    if (rustptr === 0) return;
    const realRet = getArrayU8FromWasm(rustptr, rustlen).slice();
    wasm.__wbindgen_free(rustptr, rustlen * 1);
    return realRet;

}

const lTextEncoder = typeof TextEncoder === 'undefined' ? require('util').TextEncoder : TextEncoder;

let cachedTextEncoder = new lTextEncoder('utf-8');

function passStringToWasm(arg) {

    const buf = cachedTextEncoder.encode(arg);
    const ptr = wasm.__wbindgen_malloc(buf.length);
    getUint8Memory().set(buf, ptr);
    return [ptr, buf.length];
}

function passArray8ToWasm(arg) {
    const ptr = wasm.__wbindgen_malloc(arg.length * 1);
    getUint8Memory().set(arg, ptr / 1);
    return [ptr, arg.length];
}
/**
* @param {Uint32Array} arg0
* @param {string} arg1
* @param {string} arg2
* @param {Uint8Array} arg3
* @returns {Uint8Array}
*/
export function upload_track(arg0, arg1, arg2, arg3) {
    const [ptr0, len0] = passArray32ToWasm(arg0);
    const [ptr1, len1] = passStringToWasm(arg1);
    const [ptr2, len2] = passStringToWasm(arg2);
    const [ptr3, len3] = passArray8ToWasm(arg3);
    const retptr = globalArgumentPtr();
    wasm.upload_track(retptr, ptr0, len0, ptr1, len1, ptr2, len2, ptr3, len3);
    const mem = getUint32Memory();
    const rustptr = mem[retptr / 4];
    const rustlen = mem[retptr / 4 + 1];
    if (rustptr === 0) return;
    const realRet = getArrayU8FromWasm(rustptr, rustlen).slice();
    wasm.__wbindgen_free(rustptr, rustlen * 1);
    return realRet;

}

function getArrayU32FromWasm(ptr, len) {
    return getUint32Memory().subarray(ptr / 4, ptr / 4 + len);
}

const stack = [];

function getObject(idx) {
    if ((idx & 1) === 1) {
        return stack[idx >> 1];
    } else {
        const val = slab[idx >> 1];

        return val.obj;

    }
}

function dropRef(idx) {

    idx = idx >> 1;
    if (idx < 4) return;
    let obj = slab[idx];

    obj.cnt -= 1;
    if (obj.cnt > 0) return;

    // If we hit 0 then free up our space in the slab
    slab[idx] = slab_next;
    slab_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropRef(idx);
    return ret;
}

function freeWrapper(ptr) {

    wasm.__wbg_wrapper_free(ptr);
}
/**
*/
export class Wrapper {

    free() {
        const ptr = this.ptr;
        this.ptr = 0;
        freeWrapper(ptr);
    }

    /**
    * @param {Uint8Array} arg0
    * @returns {}
    */
    constructor(arg0) {
        const [ptr0, len0] = passArray8ToWasm(arg0);
        this.ptr = wasm.wrapper_new(ptr0, len0);
    }
    /**
    * @returns {Uint32Array}
    */
    id() {
        const retptr = globalArgumentPtr();
        wasm.wrapper_id(retptr, this.ptr);
        const mem = getUint32Memory();
        const rustptr = mem[retptr / 4];
        const rustlen = mem[retptr / 4 + 1];
        if (rustptr === 0) return;
        const realRet = getArrayU32FromWasm(rustptr, rustlen).slice();
        wasm.__wbindgen_free(rustptr, rustlen * 4);
        return realRet;

    }
    /**
    * @returns {any}
    */
    action() {
        return takeObject(wasm.wrapper_action(this.ptr));
    }
}

export function __wbindgen_object_drop_ref(i) {
    dropRef(i);
}

const lTextDecoder = typeof TextDecoder === 'undefined' ? require('util').TextDecoder : TextDecoder;

let cachedTextDecoder = new lTextDecoder('utf-8');

function getStringFromWasm(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory().subarray(ptr, ptr + len));
}

export function __wbindgen_string_new(p, l) {
    return addHeapObject(getStringFromWasm(p, l));
}

export function __wbindgen_json_parse(ptr, len) {
    return addHeapObject(JSON.parse(getStringFromWasm(ptr, len)));
}

export function __wbindgen_json_serialize(idx, ptrptr) {
    const [ptr, len] = passStringToWasm(JSON.stringify(getObject(idx)));
    getUint32Memory()[ptrptr / 4] = ptr;
    return len;
}

export function __wbindgen_throw(ptr, len) {
    throw new Error(getStringFromWasm(ptr, len));
}


import { guid } from './uuid.js'
//const _proto = import('./hex_server_protocol');
const _proto = import(/* webpackChunkName: "hex_server_protocol" */ './hex_server_protocol');
//import _proto from './hex_server_protocol.js';

// Since webpack will change the name and potentially the path of the
// `.wasm` file, we have to provide a `locateFile()` hook to redirect
// to the appropriate URL.
// More details: https://kripken.github.io/emscripten-site/docs/api_reference/module.html
/*const module = fibonacci({
  locateFile(path) {
    if(path.endsWith('.wasm')) {
      return fibonacciModule;
    }
    return path;
  }
});*/

const CALLS = {
    Search: ["query"],
    GetTrack: ["key"],
    StreamNext: ["key"],
    StreamEnd: [],
    StreamSeek: ["sample"],
    UpdateTrack: ["key", "title", "album", "interpret", "people", "composer"],
    GetSuggestion: ["key"],
    AddPlaylist: ["name"],
    DeletePlaylist: ["key"],
    SetPlaylistImage: ["key"],
    AddToPlaylist: ["key", "playlist"],
    DeleteFromPlaylist: ["key", "playlist"],
    UpdatePlaylist: ["key", "title", "desc"],
    GetPlaylists: [],
    GetPlaylist: ["key"],
    GetPlaylistsOfTrack: ["key"],
    DeleteTrack: ["key"],
    UploadYoutube: ["path"],
    UploadTrack: ["name", "format", "data"],
    VoteForTrack: ["key"],
    AskUploadProgress: [],
    GetToken: ["GetToken"],
    UpdateToken: ["token", "key", "played", "pos"],
    CreateToken: [],
    LastToken: [],
    GetSummarise: [],
    GetEvents: [],
    Download: ["format", "tracks"],
    AskDownloadProgress: []
}

let proto = null;

class Protocol {
    constructor() {
        let self = this;
        this.buffered_requests = [];
        this.pending_requests = {};

        // create function calls to the protocol
        for(const call in CALLS) {
            // convert CamelCase to underscore_case for function calls
            const under = call.split(/(?=[A-Z])/).join('_').toLowerCase();
            if(CALLS[call].length == 0)
                this[under] = new Function("", "return this.request('" + call + "', null);");
            else
                this[under] = new Function(CALLS[call].join(", "), "return this.request('" + call + "', {" + CALLS[call].join(",") + "});");
        }

        _proto.then(x => {
            alert("LOADED");
            proto = x;
            self.try_connect('ws://' + location.hostname + ':2794');
        });

        return this;
    }

    try_connect(addr) {
        this.socket = new WebSocket('ws://' + location.hostname + ':2794', 'rust-websocket');
        this.socket.binaryType = 'arraybuffer';

        this.socket.onerror = function(err) {
            console.error("Websocket error occured: " + err);
        }

        let self = this;
        this.socket.onclose = function() {
            console.error("Connection to " + this.url + " closed!");

            setTimeout(_ => self.try_connect(addr), 500);
        }

        this.socket.onmessage = this.message.bind(this);

        this.socket.onopen = function() {
            const buffered = self.buffered_requests.splice(0, self.buffered_requests.length);

            for(const idx in buffered) {
                const [id, req] = buffered[idx];

                const buf = proto.request_to_buf(id, req);
                self.socket.send(buf.buffer);
            }
        }
    }

    message(msg) {
        let answ = new proto.Wrapper(new Uint8Array(msg.data));
        
        if(!answ)
            console.error("Could not parse answer!");
        
        const id = answ.id();
        if(!id) {
            console.error(msg);
            console.error("Could not parse answer!");
            return;
        }

        if(this.pending_requests[id] == null) {
            console.error("Got answer without request!");
            return;
        }
        
        const [type, resolve, reject] = this.pending_requests[id];
        let action = answ.action();
            
        if(typeof action === "string" && action != type) {
            reject(action);
            return;
        }
        
        //console.log("Answer");
        //console.log(action);

        const key = Object.keys(action)[0];
        action = action[key];
        
        /*const pack_type = Object.keys(action)[0];
        console.log(pack_type);
        if(type != pack_type) {
            reject("Got packet with invalid type!");
            return;
        }*/
        
        answ = null;
        msg = null;
        delete this.pending_requests[id];

        resolve(action);
    }

    dice_id() {
        return Array.from({length: 4}, () => Math.floor(Math.random() * (2 ** 32)));
    }

    request(type, param, id) {
        if(id == null)
            id = this.dice_id();

        let req = {};
        req[type] = param;

        const promise = new Promise((resolve, reject) => this.pending_requests[id] = [type, resolve, reject]);

        //console.log("Request " + type);
        //console.log(req);
        if(!proto || this.socket.readyState != WebSocket.OPEN) {
            this.buffered_requests.push([id, req]);
        
            return promise;
        }

        const buf = proto.request_to_buf(id, req);
        
        if(!buf) {
            console.error("Could not serialize packet: " + JSON.stringify(req));
            return Promise.reject("could not serialize");
        }

        this.socket.send(buf.buffer);

        return promise;
    }

    start_search(query) {
        const id = this.dice_id();

        let self = this;
        return function() {
            return self.request('Search', {'query': query}, id);
        };
    }

    start_stream(key) {
        const id = this.dice_id();

        let self = this;
        let first = true;
        return [
            function() {
                if(first) {
                    first = false;
                    return self.request("StreamNext", {"key": key}, id);
                } else 
                    return self.request("StreamNext", {"key": null}, id);
            },
            function(sample) {
                return self.request("StreamSeek", {"sample": sample}, id);
            },
            function() {
                return self.request("StreamEnd", null, id);
            }
        ];
    }

    get_suggestions(keys) {
        var promises = [];
        for(const key of keys) {
            promises.push(this.GetSuggestion(key));
        }

        return Promise.all(promises);
    }

    upload_tracks(tracks) {
        let promises = [];
        for(const track of tracks) {
            promises.push(this.UploadTracks(file[0], file[1], file[2]));
        }

        return Promise.all(promises);
    }
}

export default new Protocol();
